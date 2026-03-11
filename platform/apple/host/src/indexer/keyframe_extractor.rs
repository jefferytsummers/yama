//! Keyframe extraction for video indexing.
//!
//! Extracts keyframes from videos using multiple strategies:
//! - **Interval**: Fixed time intervals
//! - **Scene Change**: Content-aware detection of scene boundaries
//! - **I-Frame**: Extract actual I-frames (keyframes) from the video stream
//! - **Combined**: Scene change detection with interval fallback

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use tokio::fs;
use tracing::{debug, info, warn};

use yama_platform_traits::{ImageFormat, KeyframeStrategy};

use crate::inference::{ExtractionConfig, ExtractedFrame, FrameExtractor, VideoInfo};

/// Keyframe extraction configuration.
#[derive(Debug, Clone)]
pub struct KeyframeConfig {
    /// Extraction strategy.
    pub strategy: KeyframeStrategy,
    /// Interval in milliseconds (for Interval/Combined strategies).
    pub interval_ms: u64,
    /// Scene change threshold (0.0-1.0, lower = more sensitive).
    pub scene_threshold: f64,
    /// Maximum keyframes to extract.
    pub max_keyframes: usize,
    /// Output image format.
    pub output_format: ImageFormat,
    /// JPEG quality (1-100).
    pub jpeg_quality: u8,
    /// Output image dimensions (None = preserve original).
    pub output_size: Option<(u32, u32)>,
    /// Output directory for extracted keyframes.
    pub output_dir: PathBuf,
}

impl Default for KeyframeConfig {
    fn default() -> Self {
        Self {
            strategy: KeyframeStrategy::Combined,
            interval_ms: 1000,
            scene_threshold: 0.3,
            max_keyframes: 500,
            output_format: ImageFormat::Jpeg,
            jpeg_quality: 85,
            output_size: None,
            output_dir: PathBuf::from("keyframes"),
        }
    }
}

/// Extracted keyframe with metadata.
#[derive(Debug, Clone)]
pub struct Keyframe {
    /// Frame number (0-indexed).
    pub frame_number: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Frame width.
    pub width: u32,
    /// Frame height.
    pub height: u32,
    /// Path to saved image file.
    pub image_path: PathBuf,
    /// Image file size in bytes.
    pub file_size: u64,
    /// Extraction method used.
    pub extraction_method: KeyframeStrategy,
    /// Scene change score (if applicable).
    pub scene_score: Option<f64>,
}

/// Progress information for extraction.
#[derive(Debug, Clone)]
pub struct ExtractionProgress {
    /// Total frames to process.
    pub total_frames: u64,
    /// Frames processed so far.
    pub processed_frames: u64,
    /// Keyframes extracted so far.
    pub keyframes_extracted: u64,
    /// Current timestamp being processed.
    pub current_timestamp_ms: u64,
    /// Video duration in milliseconds.
    pub duration_ms: u64,
    /// Percentage complete.
    pub percent: f64,
}

/// Keyframe extractor for video indexing.
pub struct KeyframeExtractor {
    config: KeyframeConfig,
    cancelled: Arc<AtomicBool>,
    keyframes_extracted: Arc<AtomicU64>,
}

