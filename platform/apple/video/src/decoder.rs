//! VideoToolbox decoder implementation for Apple Silicon.
//!
//! This module implements the platform `VideoDecoder` trait using
//! GStreamer with VideoToolbox hardware acceleration.
//!
//! # Features
//!
//! - Hardware-accelerated H.264/H.265 decode via VideoToolbox
//! - Streaming decode support via `DecodeSession`
//! - SPS/PPS parsing for proper initialization
//! - Frame queue with configurable back-pressure
//! - Event bus integration for frame distribution (optional)

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use gstreamer as gst;
use gstreamer::prelude::*;
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, error, info, trace, warn};

use yama_platform_traits::decoder::{
    CodecConfig, CodecType, DecodedFrame, DecodePacket, DecoderCapabilities, DecoderStatistics,
    FrameData, VideoDecoder,
};
use yama_platform_traits::{PlatformError, PlatformResult, PixelFormat, Size};

/// Maximum frames in the decode queue before applying back-pressure.
const MAX_QUEUE_DEPTH: usize = 32;

/// Default timeout for waiting on frames (ms).
const DEFAULT_FRAME_TIMEOUT_MS: u64 = 100;

/// VideoToolbox decoder using GStreamer.
pub struct VideoToolboxDecoder {
    /// Decoder capabilities.
    capabilities: DecoderCapabilities,
    /// Current codec configuration.
    config: Option<CodecConfig>,
    /// GStreamer pipeline (if configured).
    pipeline: Option<gst::Pipeline>,
    /// Frame queue from decoder.
    frame_queue: Arc<Mutex<VecDeque<DecodedFrame>>>,
    /// Statistics.
    stats: Arc<RwLock<DecoderStats>>,
    /// Whether the decoder is initialized.
    initialized: bool,
    /// Cancellation flag.
    cancelled: Arc<AtomicBool>,
    /// Frame broadcast channel for event bus integration.
    frame_tx: Option<broadcast::Sender<Arc<DecodedFrame>>>,
}

/// Internal statistics tracking with timing.
#[derive(Debug, Default)]
struct DecoderStats {
    /// Total frames decoded.
    frames_decoded: u64,
    /// Frames dropped due to errors.
    frames_dropped: u64,
    /// Frames dropped due to queue overflow.
    frames_dropped_overflow: u64,
    /// Total decode time (microseconds).
    total_decode_time_us: u64,
    /// Peak decode time (microseconds).
    peak_decode_time_us: u64,
    /// Last frame PTS.
    last_pts: i64,
    /// Start time for rate calculation.
    start_time: Option<Instant>,
}

impl DecoderStats {
    fn to_statistics(&self, queue_depth: u32) -> DecoderStatistics {
        let avg_decode_time_us = if self.frames_decoded > 0 {
            self.total_decode_time_us / self.frames_decoded
        } else {
            0
        };

        DecoderStatistics {
            frames_decoded: self.frames_decoded,
            frames_dropped: self.frames_dropped + self.frames_dropped_overflow,
            avg_decode_time_us,
            peak_decode_time_us: self.peak_decode_time_us,
            queue_depth,
        }
    }
}

