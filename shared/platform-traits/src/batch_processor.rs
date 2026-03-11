//! Batch video processing trait.
//!
//! This module defines the [`BatchProcessor`] trait for processing multiple video
//! files through the indexing pipeline (keyframe extraction, embedding, transcription).
//!
//! # Usage
//!
//! ```rust,ignore
//! use yama_platform_traits::{BatchProcessor, BatchConfig, BatchProgress};
//!
//! let config = BatchConfig::default();
//! let processor = create_batch_processor().await?;
//!
//! let progress_callback = |progress: BatchProgress| {
//!     println!("{}% - {:?}", progress.percent, progress.stage);
//! };
//!
//! let result = processor.process_videos(&paths, &config, Some(progress_callback)).await?;
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::PlatformResult;

/// Callback type for progress reporting.
pub type ProgressCallback = Arc<dyn Fn(BatchProgress) + Send + Sync>;

/// Configuration for batch video processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Number of concurrent videos to process.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    /// Keyframe extraction configuration.
    #[serde(default)]
    pub keyframe: KeyframeConfig,

    /// Embedding generation configuration.
    #[serde(default)]
    pub embedding: EmbeddingConfig,

    /// Transcription configuration.
    #[serde(default)]
    pub transcription: TranscriptionConfig,

    /// How to handle errors.
    #[serde(default)]
    pub error_strategy: ErrorStrategy,

    /// Whether to skip already-processed videos.
    #[serde(default = "default_skip_existing")]
    pub skip_existing: bool,

    /// Output directory for generated artifacts.
    pub output_dir: Option<PathBuf>,
}

fn default_concurrency() -> usize {
    4
}

fn default_skip_existing() -> bool {
    true
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            keyframe: KeyframeConfig::default(),
            embedding: EmbeddingConfig::default(),
            transcription: TranscriptionConfig::default(),
            error_strategy: ErrorStrategy::default(),
            skip_existing: default_skip_existing(),
            output_dir: None,
        }
    }
}

/// Keyframe extraction configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeConfig {
    /// Extraction strategy.
    #[serde(default)]
    pub strategy: KeyframeStrategy,

    /// Target interval in milliseconds (for Interval strategy).
    #[serde(default = "default_interval_ms")]
    pub interval_ms: u64,

    /// Scene change threshold (0.0-1.0, for SceneChange strategy).
    #[serde(default = "default_scene_threshold")]
    pub scene_threshold: f64,

    /// Maximum keyframes per video.
    #[serde(default = "default_max_keyframes")]
    pub max_keyframes: usize,

    /// Output image format.
    #[serde(default)]
    pub output_format: ImageFormat,

    /// JPEG quality (1-100).
    #[serde(default = "default_jpeg_quality")]
    pub jpeg_quality: u8,

    /// Output image dimensions (None = preserve original).
    pub output_size: Option<(u32, u32)>,
}

fn default_interval_ms() -> u64 {
    1000
}

fn default_scene_threshold() -> f64 {
    0.3
}

fn default_max_keyframes() -> usize {
    500
}

fn default_jpeg_quality() -> u8 {
    85
}

impl Default for KeyframeConfig {
    fn default() -> Self {
        Self {
            strategy: KeyframeStrategy::Combined,
            interval_ms: default_interval_ms(),
            scene_threshold: default_scene_threshold(),
            max_keyframes: default_max_keyframes(),
            output_format: ImageFormat::Jpeg,
            jpeg_quality: default_jpeg_quality(),
            output_size: None,
        }
    }
}

/// Keyframe extraction strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyframeStrategy {
    /// Extract at fixed intervals.
    Interval,
    /// Extract at scene changes.
    SceneChange,
    /// Extract I-frames only.
    Iframe,
    /// Combined: scene changes + interval fallback.
    #[default]
    Combined,
}

/// Output image format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// JPEG format (lossy, smaller files).
    #[default]
    Jpeg,
    /// PNG format (lossless, larger files).
    Png,
    /// WebP format (modern, good compression).
    WebP,
}

impl ImageFormat {
    /// Get the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
        }
    }
}

/// Embedding generation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Whether to generate embeddings.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Model to use for embeddings.
    #[serde(default = "default_clip_model")]
    pub model: String,

    /// Batch size for embedding generation.
    #[serde(default = "default_embed_batch_size")]
    pub batch_size: usize,
}

fn default_enabled() -> bool {
    true
}

fn default_clip_model() -> String {
    "clip-vit-base-patch32".to_string()
}