impl KeyframeExtractor {
    /// Create a new keyframe extractor.
    pub fn new(config: KeyframeConfig) -> Self {
        Self {
            config,
            cancelled: Arc::new(AtomicBool::new(false)),
            keyframes_extracted: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Cancel ongoing extraction.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Get the number of keyframes extracted so far.
    pub fn keyframes_extracted(&self) -> u64 {
        self.keyframes_extracted.load(Ordering::Relaxed)
    }

    /// Extract keyframes from a video file.
    pub async fn extract(
        &self,
        video_path: &Path,
        progress_callback: Option<Box<dyn Fn(ExtractionProgress) + Send>>,
    ) -> Result<Vec<Keyframe>> {
        self.cancelled.store(false, Ordering::Relaxed);
        self.keyframes_extracted.store(0, Ordering::Relaxed);

        // Ensure output directory exists
        fs::create_dir_all(&self.config.output_dir)
            .await
            .context("Failed to create output directory")?;

        // Create underlying frame extractor
        let extraction_config = ExtractionConfig {
            interval_ms: self.config.interval_ms,
            max_frames: self.config.max_keyframes as u32,
            target_width: self.config.output_size.map(|(w, _)| w),
            target_height: self.config.output_size.map(|(_, h)| h),
            jpeg_quality: self.config.jpeg_quality as u32,
        };
        let frame_extractor = FrameExtractor::new(extraction_config);

        // Get video info
        let video_info = frame_extractor.get_video_info(video_path)?;
        info!(
            "Extracting keyframes from video: {}x{}, {:.1}fps, {}ms",
            video_info.width, video_info.height, video_info.fps, video_info.duration_ms
        );

        // Extract based on strategy
        match self.config.strategy {
            KeyframeStrategy::Interval => {
                self.extract_interval(video_path, &frame_extractor, &video_info, progress_callback)
                    .await
            }
            KeyframeStrategy::SceneChange => {
                self.extract_scene_change(video_path, &frame_extractor, &video_info, progress_callback)
                    .await
            }
            KeyframeStrategy::Iframe => {
                // For now, I-frame extraction falls back to interval
                // Full implementation would use GStreamer to detect actual I-frames
                self.extract_interval(video_path, &frame_extractor, &video_info, progress_callback)
                    .await
            }
            KeyframeStrategy::Combined => {
                self.extract_combined(video_path, &frame_extractor, &video_info, progress_callback)
                    .await
            }
        }
    }

    /// Extract keyframes at fixed intervals.
    async fn extract_interval(
        &self,
        video_path: &Path,
        frame_extractor: &FrameExtractor,
        video_info: &VideoInfo,
        progress_callback: Option<Box<dyn Fn(ExtractionProgress) + Send>>,
    ) -> Result<Vec<Keyframe>> {
        let frames = frame_extractor.extract_frames(video_path).await?;

        self.save_frames(
            &frames,
            video_path,
            video_info,
            KeyframeStrategy::Interval,
            progress_callback,
        )
        .await
    }

    /// Extract keyframes at scene changes.
    async fn extract_scene_change(
        &self,
        video_path: &Path,
        frame_extractor: &FrameExtractor,
        video_info: &VideoInfo,
        progress_callback: Option<Box<dyn Fn(ExtractionProgress) + Send>>,
    ) -> Result<Vec<Keyframe>> {
        // Extract frames densely for scene change detection
        let dense_config = ExtractionConfig {
            interval_ms: 200, // Dense sampling for scene detection
            max_frames: (self.config.max_keyframes * 5) as u32, // More candidates
            target_width: Some(320), // Small size for faster comparison
            target_height: Some(180),
            jpeg_quality: 60,
        };
        let dense_extractor = FrameExtractor::new(dense_config);

        let dense_frames = dense_extractor.extract_frames(video_path).await?;

        if dense_frames.is_empty() {
            return Ok(Vec::new());
        }

        // Detect scene changes using histogram comparison
        let scene_frames = self.detect_scene_changes(&dense_frames).await?;

        // Re-extract scene frames at full quality
        let mut keyframes = Vec::new();
        for (idx, score) in scene_frames {
            if self.cancelled.load(Ordering::Relaxed) {
                break;
            }
            if keyframes.len() >= self.config.max_keyframes {
                break;
            }

            let frame = &dense_frames[idx];
            let keyframe = self
                .save_single_frame(
                    frame,
                    video_path,
                    KeyframeStrategy::SceneChange,
                    Some(score),
                    keyframes.len(),
                )
                .await?;
            keyframes.push(keyframe);

            self.keyframes_extracted
                .fetch_add(1, Ordering::Relaxed);

            if let Some(ref callback) = progress_callback {
                callback(ExtractionProgress {
                    total_frames: dense_frames.len() as u64,
                    processed_frames: (idx + 1) as u64,
                    keyframes_extracted: keyframes.len() as u64,
                    current_timestamp_ms: frame.timestamp_ms,
                    duration_ms: video_info.duration_ms,
                    percent: (keyframes.len() as f64 / self.config.max_keyframes as f64) * 100.0,
                });
            }
        }

        Ok(keyframes)
    }

    /// Extract using combined strategy (scene change + interval fallback).
    async fn extract_combined(
        &self,
        video_path: &Path,
        frame_extractor: &FrameExtractor,
        video_info: &VideoInfo,
        progress_callback: Option<Box<dyn Fn(ExtractionProgress) + Send>>,
    ) -> Result<Vec<Keyframe>> {
        // First, try scene change detection
        let scene_keyframes = self
            .extract_scene_change(video_path, frame_extractor, video_info, None)
            .await?;

        // If we got enough scene changes, use those
        if scene_keyframes.len() >= self.config.max_keyframes / 2 {
            return Ok(scene_keyframes);
        }

        // Otherwise, fill in gaps with interval-based extraction
        let interval_frames = frame_extractor.extract_frames(video_path).await?;

        let mut keyframes = scene_keyframes;
        let mut interval_idx = 0;

        for frame in &interval_frames {
            if self.cancelled.load(Ordering::Relaxed) {
                break;
            }
            if keyframes.len() >= self.config.max_keyframes {
                break;
            }

            // Check if we already have a keyframe near this timestamp
            let min_gap_ms = self.config.interval_ms / 2;
            let has_nearby = keyframes
                .iter()
                .any(|kf| (kf.timestamp_ms as i64 - frame.timestamp_ms as i64).unsigned_abs() < min_gap_ms);

            if !has_nearby {
                let keyframe = self
                    .save_single_frame(
                        frame,
                        video_path,
                        KeyframeStrategy::Interval,
                        None,
                        keyframes.len(),
                    )
                    .await?;
                keyframes.push(keyframe);

                self.keyframes_extracted
                    .fetch_add(1, Ordering::Relaxed);

                if let Some(ref callback) = progress_callback {
                    callback(ExtractionProgress {
                        total_frames: interval_frames.len() as u64,
                        processed_frames: (interval_idx + 1) as u64,
                        keyframes_extracted: keyframes.len() as u64,
                        current_timestamp_ms: frame.timestamp_ms,
                        duration_ms: video_info.duration_ms,
                        percent: (keyframes.len() as f64 / self.config.max_keyframes as f64) * 100.0,
                    });
                }
            }

            interval_idx += 1;
        }

        // Sort by timestamp
        keyframes.sort_by_key(|kf| kf.timestamp_ms);

        Ok(keyframes)
    }

    /// Detect scene changes using simple histogram comparison.
    async fn detect_scene_changes(
        &self,
        frames: &[ExtractedFrame],
    ) -> Result<Vec<(usize, f64)>> {
        let mut scene_changes = Vec::new();

        // Always include first frame
        if !frames.is_empty() {
            scene_changes.push((0, 1.0));
        }

        // Compare consecutive frames
        let mut prev_histogram: Option<Vec<u32>> = None;

        for (idx, frame) in frames.iter().enumerate() {
            if self.cancelled.load(Ordering::Relaxed) {
                break;
            }

            let histogram = self.compute_histogram(&frame.data);

            if let Some(ref prev) = prev_histogram {
                let diff = self.histogram_difference(prev, &histogram);

                // If difference exceeds threshold, it's a scene change
                if diff > self.config.scene_threshold {
                    scene_changes.push((idx, diff));
                    debug!(
                        "Scene change detected at {}ms (diff: {:.3})",
                        frame.timestamp_ms, diff
                    );
                }
            }

            prev_histogram = Some(histogram);
        }

        // Limit to max_keyframes
        if scene_changes.len() > self.config.max_keyframes {
            // Keep frames with highest scene change scores
            scene_changes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            scene_changes.truncate(self.config.max_keyframes);
            scene_changes.sort_by_key(|a| a.0);
        }

        Ok(scene_changes)
    }

    /// Compute a simple grayscale histogram from JPEG data.
    fn compute_histogram(&self, jpeg_data: &[u8]) -> Vec<u32> {
        // Decode JPEG to get pixel values
        let img = image::load_from_memory(jpeg_data);
        let mut histogram = vec![0u32; 256];

        if let Ok(img) = img {
            let gray = img.to_luma8();
            for pixel in gray.pixels() {
                histogram[pixel.0[0] as usize] += 1;
            }
        }

        histogram
    }

    /// Compute normalized difference between two histograms.
    fn histogram_difference(&self, hist1: &[u32], hist2: &[u32]) -> f64 {
        let total1: u32 = hist1.iter().sum();
        let total2: u32 = hist2.iter().sum();

        if total1 == 0 || total2 == 0 {
            return 1.0;
        }

        let mut diff = 0.0;
        for i in 0..256 {
            let n1 = hist1[i] as f64 / total1 as f64;
            let n2 = hist2[i] as f64 / total2 as f64;
            diff += (n1 - n2).abs();
        }

        // Normalize to 0.0-1.0 range
        diff / 2.0
    }

    /// Save extracted frames to disk.
    async fn save_frames(
        &self,
        frames: &[ExtractedFrame],
        video_path: &Path,
        video_info: &VideoInfo,
        method: KeyframeStrategy,
        progress_callback: Option<Box<dyn Fn(ExtractionProgress) + Send>>,
    ) -> Result<Vec<Keyframe>> {
        let mut keyframes = Vec::new();

        for (idx, frame) in frames.iter().enumerate() {
            if self.cancelled.load(Ordering::Relaxed) {
                break;
            }
            if keyframes.len() >= self.config.max_keyframes {
                break;
            }

            let keyframe = self
                .save_single_frame(frame, video_path, method, None, idx)
                .await?;
            keyframes.push(keyframe);

            self.keyframes_extracted
                .fetch_add(1, Ordering::Relaxed);

            if let Some(ref callback) = progress_callback {
                callback(ExtractionProgress {
                    total_frames: frames.len() as u64,
                    processed_frames: (idx + 1) as u64,
                    keyframes_extracted: keyframes.len() as u64,
                    current_timestamp_ms: frame.timestamp_ms,
                    duration_ms: video_info.duration_ms,
                    percent: ((idx + 1) as f64 / frames.len() as f64) * 100.0,
                });
            }
        }

        Ok(keyframes)
    }

    /// Save a single frame to disk.
    async fn save_single_frame(
        &self,
        frame: &ExtractedFrame,
        video_path: &Path,
        method: KeyframeStrategy,
        scene_score: Option<f64>,
        index: usize,
    ) -> Result<Keyframe> {
        // Generate filename
        let video_stem = video_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("video");
        let extension = self.config.output_format.extension();
        let filename = format!(
            "{}_{}_{:05}.{}",
            video_stem, frame.timestamp_ms, index, extension
        );
        let image_path = self.config.output_dir.join(&filename);

        // Save frame data
        let data = if self.config.output_format == ImageFormat::Jpeg {
            // Already JPEG, just write it
            frame.data.clone()
        } else {
            // Convert format
            let img = image::load_from_memory(&frame.data)
                .context("Failed to decode frame")?;

            let mut buffer = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut buffer);

            match self.config.output_format {
                ImageFormat::Png => {
                    img.write_to(&mut cursor, image::ImageFormat::Png)
                        .context("Failed to encode PNG")?;
                }
                ImageFormat::WebP => {
                    img.write_to(&mut cursor, image::ImageFormat::WebP)
                        .context("Failed to encode WebP")?;
                }
                _ => {
                    img.write_to(&mut cursor, image::ImageFormat::Jpeg)
                        .context("Failed to encode JPEG")?;
                }
            }
            buffer
        };

        let file_size = data.len() as u64;
        fs::write(&image_path, &data)
            .await
            .context("Failed to write keyframe")?;

        Ok(Keyframe {
            frame_number: frame.number,
            timestamp_ms: frame.timestamp_ms,
            width: frame.width,
            height: frame.height,
            image_path,
            file_size,
            extraction_method: method,
            scene_score,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = KeyframeConfig::default();
        assert_eq!(config.strategy, KeyframeStrategy::Combined);
        assert_eq!(config.interval_ms, 1000);
        assert_eq!(config.max_keyframes, 500);
    }

    #[test]
    fn test_histogram_difference() {
        let extractor = KeyframeExtractor::new(KeyframeConfig::default());

        // Same histogram should have zero difference
        let hist1 = vec![100u32; 256];
        let diff = extractor.histogram_difference(&hist1, &hist1);
        assert!(diff < 0.001);

        // Very different histograms
        let mut hist2 = vec![0u32; 256];
        hist2[0] = 1000;
        let mut hist3 = vec![0u32; 256];
        hist3[255] = 1000;
        let diff = extractor.histogram_difference(&hist2, &hist3);
        assert!(diff > 0.9);
    }
}