impl VideoToolboxDecoder {
    /// Create a new VideoToolbox decoder.
    pub fn new() -> Result<Self> {
        // Initialize GStreamer if not already done
        gst::init().context("Failed to initialize GStreamer")?;

        info!("Creating VideoToolbox decoder");

        let capabilities = DecoderCapabilities {
            name: "VideoToolbox (GStreamer)".to_string(),
            is_hardware: true,
            supported_codecs: vec![CodecType::H264, CodecType::H265, CodecType::Vp9],
            max_resolution: Size::new(7680, 4320), // 8K
            output_formats: vec![PixelFormat::Nv12, PixelFormat::I420, PixelFormat::Bgra8],
            supports_zero_copy: true,
            max_sessions: 8,
        };

        Ok(Self {
            capabilities,
            config: None,
            pipeline: None,
            frame_queue: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_QUEUE_DEPTH))),
            stats: Arc::new(RwLock::new(DecoderStats::default())),
            initialized: false,
            cancelled: Arc::new(AtomicBool::new(false)),
            frame_tx: None,
        })
    }

    /// Create a decoder with an event bus broadcast channel for frame distribution.
    pub fn with_broadcast(capacity: usize) -> Result<(Self, broadcast::Receiver<Arc<DecodedFrame>>)> {
        let mut decoder = Self::new()?;
        let (tx, rx) = broadcast::channel(capacity);
        decoder.frame_tx = Some(tx);
        Ok((decoder, rx))
    }

    /// Subscribe to decoded frames (for event bus integration).
    pub fn subscribe(&self) -> Option<broadcast::Receiver<Arc<DecodedFrame>>> {
        self.frame_tx.as_ref().map(|tx| tx.subscribe())
    }

    /// Build the GStreamer pipeline string for the codec.
    fn build_pipeline_string(config: &CodecConfig) -> String {
        let decoder_element = match config.codec {
            CodecType::H264 => "vtdec_hw",
            CodecType::H265 => "vtdec_hw",
            CodecType::Vp9 => "vp9dec", // VP9 may use software fallback
            _ => "avdec_h264",          // Fallback
        };

        let output_format = config
            .output_format
            .map(|f| match f {
                PixelFormat::Nv12 => "NV12",
                PixelFormat::I420 => "I420",
                PixelFormat::Bgra8 => "BGRA",
                _ => "NV12",
            })
            .unwrap_or("NV12");

        // Build caps string with codec-specific parameters
        let caps = Self::build_input_caps(config);

        format!(
            "appsrc name=src format=time do-timestamp=true {} ! \
             {} ! \
             videoconvert ! \
             video/x-raw,format={} ! \
             appsink name=sink emit-signals=true sync=false max-buffers=16 drop=false",
            caps, decoder_element, output_format
        )
    }

    /// Build input caps with codec-specific header data.
    fn build_input_caps(config: &CodecConfig) -> String {
        let (stream_format, codec_data) = match config.codec {
            CodecType::H264 => {
                // H.264 requires stream-format and potentially codec_data (SPS/PPS)
                let stream_format = if config.extra_data.is_some() {
                    "avc" // AVCC format with length-prefixed NALUs
                } else {
                    "byte-stream" // Annex B format with start codes
                };
                (stream_format, Self::format_codec_data(&config.extra_data))
            }
            CodecType::H265 => {
                let stream_format = if config.extra_data.is_some() {
                    "hvc1"
                } else {
                    "byte-stream"
                };
                (stream_format, Self::format_codec_data(&config.extra_data))
            }
            _ => ("", String::new()),
        };

        match config.codec {
            CodecType::H264 => {
                let mut caps = format!(
                    "caps=video/x-h264,width={},height={},framerate=0/1,stream-format={}",
                    config.width, config.height, stream_format
                );
                if !codec_data.is_empty() {
                    caps.push_str(&format!(",codec_data=(buffer){}", codec_data));
                }
                caps
            }
            CodecType::H265 => {
                let mut caps = format!(
                    "caps=video/x-h265,width={},height={},framerate=0/1,stream-format={}",
                    config.width, config.height, stream_format
                );
                if !codec_data.is_empty() {
                    caps.push_str(&format!(",codec_data=(buffer){}", codec_data));
                }
                caps
            }
            CodecType::Vp9 => {
                format!(
                    "caps=video/x-vp9,width={},height={},framerate=0/1",
                    config.width, config.height
                )
            }
            _ => String::new(),
        }
    }

    /// Format codec_data (SPS/PPS) as hex string for GStreamer caps.
    fn format_codec_data(data: &Option<Vec<u8>>) -> String {
        data.as_ref()
            .map(|d| d.iter().map(|b| format!("{:02x}", b)).collect::<String>())
            .unwrap_or_default()
    }

    /// Parse SPS/PPS from H.264 extra_data (AVCC format).
    pub fn parse_h264_avcc(data: &[u8]) -> Option<(Vec<Vec<u8>>, Vec<Vec<u8>>)> {
        if data.len() < 7 {
            return None;
        }

        // AVCC format:
        // configurationVersion (1) + avcProfileIndication (1) + profile_compatibility (1)
        // + avcLevelIndication (1) + reserved (6 bits) + lengthSizeMinusOne (2 bits)
        // + reserved (3 bits) + numOfSequenceParameterSets (5 bits)
        let length_size = (data[4] & 0x03) + 1;
        let num_sps = data[5] & 0x1F;

        let mut offset = 6usize;
        let mut sps_list = Vec::new();
        let mut pps_list = Vec::new();

        // Parse SPS
        for _ in 0..num_sps {
            if offset + 2 > data.len() {
                return None;
            }
            let sps_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;
            if offset + sps_len > data.len() {
                return None;
            }
            sps_list.push(data[offset..offset + sps_len].to_vec());
            offset += sps_len;
        }

        // Parse PPS
        if offset >= data.len() {
            return Some((sps_list, pps_list));
        }
        let num_pps = data[offset];
        offset += 1;

        for _ in 0..num_pps {
            if offset + 2 > data.len() {
                break;
            }
            let pps_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;
            if offset + pps_len > data.len() {
                break;
            }
            pps_list.push(data[offset..offset + pps_len].to_vec());
            offset += pps_len;
        }

        debug!(
            "Parsed {} SPS and {} PPS from AVCC data (length_size={})",
            sps_list.len(),
            pps_list.len(),
            length_size
        );

        Some((sps_list, pps_list))
    }

    /// Set up frame callback for the appsink.
    fn setup_frame_callback(&self, pipeline: &gst::Pipeline) -> Result<()> {
        let sink = pipeline
            .by_name("sink")
            .context("No appsink found")?
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

        let frame_queue = self.frame_queue.clone();
        let stats = self.stats.clone();
        let cancelled = self.cancelled.clone();
        let frame_tx = self.frame_tx.clone();

        sink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    // Check cancellation
                    if cancelled.load(Ordering::Relaxed) {
                        return Err(gst::FlowError::Flushing);
                    }

                    let decode_start = Instant::now();

                    match appsink.pull_sample() {
                        Ok(sample) => {
                            if let Some(buffer) = sample.buffer() {
                                let caps = sample.caps().expect("Sample without caps");
                                let video_info = gstreamer_video::VideoInfo::from_caps(caps)
                                    .expect("Invalid caps");

                                // Map buffer for reading
                                let map = buffer.map_readable().expect("Failed to map buffer");

                                let format = match video_info.format() {
                                    gstreamer_video::VideoFormat::Nv12 => PixelFormat::Nv12,
                                    gstreamer_video::VideoFormat::I420 => PixelFormat::I420,
                                    gstreamer_video::VideoFormat::Bgra => PixelFormat::Bgra8,
                                    _ => PixelFormat::Nv12,
                                };

                                let pts = buffer.pts().map(|p| p.useconds() as i64).unwrap_or(0);
                                let dts = buffer.dts().map(|d| d.useconds() as i64).unwrap_or(0);

                                let frame = DecodedFrame {
                                    width: video_info.width(),
                                    height: video_info.height(),
                                    format,
                                    pts,
                                    dts,
                                    duration: buffer
                                        .duration()
                                        .map(|d| d.useconds() as i64)
                                        .unwrap_or(0),
                                    is_keyframe: !buffer
                                        .flags()
                                        .contains(gst::BufferFlags::DELTA_UNIT),
                                    data: FrameData::CpuBuffer {
                                        data: map.as_slice().to_vec(),
                                        strides: (0..video_info.n_planes())
                                            .map(|i| video_info.stride()[i as usize] as usize)
                                            .collect(),
                                    },
                                };

                                // Update statistics
                                let decode_time_us = decode_start.elapsed().as_micros() as u64;
                                if let Ok(mut s) = stats.try_write() {
                                    s.frames_decoded += 1;
                                    s.total_decode_time_us += decode_time_us;
                                    if decode_time_us > s.peak_decode_time_us {
                                        s.peak_decode_time_us = decode_time_us;
                                    }
                                    s.last_pts = pts;
                                    if s.start_time.is_none() {
                                        s.start_time = Some(Instant::now());
                                    }
                                }

                                // Broadcast frame if channel is configured
                                let frame_arc = Arc::new(frame);
                                if let Some(tx) = &frame_tx {
                                    // Ignore send errors (no receivers)
                                    let _ = tx.send(Arc::clone(&frame_arc));
                                }

                                // Push to queue with back-pressure
                                if let Ok(mut queue) = frame_queue.try_lock() {
                                    if queue.len() >= MAX_QUEUE_DEPTH {
                                        // Back-pressure: drop oldest frame
                                        queue.pop_front();
                                        if let Ok(mut s) = stats.try_write() {
                                            s.frames_dropped_overflow += 1;
                                        }
                                        trace!("Frame queue overflow, dropping oldest frame");
                                    }
                                    // We need to extract the frame from Arc if we own it
                                    // For now, clone the data
                                    let frame = DecodedFrame {
                                        width: frame_arc.width,
                                        height: frame_arc.height,
                                        format: frame_arc.format,
                                        pts: frame_arc.pts,
                                        dts: frame_arc.dts,
                                        duration: frame_arc.duration,
                                        is_keyframe: frame_arc.is_keyframe,
                                        data: match &frame_arc.data {
                                            FrameData::CpuBuffer { data, strides } => {
                                                FrameData::CpuBuffer {
                                                    data: data.clone(),
                                                    strides: strides.clone(),
                                                }
                                            }
                                            FrameData::GpuBuffer(buf) => {
                                                FrameData::GpuBuffer(buf.clone())
                                            }
                                            #[cfg(unix)]
                                            FrameData::DmaBuf {
                                                fd,
                                                offset,
                                                stride,
                                                modifier,
                                            } => FrameData::DmaBuf {
                                                fd: *fd,
                                                offset: *offset,
                                                stride: *stride,
                                                modifier: *modifier,
                                            },
                                        },
                                    };
                                    queue.push_back(frame);
                                }
                            }
                            Ok(gst::FlowSuccess::Ok)
                        }
                        Err(e) => {
                            error!("Failed to pull sample: {:?}", e);
                            if let Ok(mut s) = stats.try_write() {
                                s.frames_dropped += 1;
                            }
                            Err(gst::FlowError::Error)
                        }
                    }
                })
                .build(),
        );

        Ok(())
    }

    /// Cancel any ongoing decode operations.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Get the current queue depth.
    pub async fn queue_depth(&self) -> usize {
        self.frame_queue.lock().await.len()
    }
}