fn default_embed_batch_size() -> usize {
    32
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            model: default_clip_model(),
            batch_size: default_embed_batch_size(),
        }
    }
}

/// Transcription configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Whether to transcribe audio.
    #[serde(default)]
    pub enabled: bool,

    /// Whisper model size.
    #[serde(default)]
    pub model_size: WhisperModelSize,

    /// Language hint (ISO 639-1 code, e.g., "en").
    pub language: Option<String>,

    /// Whether to include word-level timestamps.
    #[serde(default = "default_enabled")]
    pub word_timestamps: bool,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_size: WhisperModelSize::Base,
            language: None,
            word_timestamps: true,
        }
    }
}

/// Whisper model sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhisperModelSize {
    /// Tiny model (~75MB, fastest).
    Tiny,
    /// Base model (~142MB).
    #[default]
    Base,
    /// Small model (~466MB).
    Small,
    /// Medium model (~1.5GB).
    Medium,
    /// Large model (~2.9GB, most accurate).
    Large,
}

impl WhisperModelSize {
    /// Get the model name for loading.
    pub fn model_name(&self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Base => "base",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    /// Approximate model size in bytes.
    pub fn size_bytes(&self) -> u64 {
        match self {
            Self::Tiny => 75_000_000,
            Self::Base => 142_000_000,
            Self::Small => 466_000_000,
            Self::Medium => 1_500_000_000,
            Self::Large => 2_900_000_000,
        }
    }
}

/// How to handle errors during batch processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStrategy {
    /// Stop processing on first error.
    StopOnError,
    /// Skip failed videos and continue.
    #[default]
    SkipAndContinue,
    /// Retry failed videos up to N times.
    RetryWithBackoff,
}

/// Current processing stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStage {
    /// Scanning for video files.
    Scanning,
    /// Extracting keyframes.
    ExtractingKeyframes,
    /// Generating embeddings.
    GeneratingEmbeddings,
    /// Transcribing audio.
    Transcribing,
    /// Writing to database.
    WritingDatabase,
    /// Processing complete.
    Complete,
    /// Processing failed.
    Failed,
}

impl BatchStage {
    /// Human-readable stage name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Scanning => "Scanning",
            Self::ExtractingKeyframes => "Extracting keyframes",
            Self::GeneratingEmbeddings => "Generating embeddings",
            Self::Transcribing => "Transcribing",
            Self::WritingDatabase => "Writing to database",
            Self::Complete => "Complete",
            Self::Failed => "Failed",
        }
    }
}

/// Progress information for batch processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
    /// Current processing stage.
    pub stage: BatchStage,

    /// Overall progress percentage (0-100).
    pub percent: f64,

    /// Current video being processed.
    pub current_video: Option<String>,

    /// Index of current video (1-based).
    pub current_index: usize,

    /// Total number of videos to process.
    pub total_videos: usize,

    /// Number of videos completed successfully.
    pub completed_videos: usize,

    /// Number of videos that failed.
    pub failed_videos: usize,

    /// Number of videos skipped (already processed).
    pub skipped_videos: usize,

    /// Keyframes extracted so far.
    pub keyframes_extracted: usize,

    /// Embeddings generated so far.
    pub embeddings_generated: usize,

    /// Transcription segments generated.
    pub transcripts_generated: usize,

    /// Estimated time remaining in seconds.
    pub eta_seconds: Option<f64>,

    /// Processing rate (videos per second).
    pub rate_videos_per_second: Option<f64>,
}

impl Default for BatchProgress {
    fn default() -> Self {
        Self {
            stage: BatchStage::Scanning,
            percent: 0.0,
            current_video: None,
            current_index: 0,
            total_videos: 0,
            completed_videos: 0,
            failed_videos: 0,
            skipped_videos: 0,
            keyframes_extracted: 0,
            embeddings_generated: 0,
            transcripts_generated: 0,
            eta_seconds: None,
            rate_videos_per_second: None,
        }
    }
}

/// Result of batch processing a single video.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProcessingResult {
    /// Path to the video file.
    pub path: PathBuf,

    /// Whether processing was successful.
    pub success: bool,

    /// Error message if failed.
    pub error: Option<String>,

    /// Whether this video was skipped (already processed).
    pub skipped: bool,

    /// Number of keyframes extracted.
    pub keyframes_extracted: usize,

    /// Number of embeddings generated.
    pub embeddings_generated: usize,

    /// Number of transcript segments.
    pub transcript_segments: usize,

    /// Processing time in milliseconds.
    pub processing_time_ms: u64,
}

