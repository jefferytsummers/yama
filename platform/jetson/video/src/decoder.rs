//! NVDEC decoder implementation for NVIDIA Jetson.
//!
//! This module implements the platform `VideoDecoder` trait using
//! GStreamer with NVDEC hardware acceleration.

use std::collections::VecDeque;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use gstreamer as gst;
use gstreamer::prelude::*;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use yama_platform_traits::{PixelFormat, PlatformError, PlatformResult, Size};
use yama_platform_traits::decoder::{
    CodecConfig, CodecType, DecodedFrame, DecodePacket, DecoderCapabilities,
    DecoderStatistics, FrameData, VideoDecoder,
};

/// NVDEC decoder using GStreamer.
pub struct NvdecDecoder {
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

impl NvdecDecoder {
    /// Create a new NVDEC decoder.
    pub fn new() -> Result<Self> {
        gst::init().context("Failed to initialize GStreamer")?;

        info!("Creating NVDEC decoder for Jetson");

        // Check for Jetson-specific capabilities
        let is_jetson = Self::detect_jetson();

        let capabilities = DecoderCapabilities {
            name: if is_jetson {
                "NVDEC (Jetson)".to_string()
            } else {
                "NVDEC (Desktop)".to_string()
            },
            is_hardware: true,
            supported_codecs: vec![
                CodecType::H264,
                CodecType::H265,
                CodecType::Vp9,
                CodecType::Av1, // Orin supports AV1
            ],
            max_resolution: Size::new(7680, 4320), // 8K
            output_formats: vec![
                PixelFormat::Nv12,
                PixelFormat::I420,
                PixelFormat::Bgra8,
            ],
            supports_zero_copy: true, // DMA-BUF support
            max_sessions: if is_jetson { 16 } else { 8 },
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

    /// Detect if running on Jetson.
    fn detect_jetson() -> bool {
        #[cfg(target_os = "linux")]
        {
            use std::path::Path;
            Path::new("/etc/nv_tegra_release").exists()
                || Path::new("/proc/device-tree/compatible")
                    .read_to_string()
                    .map(|s| s.contains("nvidia,tegra"))
                    .unwrap_or(false)
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    /// Build the GStreamer pipeline string for the codec.
    fn build_pipeline_string(config: &CodecConfig) -> String {
        // Use nvv4l2decoder on Jetson, nvdec on desktop
        let decoder_element = if Self::detect_jetson() {
            match config.codec {
                CodecType::H264 => "nvv4l2decoder",
                CodecType::H265 => "nvv4l2decoder",
                CodecType::Vp9 => "nvv4l2decoder",
                CodecType::Av1 => "nvv4l2decoder",
                _ => "avdec_h264",
            }
        } else {
            match config.codec {
                CodecType::H264 => "nvh264dec",
                CodecType::H265 => "nvh265dec",
                CodecType::Vp9 => "nvvp9dec",
                CodecType::Av1 => "nvav1dec",
                _ => "avdec_h264",
            }
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

        // On Jetson, prefer nvvidconv for efficient color conversion
        let converter = if Self::detect_jetson() {
            "nvvidconv"
        } else {
            "videoconvert"
        };

        format!(
            "appsrc name=src ! \
             {} ! \
             {} ! \
             video/x-raw,format={} ! \
             appsink name=sink emit-signals=true sync=false",
            decoder_element, converter, output_format
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

                                // Check for DMA-BUF
                                #[cfg(unix)]
                                let has_dma_buf = buffer
                                    .meta::<gstreamer::meta::ParentBufferMeta>()
                                    .is_some();
                                #[cfg(not(unix))]
                                let has_dma_buf = false;

                                let map = buffer.map_readable().expect("Failed to map buffer");

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

                                if let Ok(mut queue) = frame_queue.try_lock() {
                                    if queue.len() >= 16 {
                                        queue.pop_front();
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

impl Default for NvdecDecoder {
    fn default() -> Self {
        Self::new().expect("Failed to create NVDEC decoder")
    }
}

#[async_trait]
impl VideoDecoder for NvdecDecoder {
    fn capabilities(&self) -> &DecoderCapabilities {
        &self.capabilities
    }

    fn configure(&mut self, config: CodecConfig) -> PlatformResult<()> {
        if !self.capabilities.supported_codecs.contains(&config.codec) {
            return Err(PlatformError::CodecNotSupported(format!(
                "{:?} not supported by NVDEC",
                config.codec
            )));
        }

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
            "Configuring NVDEC decoder: {:?} {}x{}",
            config.codec, config.width, config.height
        );

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

        let src = pipeline
            .by_name("src")
            .ok_or_else(|| PlatformError::InitializationFailed("No appsrc".to_string()))?
            .downcast::<gstreamer_app::AppSrc>()
            .map_err(|_| PlatformError::InitializationFailed("Not an AppSrc".to_string()))?;

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

        src.push_buffer(buffer)
            .map_err(|e| PlatformError::DecodeError(format!("Failed to push buffer: {:?}", e)))?;

        self.stats.frames_decoded += 1;

        let mut frames = Vec::new();
        let mut queue = self.frame_queue.lock().await;
        while let Some(frame) = queue.pop_front() {
            frames.push(frame);
        }

        Ok(frames)
    }

    async fn flush(&mut self) -> PlatformResult<Vec<DecodedFrame>> {
        if let Some(pipeline) = &self.pipeline {
            let src = pipeline
                .by_name("src")
                .ok_or_else(|| PlatformError::InitializationFailed("No appsrc".to_string()))?
                .downcast::<gstreamer_app::AppSrc>()
                .map_err(|_| PlatformError::InitializationFailed("Not an AppSrc".to_string()))?;

            src.end_of_stream()
                .map_err(|e| PlatformError::DecodeError(format!("EOS failed: {:?}", e)))?;

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

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

        if let Ok(mut queue) = self.frame_queue.try_lock() {
            queue.clear();
        }

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

impl Drop for NvdecDecoder {
    fn drop(&mut self) {
        if let Some(pipeline) = &self.pipeline {
            let _ = pipeline.set_state(gst::State::Null);
        }
    }
}

/// Create the platform-appropriate video decoder.
pub fn create_decoder() -> Result<Box<dyn VideoDecoder>> {
    Ok(Box::new(NvdecDecoder::new()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_capabilities() {
        let decoder = NvdecDecoder::new().unwrap();
        let caps = decoder.capabilities();

        assert!(caps.is_hardware);
        assert!(caps.supported_codecs.contains(&CodecType::H264));
        assert!(caps.supports_zero_copy);
    }
}