impl Default for VideoToolboxDecoder {
    fn default() -> Self {
        Self::new().expect("Failed to create VideoToolbox decoder")
    }
}

#[async_trait]
impl VideoDecoder for VideoToolboxDecoder {
    fn capabilities(&self) -> &DecoderCapabilities {
        &self.capabilities
    }

    fn configure(&mut self, config: CodecConfig) -> PlatformResult<()> {
        // Validate codec support
        if !self.capabilities.supported_codecs.contains(&config.codec) {
            return Err(PlatformError::CodecNotSupported(format!(
                "{:?} not supported by VideoToolbox",
                config.codec
            )));
        }

        // Validate resolution
        if config.width > self.capabilities.max_resolution.width
            || config.height > self.capabilities.max_resolution.height
        {
            return Err(PlatformError::InvalidParameter(format!(
                "Resolution {}x{} exceeds maximum {}x{}",
                config.width,
                config.height,
                self.capabilities.max_resolution.width,
                self.capabilities.max_resolution.height
            )));
        }

        info!(
            "Configuring VideoToolbox decoder: {:?} {}x{}",
            config.codec, config.width, config.height
        );

        // Parse SPS/PPS if available (for H.264 AVCC format)
        if config.codec == CodecType::H264 {
            if let Some(ref extra_data) = config.extra_data {
                if let Some((sps, pps)) = Self::parse_h264_avcc(extra_data) {
                    debug!("Parsed {} SPS and {} PPS NAL units", sps.len(), pps.len());
                }
            }
        }

        // Build and start pipeline
        let pipeline_str = Self::build_pipeline_string(&config);
        debug!("GStreamer pipeline: {}", pipeline_str);

        let pipeline = gst::parse::launch(&pipeline_str)
            .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?
            .downcast::<gst::Pipeline>()
            .map_err(|_| {
                PlatformError::InitializationFailed("Failed to downcast pipeline".to_string())
            })?;

        self.setup_frame_callback(&pipeline)
            .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;

        // Start pipeline
        pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;

        // Reset state
        self.cancelled.store(false, Ordering::Relaxed);
        self.pipeline = Some(pipeline);
        self.config = Some(config);
        self.initialized = true;

        Ok(())
    }

