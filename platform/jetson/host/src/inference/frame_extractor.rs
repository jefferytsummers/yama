//! Frame Extraction for NVIDIA Jetson
//!
//! Uses GStreamer with NVDEC (nvv4l2decoder) for hardware-accelerated
//! video decoding on Jetson platforms.
//!
//! # Pipeline
//!
//! ```text
//! filesrc ! decodebin ! nvv4l2decoder ! nvvidconv !
//! video/x-raw,format=NV12 ! jpegenc ! appsink
//! ```
//!
//! # Zero-Copy Path
//!
//! For best performance, frames can be kept in GPU memory using DMA-BUF:
//!
//! ```text
//! filesrc ! nvv4l2decoder ! nvvidconv ! video/x-raw(memory:NVMM) ! appsink
//! ```

use std::path::Path;

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_pbutils as gst_pbutils;
use gst_pbutils::prelude::*;
use tracing::{debug, error, info, warn};

/// Extracted frame data.
#[derive(Debug, Clone)]
pub struct ExtractedFrame {
    /// Frame number (0-indexed).
    pub number: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Frame width.
    pub width: u32,
    /// Frame height.
    pub height: u32,
    /// JPEG-encoded image data.
    pub data: Vec<u8>,
}

/// Frame extraction configuration.
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Interval between frames in milliseconds.
    pub interval_ms: u64,
    /// Maximum number of frames to extract.
    pub max_frames: u32,
    /// Target width (None = original).
    pub target_width: Option<u32>,
    /// Target height (None = original).
    pub target_height: Option<u32>,
    /// JPEG quality (1-100).
    pub jpeg_quality: u32,
    /// Use hardware decoder (nvv4l2decoder).
    pub use_hw_decoder: bool,
    /// Use hardware color converter (nvvidconv).
    pub use_hw_converter: bool,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            max_frames: 300,
            target_width: None,
            target_height: None,
            jpeg_quality: 85,
            use_hw_decoder: true,
            use_hw_converter: true,
        }
    }
}

/// Video metadata.
#[derive(Debug, Clone)]
pub struct VideoInfo {
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Video width.
    pub width: u32,
    /// Video height.
    pub height: u32,
    /// Frames per second.
    pub fps: f32,
    /// Video codec.
    pub codec: String,
    /// Estimated frame count based on extraction config.
    pub estimated_frames: u32,
}

/// Frame extractor for video files using GStreamer with NVDEC.
pub struct FrameExtractor {
    config: ExtractionConfig,
}

impl FrameExtractor {
    /// Create a new frame extractor.
    ///
    /// Initializes GStreamer if not already initialized.
    pub fn new(config: ExtractionConfig) -> Self {
        // Initialize GStreamer (safe to call multiple times)
        if let Err(e) = gst::init() {
            warn!("GStreamer init warning (may already be initialized): {}", e);
        }
        Self { config }
    }

    /// Get video metadata without extracting frames.
    pub fn get_video_info(&self, video_path: &Path) -> Result<VideoInfo> {
        debug!("Getting video info for: {:?}", video_path);

        let discoverer = gst_pbutils::Discoverer::new(gst::ClockTime::from_seconds(10))
            .context("Failed to create discoverer")?;

        let uri = format!("file://{}", video_path.display());
        let info = discoverer
            .discover_uri(&uri)
            .context("Failed to discover video")?;

        let duration_ns = info.duration().unwrap_or(gst::ClockTime::ZERO);
        let duration_ms = duration_ns.nseconds() / 1_000_000;

        // Find video stream info
        let mut width = 1920;
        let mut height = 1080;
        let mut fps = 30.0_f32;
        let mut codec = "unknown".to_string();

        for stream in info.video_streams() {
            width = stream.width();
            height = stream.height();
            if let (num, denom) = (stream.framerate().numer(), stream.framerate().denom()) {
                if denom > 0 {
                    fps = num as f32 / denom as f32;
                }
            }
            if let Some(caps) = stream.caps() {
                if let Some(structure) = caps.structure(0) {
                    codec = structure.name().to_string();
                }
            }
            break;
        }

        let estimated_frames = std::cmp::min(
            (duration_ms / self.config.interval_ms) as u32,
            self.config.max_frames,
        );

        Ok(VideoInfo {
            duration_ms,
            width,
            height,
            fps,
            codec,
            estimated_frames,
        })
    }

