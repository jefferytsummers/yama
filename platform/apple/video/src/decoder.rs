//! VideoToolbox decoder implementation for Apple Silicon.
//!
//! This module implements the platform `VideoDecoder` trait using
//! GStreamer with VideoToolbox hardware acceleration.

use std::collections::VecDeque;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use gstreamer as gst;
use gstreamer::prelude::*;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use yama_platform_traits::{
    PlatformError, PlatformResult, PixelFormat, Size,
};
use yama_platform_traits::decoder::{
    CodecConfig, CodecType, DecodedFrame, DecodePacket, DecoderCapabilities,
    DecoderStatistics, FrameData, VideoDecoder,
};

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
    stats: DecoderStatistics,
    /// Whether the decoder is initialized.
    initialized: bool,
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
            supported_codecs: vec![
                CodecType::H264,
                CodecType::H265,
                CodecType::Vp9,
            ],
            max_resolution: Size::new(7680, 4320), // 8K
            output_formats: vec![
                PixelFormat::Nv12,
                PixelFormat::I420,
                PixelFormat::Bgra8,
            ],
            supports_zero_copy: true,
            max_sessions: 8,
        };

        Ok(Self {
            capabilities,
            config: None,
            pipeline: None,
            frame_queue: Arc::new(Mutex::new(VecDeque::with_capacity(16))),
            stats: DecoderStatistics::default(),
            initialized: false,
        })
    }

    /// Build the GStreamer pipeline string for the codec.
    fn build_pipeline_string(config: &CodecConfig) -> String {
        let decoder_element = match config.codec {
            CodecType::H264 => "vtdec_hw",
            CodecType::H265 => "vtdec_hw",
            CodecType::Vp9 => "vp9dec", // VP9 may use software fallback
            _ => "avdec_h264", // Fallback
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

        format!(
            "appsrc name=src ! \
             {} ! \
             videoconvert ! \
             video/x-raw,format={} ! \
             appsink name=sink emit-signals=true sync=false",
            decoder_element,
            output_format
        )
    }

    /// Set up frame callback for the appsink.
    fn setup_frame_callback(&self, pipeline: &gst::Pipeline) -> Result<()> {
        let sink = pipeline
            .by_name("sink")
            .context("No appsink found")?
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

        let frame_queue = self.frame_queue.clone();

        sink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    match appsink.pull_sample() {
                        Ok(sample) => {
                            if let Some(buffer) = sample.buffer() {
                                let caps = sample.caps().expect("Sample without caps");
                                let video_info = gstreamer_video::VideoInfo::from_caps(caps)
                                    .expect("Invalid caps");

                                // Map buffer for reading
                                let map = buffer
                                    .map_readable()
                                    .expect("Failed to map buffer");

                                let format = match video_info.format() {
                                    gstreamer_video::VideoFormat::Nv12 => PixelFormat::Nv12,
                                    gstreamer_video::VideoFormat::I420 => PixelFormat::I420,
                                    gstreamer_video::VideoFormat::Bgra => PixelFormat::Bgra8,
                                    _ => PixelFormat::Nv12,
                                };

                                let frame = DecodedFrame {
                                    width: video_info.width(),
                                    height: video_info.height(),
                                    format,
                                    pts: buffer
                                        .pts()
                                        .map(|p| p.useconds() as i64)
                                        .unwrap_or(0),
                                    dts: buffer
                                        .dts()
                                        .map(|d| d.useconds() as i64)
                                        .unwrap_or(0),
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

                                // Push to queue
                                if let Ok(mut queue) = frame_queue.try_lock() {
                                    if queue.len() >= 16 {
                                        queue.pop_front(); // Drop oldest frame
                                    }
                                    queue.push_back(frame);
                                }
                            }
                            Ok(gst::FlowSuccess::Ok)
                        }
                        Err(_) => Err(gst::FlowError::Error),
                    }
                })
                .build(),
        );

        Ok(())
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

        self.stats.frames_decoded += 1;

        // Collect decoded frames
        let mut frames = Vec::new();
        let mut queue = self.frame_queue.lock().await;
        while let Some(frame) = queue.pop_front() {
            frames.push(frame);
        }

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

            // Wait briefly for frames to drain
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        // Collect remaining frames
        let mut frames = Vec::new();
        let mut queue = self.frame_queue.lock().await;
        while let Some(frame) = queue.pop_front() {
            frames.push(frame);
        }

        Ok(frames)
    }

    fn reset(&mut self) -> PlatformResult<()> {
        if let Some(pipeline) = &self.pipeline {
            pipeline
                .set_state(gst::State::Null)
                .map_err(|e| PlatformError::DecodeError(e.to_string()))?;
        }

        self.pipeline = None;
        self.initialized = false;

        // Clear frame queue
        if let Ok(mut queue) = self.frame_queue.try_lock() {
            queue.clear();
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
        self.stats.clone()
    }
}

impl Drop for VideoToolboxDecoder {
    fn drop(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            let _ = pipeline.set_state(gst::State::Null);
        }
    }
}

/// Create the platform-appropriate video decoder.
pub fn create_decoder() -> Result<Box<dyn VideoDecoder>> {
    Ok(Box::new(VideoToolboxDecoder::new()?))
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