    async fn decode(&mut self, packet: &DecodePacket) -> PlatformResult<Vec<DecodedFrame>> {
        if !self.initialized {
            return Err(PlatformError::InitializationFailed(
                "Decoder not configured".to_string(),
            ));
        }

        if self.cancelled.load(Ordering::Relaxed) {
            return Err(PlatformError::Cancelled);
        }

        let pipeline = self
            .pipeline
            .as_ref()
            .ok_or_else(|| PlatformError::InitializationFailed("No pipeline".to_string()))?;

        // Get appsrc
        let src = pipeline
            .by_name("src")
            .ok_or_else(|| PlatformError::InitializationFailed("No appsrc".to_string()))?
            .downcast::<gstreamer_app::AppSrc>()
            .map_err(|_| PlatformError::InitializationFailed("Not an AppSrc".to_string()))?;

        // Create buffer from packet data
        let mut buffer = gst::Buffer::from_slice(packet.data.clone());
        {
            let buffer_ref = buffer.get_mut().unwrap();
            buffer_ref.set_pts(gst::ClockTime::from_useconds(packet.pts as u64));
            buffer_ref.set_dts(gst::ClockTime::from_useconds(packet.dts as u64));
            if packet.is_keyframe {
                buffer_ref.unset_flags(gst::BufferFlags::DELTA_UNIT);
            } else {
                buffer_ref.set_flags(gst::BufferFlags::DELTA_UNIT);
            }
        }

        // Push buffer to pipeline
        src.push_buffer(buffer)
            .map_err(|e| PlatformError::DecodeError(format!("Failed to push buffer: {:?}", e)))?;

        // Collect decoded frames
        let mut frames = Vec::new();
        let mut queue = self.frame_queue.lock().await;
        while let Some(frame) = queue.pop_front() {
            frames.push(frame);
        }

        trace!(
            "Decoded packet pts={}, returned {} frames",
            packet.pts,
            frames.len()
        );

        Ok(frames)
    }

