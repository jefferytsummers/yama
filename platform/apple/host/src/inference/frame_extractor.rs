//! Frame extraction from video files.
//!
//! Uses GStreamer with VideoToolbox for hardware-accelerated decoding on Apple.

use std::path::Path;

use anyhow::{Context, Result};
use tracing::{debug, info};

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
    /// Raw pixel data (NV12 format).
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
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            max_frames: 300,
            target_width: None,
            target_height: None,
        }
    }
}

/// Frame extractor for video files.
pub struct FrameExtractor {
    config: ExtractionConfig,
}

impl FrameExtractor {
    /// Create a new frame extractor.
    pub fn new(config: ExtractionConfig) -> Self {
        Self { config }
    }

    /// Get video metadata without extracting frames.
    pub fn get_video_info(&self, video_path: &Path) -> Result<VideoInfo> {
        // For now, return mock video info
        // TODO: Implement actual video probing with GStreamer
        debug!("Getting video info for: {:?}", video_path);

        Ok(VideoInfo {
            duration_ms: 10000, // 10 seconds
            width: 1920,
            height: 1080,
            fps: 30.0,
            codec: "h264".to_string(),
            estimated_frames: 10, // Based on our interval
        })
    }

    /// Extract frames from a video file.
    ///
    /// Returns an iterator/stream of frames. For now, this is a placeholder
    /// that will be implemented with actual GStreamer extraction.
    pub async fn extract_frames(
        &self,
        video_path: &Path,
    ) -> Result<Vec<ExtractedFrame>> {
        info!("Extracting frames from: {:?}", video_path);

        // Get video info
        let info = self.get_video_info(video_path)?;

        // Calculate how many frames to extract
        let frame_count = std::cmp::min(
            (info.duration_ms / self.config.interval_ms) as u32,
            self.config.max_frames,
        );

        // For now, return mock frames
        // TODO: Implement actual frame extraction with GStreamer
        let mut frames = Vec::with_capacity(frame_count as usize);

        for i in 0..frame_count {
            let timestamp_ms = i as u64 * self.config.interval_ms;

            frames.push(ExtractedFrame {
                number: i as u64,
                timestamp_ms,
                width: info.width,
                height: info.height,
                data: Vec::new(), // Empty for mock
            });
        }

        info!("Extracted {} frames", frames.len());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_frame_extraction_mock() {
        let config = ExtractionConfig {
            interval_ms: 1000,
            max_frames: 5,
            ..Default::default()
        };

        let extractor = FrameExtractor::new(config);
        let frames = extractor
            .extract_frames(&PathBuf::from("/tmp/test.mp4"))
            .await
            .unwrap();

        assert_eq!(frames.len(), 5);
        assert_eq!(frames[0].timestamp_ms, 0);
        assert_eq!(frames[1].timestamp_ms, 1000);
    }
}