    /// Extract frames from a video file.
    ///
    /// Uses GStreamer with NVDEC for hardware-accelerated decoding on Jetson.
    pub async fn extract_frames(&self, video_path: &Path) -> Result<Vec<ExtractedFrame>> {
        info!("Extracting frames from: {:?}", video_path);

        // Verify file exists
        if !video_path.exists() {
            anyhow::bail!("Video file not found: {:?}", video_path);
        }

        // Get video info for duration and frame count calculation
        let info = self.get_video_info(video_path)?;
        info!(
            "Video: {}x{}, {:.1} fps, {} ms duration, codec: {}",
            info.width, info.height, info.fps, info.duration_ms, info.codec
        );

        // Calculate target dimensions (preserve aspect ratio)
        let (target_width, target_height) = self.calculate_target_size(info.width, info.height);

        // Build GStreamer pipeline for frame extraction
        let pipeline_str = self.build_extraction_pipeline(video_path, target_width, target_height)?;
        debug!("GStreamer pipeline: {}", pipeline_str);

        let pipeline = gst::parse::launch(&pipeline_str)
            .context("Failed to parse pipeline")?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to Pipeline"))?;

        // Get appsink
        let sink = pipeline
            .by_name("sink")
            .context("No appsink found")?
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

        // Configure appsink for manual pulling
        sink.set_property("emit-signals", false);
        sink.set_property("sync", false);
        sink.set_property("max-buffers", 1u32);
        sink.set_property("drop", true);

        // Start pipeline
        pipeline
            .set_state(gst::State::Playing)
            .context("Failed to start pipeline")?;

        // Extract frames at specified intervals
        let extraction_result = self
            .extract_frames_at_intervals(
                &pipeline,
                &sink,
                self.config.interval_ms,
                self.config.max_frames,
                info.duration_ms,
            )
            .await;

        // Stop pipeline
        pipeline
            .set_state(gst::State::Null)
            .context("Failed to stop pipeline")?;

        let extracted = extraction_result?;
        info!("Extracted {} frames", extracted.len());

        Ok(extracted)
    }

    /// Calculate target dimensions preserving aspect ratio.
    fn calculate_target_size(&self, original_width: u32, original_height: u32) -> (u32, u32) {
        match (self.config.target_width, self.config.target_height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => {
                let h = (original_height as f32 * w as f32 / original_width as f32) as u32;
                (w, h)
            }
            (None, Some(h)) => {
                let w = (original_width as f32 * h as f32 / original_height as f32) as u32;
                (w, h)
            }
            (None, None) => {
                // Default: scale down large videos to max 1280 width
                if original_width > 1280 {
                    let scale = 1280.0 / original_width as f32;
                    let w = 1280;
                    let h = (original_height as f32 * scale) as u32;
                    (w, h)
                } else {
                    (original_width, original_height)
                }
            }
        }
    }

    /// Build the GStreamer pipeline string for frame extraction.
    ///
    /// Uses Jetson-specific elements:
    /// - nvv4l2decoder: NVDEC hardware decoder
    /// - nvvidconv: GPU color space conversion
    fn build_extraction_pipeline(
        &self,
        video_path: &Path,
        target_width: u32,
        target_height: u32,
    ) -> Result<String> {
        let path_str = video_path
            .to_str()
            .context("Invalid video path (non-UTF8)")?;

        // Check if Jetson-specific elements are available
        let has_nvdec = self.check_element_available("nvv4l2decoder");
        let has_nvvidconv = self.check_element_available("nvvidconv");

        let use_hw_decoder = self.config.use_hw_decoder && has_nvdec;
        let use_hw_converter = self.config.use_hw_converter && has_nvvidconv;

        if use_hw_decoder {
            info!("Using hardware decoder: nvv4l2decoder");
        } else {
            info!("Using software decoder: decodebin");
        }

        // Build pipeline based on available elements
        let pipeline = if use_hw_decoder && use_hw_converter {
            // Full hardware pipeline (Jetson)
            format!(
                "filesrc location=\"{}\" ! \
                 qtdemux ! h264parse ! nvv4l2decoder ! \
                 nvvidconv ! \
                 video/x-raw,format=BGRx,width={},height={} ! \
                 videoconvert ! \
                 video/x-raw,format=RGB ! \
                 jpegenc quality={} ! \
                 appsink name=sink",
                path_str, target_width, target_height, self.config.jpeg_quality
            )
        } else if use_hw_converter {
            // Software decode, hardware convert
            format!(
                "filesrc location=\"{}\" ! \
                 decodebin ! \
                 nvvidconv ! \
                 video/x-raw,format=BGRx,width={},height={} ! \
                 videoconvert ! \
                 video/x-raw,format=RGB ! \
                 jpegenc quality={} ! \
                 appsink name=sink",
                path_str, target_width, target_height, self.config.jpeg_quality
            )
        } else {
            // Fallback: fully software pipeline
            format!(
                "filesrc location=\"{}\" ! \
                 decodebin ! \
                 videoconvert ! \
                 videoscale ! \
                 video/x-raw,format=RGB,width={},height={} ! \
                 jpegenc quality={} ! \
                 appsink name=sink",
                path_str, target_width, target_height, self.config.jpeg_quality
            )
        };

        Ok(pipeline)
    }