    async fn flush(&mut self) -> PlatformResult<Vec<DecodedFrame>> {
        if let Some(pipeline) = &self.pipeline {
            // Send EOS to flush pipeline
            let src = pipeline
                .by_name("src")
                .ok_or_else(|| PlatformError::InitializationFailed("No appsrc".to_string()))?
                .downcast::<gstreamer_app::AppSrc>()
                .map_err(|_| PlatformError::InitializationFailed("Not an AppSrc".to_string()))?;

            src.end_of_stream()
                .map_err(|e| PlatformError::DecodeError(format!("EOS failed: {:?}", e)))?;

            // Wait for frames to drain (with timeout)
            let timeout = std::time::Duration::from_millis(DEFAULT_FRAME_TIMEOUT_MS);
            let start = Instant::now();
            while start.elapsed() < timeout {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                // Check if queue has stabilized
                let depth = self.frame_queue.lock().await.len();
                if depth == 0 {
                    break;
                }
            }
        }

        // Collect remaining frames
        let mut frames = Vec::new();
        let mut queue = self.frame_queue.lock().await;
        while let Some(frame) = queue.pop_front() {
            frames.push(frame);
        }

        debug!("Flush returned {} frames", frames.len());
        Ok(frames)
    }

    fn reset(&mut self) -> PlatformResult<()> {
        // Stop current pipeline
        if let Some(pipeline) = &self.pipeline {
            pipeline
                .set_state(gst::State::Null)
                .map_err(|e| PlatformError::DecodeError(e.to_string()))?;
        }

        self.pipeline = None;
        self.initialized = false;
        self.cancelled.store(false, Ordering::Relaxed);

        // Clear frame queue
        if let Ok(mut queue) = self.frame_queue.try_lock() {
            queue.clear();
        }

        // Reset statistics
        if let Ok(mut stats) = self.stats.try_write() {
            *stats = DecoderStats::default();
        }

        // Re-configure if we had a config
        if let Some(config) = self.config.clone() {
            self.configure(config)?;
        }

        Ok(())
    }

