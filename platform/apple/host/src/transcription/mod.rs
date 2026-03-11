//! Audio transcription module.
//!
//! This module provides speech-to-text transcription using Whisper.
//!
//! # Features
//!
//! - Multiple Whisper model sizes (Tiny, Base, Small, Medium, Large)
//! - Word-level timestamps
//! - Language detection and translation
//! - Metal acceleration on Apple Silicon
//! - Audio extraction from video files
//!
//! # Usage
//!
//! ```rust,ignore
//! use yama_host_apple::transcription::{WhisperTranscriber, WhisperConfig, ModelSize};
//!
//! let config = WhisperConfig {
//!     model_size: ModelSize::Base,
//!     ..Default::default()
//! };
//! let transcriber = WhisperTranscriber::new("whisper-base.bin", config)?;
//!
//! let result = transcriber.transcribe(&audio_samples)?;
//! for segment in result.segments {
//!     println!("[{:.2}s - {:.2}s] {}", segment.start, segment.end, segment.text);
//! }
//! ```

pub mod audio_extractor;
pub mod whisper;

pub use audio_extractor::{AudioExtractor, AudioFormat, ExtractedAudio};
pub use whisper::{
    ModelSize, TranscriptionResult, TranscriptionSegment, WhisperConfig, WhisperTranscriber,
    Word,
};