    /// Check if a GStreamer element is available.
    fn check_element_available(&self, element_name: &str) -> bool {
        let factory = gst::ElementFactory::find(element_name);
        factory.is_some()
    }

    /// Extract frames at specified intervals using seeking.
    async fn extract_frames_at_intervals(
        &self,
        pipeline: &gst::Pipeline,
        sink: &gstreamer_app::AppSink,
        interval_ms: u64,
        max_frames: u32,
        duration_ms: u64,
    ) -> Result<Vec<ExtractedFrame>> {
        let mut frames = Vec::new();
        let mut frame_number = 0u64;
        let mut current_time_ms = 0u64;

        // Wait for pipeline to be ready
        let bus = pipeline.bus().context("No bus on pipeline")?;

        // Wait for async-done or error
        for msg in bus.iter_timed(gst::ClockTime::from_seconds(5)) {
            match msg.view() {
                gst::MessageView::AsyncDone(_) => break,
                gst::MessageView::Error(e) => {
                    anyhow::bail!("Pipeline error: {:?}", e.error());
                }
                _ => {}
            }
        }

        while current_time_ms < duration_ms && frames.len() < max_frames as usize {
            // Seek to target position
            let seek_time = gst::ClockTime::from_mseconds(current_time_ms);

            let seek_result = pipeline.seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                seek_time,
            );

            if let Err(e) = seek_result {
                warn!("Seek to {}ms failed: {:?}", current_time_ms, e);
                current_time_ms += interval_ms;
                continue;
            }

            // Wait for seek to complete
            for msg in bus.iter_timed(gst::ClockTime::from_seconds(2)) {
                match msg.view() {
                    gst::MessageView::AsyncDone(_) => break,
                    gst::MessageView::Error(e) => {
                        error!("Error during seek: {:?}", e.error());
                        break;
                    }
                    gst::MessageView::Eos(_) => {
                        debug!("End of stream reached");
                        return Ok(frames);
                    }
                    _ => {}
                }
            }

            // Pull frame from appsink
            match sink.try_pull_sample(gst::ClockTime::from_seconds(1)) {
                Some(sample) => {
                    if let Some(buffer) = sample.buffer() {
                        let map = buffer.map_readable().context("Failed to map buffer")?;
                        let jpeg_data = map.as_slice().to_vec();

                        // Get actual dimensions from caps
                        let (width, height) = if let Some(caps) = sample.caps() {
                            let structure = caps.structure(0).context("No caps structure")?;
                            let w = structure.get::<i32>("width").unwrap_or(1280) as u32;
                            let h = structure.get::<i32>("height").unwrap_or(720) as u32;
                            (w, h)
                        } else {
                            (1280, 720)
                        };

                        debug!(
                            "Frame {} @ {}ms: {} bytes JPEG, {}x{}",
                            frame_number,
                            current_time_ms,
                            jpeg_data.len(),
                            width,
                            height
                        );

                        frames.push(ExtractedFrame {
                            number: frame_number,
                            timestamp_ms: current_time_ms,
                            width,
                            height,
                            data: jpeg_data,
                        });

                        frame_number += 1;
                    }
                }
                None => {
                    warn!("No sample available at {}ms", current_time_ms);
                }
            }

            current_time_ms += interval_ms;
        }

        Ok(frames)
    }

    /// Extract frames with a callback for streaming results.
    pub async fn extract_frames_streaming<F>(
        &self,
        video_path: &Path,
        mut callback: F,
    ) -> Result<u32>
    where
        F: FnMut(ExtractedFrame) -> bool,
    {
        let frames = self.extract_frames(video_path).await?;
        let mut count = 0;

        for frame in frames {
            count += 1;
            if !callback(frame) {
                break;
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = ExtractionConfig::default();
        assert_eq!(config.interval_ms, 1000);
        assert_eq!(config.max_frames, 300);
        assert_eq!(config.jpeg_quality, 85);
        assert!(config.use_hw_decoder);
        assert!(config.use_hw_converter);
    }

    #[test]
    fn test_target_size_calculation() {
        let extractor = FrameExtractor::new(ExtractionConfig::default());

        // Large video should be scaled down
        let (w, h) = extractor.calculate_target_size(1920, 1080);
        assert_eq!(w, 1280);
        assert!(h <= 720);

        // Small video should keep original size
        let (w, h) = extractor.calculate_target_size(640, 480);
        assert_eq!(w, 640);
        assert_eq!(h, 480);
    }

    #[test]
    fn test_target_size_with_explicit_width() {
        let config = ExtractionConfig {
            target_width: Some(800),
            ..Default::default()
        };
        let extractor = FrameExtractor::new(config);

        let (w, h) = extractor.calculate_target_size(1920, 1080);
        assert_eq!(w, 800);
        assert_eq!(h, 450); // 1080 * 800/1920
    }
}