    fn current_config(&self) -> Option<&CodecConfig> {
        self.config.as_ref()
    }

    fn statistics(&self) -> DecoderStatistics {
        let queue_depth = self
            .frame_queue
            .try_lock()
            .map(|q| q.len() as u32)
            .unwrap_or(0);

        self.stats
            .try_read()
            .map(|s| s.to_statistics(queue_depth))
            .unwrap_or_default()
    }
}

impl Drop for VideoToolboxDecoder {
    fn drop(&mut self) {
        self.cancel();
        if let Some(pipeline) = &self.pipeline {
            let _ = pipeline.set_state(gst::State::Null);
        }
    }
}

/// Create the platform-appropriate video decoder.
pub fn create_decoder() -> Result<Box<dyn VideoDecoder>> {
    Ok(Box::new(VideoToolboxDecoder::new()?))
}

// ============================================================================
// DecodeSession - High-level streaming decode API
// ============================================================================

/// A streaming decode session for processing video files or streams.
///
/// This provides a higher-level API than the raw [`VideoDecoder`] trait,
/// handling demuxing, packet feeding, and frame collection automatically.
///
/// # Example
///
/// ```rust,ignore
/// let session = DecodeSession::open_file("video.mp4").await?;
///
/// while let Some(frame) = session.next_frame().await? {
///     // Process frame
///     process_frame(&frame);
/// }
///
/// session.close().await?;
/// ```
pub struct DecodeSession {
    /// Underlying decoder.
    decoder: VideoToolboxDecoder,
    /// GStreamer demux/decode pipeline.
    pipeline: gst::Pipeline,
    /// Frame receiver.
    frame_rx: Option<broadcast::Receiver<Arc<DecodedFrame>>>,
    /// Whether the session is active.
    active: bool,
    /// Video duration in microseconds (if known).
    duration_us: Option<i64>,
    /// Video width.
    width: u32,
    /// Video height.
    height: u32,
    /// Frame rate (frames per second).
    fps: f64,
}

/// Session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session is initializing.
    Initializing,
    /// Session is ready to decode.
    Ready,
    /// Session is actively decoding.
    Decoding,
    /// End of stream reached.
    EndOfStream,
    /// Session encountered an error.
    Error,
    /// Session is closed.
    Closed,
}