/// Result of batch processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Total number of videos processed.
    pub total_videos: usize,

    /// Number of videos processed successfully.
    pub successful: usize,

    /// Number of videos that failed.
    pub failed: usize,

    /// Number of videos skipped (already processed).
    pub skipped: usize,

    /// Total keyframes extracted.
    pub total_keyframes: usize,

    /// Total embeddings generated.
    pub total_embeddings: usize,

    /// Total transcript segments.
    pub total_transcripts: usize,

    /// Total processing time in milliseconds.
    pub total_time_ms: u64,

    /// Per-video results.
    pub video_results: Vec<VideoProcessingResult>,
}

impl BatchResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            total_videos: 0,
            successful: 0,
            failed: 0,
            skipped: 0,
            total_keyframes: 0,
            total_embeddings: 0,
            total_transcripts: 0,
            total_time_ms: 0,
            video_results: Vec::new(),
        }
    }

    /// Add a video result.
    pub fn add_result(&mut self, result: VideoProcessingResult) {
        self.total_videos += 1;
        if result.skipped {
            self.skipped += 1;
        } else if result.success {
            self.successful += 1;
            self.total_keyframes += result.keyframes_extracted;
            self.total_embeddings += result.embeddings_generated;
            self.total_transcripts += result.transcript_segments;
        } else {
            self.failed += 1;
        }
        self.total_time_ms += result.processing_time_ms;
        self.video_results.push(result);
    }
}

impl Default for BatchResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch processor status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Processor is idle.
    Idle,
    /// Processing is in progress.
    Running,
    /// Processing was cancelled.
    Cancelled,
    /// Processing completed.
    Completed,
    /// Processing failed.
    Failed,
}

/// Batch video processor trait.
///
/// This trait defines the interface for processing multiple video files
/// through the indexing pipeline, including keyframe extraction, embedding
/// generation, and transcription.
#[async_trait]
pub trait BatchProcessor: Send + Sync {
    /// Process a batch of video files.
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to video files to process
    /// * `config` - Processing configuration
    /// * `progress` - Optional progress callback
    ///
    /// # Returns
    ///
    /// A `BatchResult` containing the results of processing.
    async fn process_videos(
        &self,
        paths: &[PathBuf],
        config: &BatchConfig,
        progress: Option<ProgressCallback>,
    ) -> PlatformResult<BatchResult>;

    /// Cancel ongoing processing.
    ///
    /// This is a best-effort cancellation. The current video may complete
    /// before processing stops.
    async fn cancel(&self) -> PlatformResult<()>;

    /// Get the current processing status.
    fn status(&self) -> BatchStatus;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_defaults() {
        let config = BatchConfig::default();
        assert_eq!(config.concurrency, 4);
        assert!(config.skip_existing);
        assert_eq!(config.keyframe.strategy, KeyframeStrategy::Combined);
    }

    #[test]
    fn test_batch_result_aggregation() {
        let mut result = BatchResult::new();

        result.add_result(VideoProcessingResult {
            path: PathBuf::from("video1.mp4"),
            success: true,
            error: None,
            skipped: false,
            keyframes_extracted: 100,
            embeddings_generated: 100,
            transcript_segments: 50,
            processing_time_ms: 5000,
        });

        result.add_result(VideoProcessingResult {
            path: PathBuf::from("video2.mp4"),
            success: false,
            error: Some("Decode error".to_string()),
            skipped: false,
            keyframes_extracted: 0,
            embeddings_generated: 0,
            transcript_segments: 0,
            processing_time_ms: 1000,
        });

        result.add_result(VideoProcessingResult {
            path: PathBuf::from("video3.mp4"),
            success: true,
            error: None,
            skipped: true,
            keyframes_extracted: 0,
            embeddings_generated: 0,
            transcript_segments: 0,
            processing_time_ms: 10,
        });

        assert_eq!(result.total_videos, 3);
        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.total_keyframes, 100);
        assert_eq!(result.total_embeddings, 100);
        assert_eq!(result.total_transcripts, 50);
    }

    #[test]
    fn test_image_format_extension() {
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::WebP.extension(), "webp");
    }

    #[test]
    fn test_whisper_model_size() {
        assert_eq!(WhisperModelSize::Tiny.model_name(), "tiny");
        assert!(WhisperModelSize::Large.size_bytes() > WhisperModelSize::Tiny.size_bytes());
    }
}