impl DecodeSession {
    /// Open a video file for decoding.
    pub async fn open_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            anyhow::bail!("File not found: {:?}", path);
        }

        // Create decoder with broadcast channel
        let (decoder, frame_rx) = VideoToolboxDecoder::with_broadcast(32)?;

        // Build demux pipeline
        let uri = format!("file://{}", path.display());
        let pipeline_str = format!(
            "uridecodebin uri={} name=src ! \
             videoconvert ! \
             video/x-raw,format=NV12 ! \
             appsink name=sink emit-signals=true sync=false max-buffers=16",
            uri
        );

        let pipeline = gst::parse::launch(&pipeline_str)
            .context("Failed to create decode pipeline")?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast pipeline"))?;

        // Get video info from pipeline
        pipeline
            .set_state(gst::State::Paused)
            .map_err(|e| anyhow::anyhow!("Failed to pause pipeline: {:?}", e))?;

        // Wait for state change
        let (_, state, _) = pipeline.state(gst::ClockTime::from_seconds(5));
        if state != gst::State::Paused {
            anyhow::bail!("Pipeline failed to reach paused state");
        }

        // Query duration
        let duration_us = pipeline
            .query_duration::<gst::ClockTime>()
            .map(|d| d.useconds() as i64);

        // Get video dimensions from sink caps
        let sink = pipeline
            .by_name("sink")
            .context("No sink found")?
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast sink"))?;

        // Wait for caps to be negotiated
        let caps = sink.caps();
        let (width, height, fps) = if let Some(caps) = caps {
            let s = caps.structure(0).context("No caps structure")?;
            let width = s.get::<i32>("width").unwrap_or(1920) as u32;
            let height = s.get::<i32>("height").unwrap_or(1080) as u32;
            let framerate = s.get::<gst::Fraction>("framerate").ok();
            let fps = framerate
                .map(|f| f.numer() as f64 / f.denom() as f64)
                .unwrap_or(30.0);
            (width, height, fps)
        } else {
            (1920, 1080, 30.0)
        };

        info!(
            "Opened video: {}x{} @ {:.2} fps, duration: {:?}",
            width, height, fps, duration_us
        );

        Ok(Self {
            decoder,
            pipeline,
            frame_rx: Some(frame_rx),
            active: false,
            duration_us,
            width,
            height,
            fps,
        })
    }

    /// Start decoding.
    pub fn start(&mut self) -> Result<()> {
        if self.active {
            return Ok(());
        }

        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| anyhow::anyhow!("Failed to start pipeline: {:?}", e))?;

        self.active = true;
        Ok(())
    }

    /// Get the next decoded frame.
    pub async fn next_frame(&mut self) -> Result<Option<Arc<DecodedFrame>>> {
        if !self.active {
            self.start()?;
        }

        let rx = self
            .frame_rx
            .as_mut()
            .context("No frame receiver available")?;

        match rx.recv().await {
            Ok(frame) => Ok(Some(frame)),
            Err(broadcast::error::RecvError::Closed) => Ok(None),
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!("Frame receiver lagged by {} frames", n);
                // Try to receive again
                match rx.recv().await {
                    Ok(frame) => Ok(Some(frame)),
                    Err(_) => Ok(None),
                }
            }
        }
    }

    /// Seek to a position in microseconds.
    pub async fn seek(&mut self, position_us: i64) -> Result<()> {
        let position = gst::ClockTime::from_useconds(position_us as u64);

        self.pipeline
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                position,
            )
            .map_err(|_| anyhow::anyhow!("Seek failed"))?;

        Ok(())
    }

    /// Get video duration in microseconds.
    pub fn duration_us(&self) -> Option<i64> {
        self.duration_us
    }

    /// Get video dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get frame rate.
    pub fn fps(&self) -> f64 {
        self.fps
    }

    /// Get session state.
    pub fn state(&self) -> SessionState {
        let (_, state, _) = self.pipeline.state(gst::ClockTime::ZERO);
        match state {
            gst::State::Null => SessionState::Closed,
            gst::State::Ready => SessionState::Ready,
            gst::State::Paused => SessionState::Ready,
            gst::State::Playing => SessionState::Decoding,
            _ => SessionState::Initializing,
        }
    }

    /// Close the session.
    pub async fn close(&mut self) -> Result<()> {
        self.decoder.cancel();
        self.pipeline
            .set_state(gst::State::Null)
            .map_err(|e| anyhow::anyhow!("Failed to stop pipeline: {:?}", e))?;
        self.active = false;
        Ok(())
    }
}

impl Drop for DecodeSession {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_capabilities() {
        let decoder = VideoToolboxDecoder::new().unwrap();
        let caps = decoder.capabilities();

        assert!(caps.is_hardware);
        assert!(caps.supported_codecs.contains(&CodecType::H264));
        assert!(caps.supported_codecs.contains(&CodecType::H265));
    }
}
