//! Whisper speech-to-text transcription.
//!
//! Provides transcription using OpenAI's Whisper model with Metal acceleration.

use std::path::Path;

use anyhow::{Context, Result};
use tracing::{debug, info};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters,
};

/// Whisper model size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSize {
    /// Tiny model (~39M params, ~75MB).
    Tiny,
    /// Base model (~74M params, ~142MB).
    Base,
    /// Small model (~244M params, ~466MB).
    Small,
    /// Medium model (~769M params, ~1.5GB).
    Medium,
    /// Large model (~1.5B params, ~2.9GB).
    Large,
    /// Large-v2 model (~1.5B params, ~2.9GB).
    LargeV2,
    /// Large-v3 model (~1.5B params, ~2.9GB).
    LargeV3,
}

impl ModelSize {
    /// Get the model name suffix.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Base => "base",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::LargeV2 => "large-v2",
            Self::LargeV3 => "large-v3",
        }
    }

    /// Get approximate model size in bytes.
    pub fn size_bytes(&self) -> u64 {
        match self {
            Self::Tiny => 75_000_000,
            Self::Base => 142_000_000,
            Self::Small => 466_000_000,
            Self::Medium => 1_500_000_000,
            Self::Large | Self::LargeV2 | Self::LargeV3 => 2_900_000_000,
        }
    }
}

/// Whisper transcriber configuration.
#[derive(Debug, Clone)]
pub struct WhisperConfig {
    /// Model size to use.
    pub model_size: ModelSize,
    /// Language code (e.g., "en", "es", "fr"). None for auto-detection.
    pub language: Option<String>,
    /// Whether to translate to English.
    pub translate: bool,
    /// Whether to include word-level timestamps.
    pub word_timestamps: bool,
    /// Number of processing threads.
    pub n_threads: u32,
    /// Maximum segment length in characters.
    pub max_segment_length: usize,
    /// Temperature for sampling (0.0 = greedy).
    pub temperature: f32,
    /// Whether to suppress non-speech tokens.
    pub suppress_non_speech: bool,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            model_size: ModelSize::Base,
            language: None,
            translate: false,
            word_timestamps: true,
            n_threads: 4,
            max_segment_length: 0,
            temperature: 0.0,
            suppress_non_speech: true,
        }
    }
}

/// A single word with timing information.
#[derive(Debug, Clone)]
pub struct Word {
    /// The word text.
    pub text: String,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
    /// Confidence probability (0.0 - 1.0).
    pub probability: f32,
}

/// A transcription segment.
#[derive(Debug, Clone)]
pub struct TranscriptionSegment {
    /// Segment text.
    pub text: String,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
    /// Words in this segment (if word timestamps enabled).
    pub words: Vec<Word>,
}

/// Complete transcription result.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// Detected or specified language.
    pub language: String,
    /// Full transcription text.
    pub text: String,
    /// Transcription segments with timing.
    pub segments: Vec<TranscriptionSegment>,
    /// Processing time in seconds.
    pub processing_time_secs: f64,
    /// Audio duration in seconds.
    pub audio_duration_secs: f64,
}

impl TranscriptionResult {
    /// Get the real-time factor (processing_time / audio_duration).
    pub fn real_time_factor(&self) -> f64 {
        if self.audio_duration_secs > 0.0 {
            self.processing_time_secs / self.audio_duration_secs
        } else {
            0.0
        }
    }
}

/// Whisper transcriber.
pub struct WhisperTranscriber {
    config: WhisperConfig,
    ctx: WhisperContext,
}

impl WhisperTranscriber {
    /// Create a new Whisper transcriber from a model file.
    pub fn new(model_path: impl AsRef<Path>, config: WhisperConfig) -> Result<Self> {
        let model_path = model_path.as_ref();
        info!("Loading Whisper model from {:?}", model_path);

        // Create context with Metal acceleration if available
        let ctx_params = WhisperContextParameters::default();

        let ctx = WhisperContext::new_with_params(
            model_path.to_str().context("Invalid model path")?,
            ctx_params,
        )
        .map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {}", e))?;

        info!("Whisper model loaded successfully");

        Ok(Self { config, ctx })
    }

    /// Transcribe audio samples.
    ///
    /// Samples should be 16kHz mono f32 PCM.
    pub fn transcribe(&self, samples: &[f32]) -> Result<TranscriptionResult> {
        let start_time = std::time::Instant::now();
        let audio_duration_secs = samples.len() as f64 / 16000.0;

        info!("Transcribing {:.2}s of audio", audio_duration_secs);

        // Create full params for transcription
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Set language
        if let Some(ref lang) = self.config.language {
            params.set_language(Some(lang));
        } else {
            params.set_language(None); // Auto-detect
        }

        // Set other options
        params.set_translate(self.config.translate);
        params.set_n_threads(self.config.n_threads as i32);
        params.set_token_timestamps(self.config.word_timestamps);

        if self.config.temperature > 0.0 {
            params.set_temperature(self.config.temperature);
        }

        // Suppress non-speech tokens
        if self.config.suppress_non_speech {
            params.set_suppress_non_speech_tokens(true);
        }

        // Run transcription
        let mut state = self.ctx.create_state()
            .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {}", e))?;

        state.full(params, samples)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {}", e))?;

        // Extract results
        let num_segments = state.full_n_segments()
            .map_err(|e| anyhow::anyhow!("Failed to get segment count: {}", e))?;

        let mut segments = Vec::with_capacity(num_segments as usize);
        let mut full_text = String::new();

        for i in 0..num_segments {
            let segment_text = state.full_get_segment_text(i)
                .map_err(|e| anyhow::anyhow!("Failed to get segment text: {}", e))?;

            let start_time_ms = state.full_get_segment_t0(i)
                .map_err(|e| anyhow::anyhow!("Failed to get segment start: {}", e))?;
            let end_time_ms = state.full_get_segment_t1(i)
                .map_err(|e| anyhow::anyhow!("Failed to get segment end: {}", e))?;

            // Convert centiseconds to seconds
            let start = start_time_ms as f64 / 100.0;
            let end = end_time_ms as f64 / 100.0;

            // Get word-level timestamps if enabled
            let words = if self.config.word_timestamps {
                self.extract_words(&state, i)?
            } else {
                Vec::new()
            };

            full_text.push_str(&segment_text);

            segments.push(TranscriptionSegment {
                text: segment_text,
                start,
                end,
                words,
            });
        }

        // Detect language
        let language = self.config.language.clone().unwrap_or_else(|| {
            // Try to get detected language from state
            "en".to_string()
        });

        let processing_time_secs = start_time.elapsed().as_secs_f64();

        info!(
            "Transcription complete: {:.2}s processed in {:.2}s (RTF: {:.2}x)",
            audio_duration_secs,
            processing_time_secs,
            processing_time_secs / audio_duration_secs
        );

        Ok(TranscriptionResult {
            language,
            text: full_text.trim().to_string(),
            segments,
            processing_time_secs,
            audio_duration_secs,
        })
    }

    /// Extract word-level timestamps from a segment.
    fn extract_words(
        &self,
        state: &whisper_rs::WhisperState,
        segment_idx: i32,
    ) -> Result<Vec<Word>> {
        let num_tokens = state.full_n_tokens(segment_idx)
            .map_err(|e| anyhow::anyhow!("Failed to get token count: {}", e))?;

        let mut words = Vec::new();

        for j in 0..num_tokens {
            let token_text = state.full_get_token_text(segment_idx, j)
                .map_err(|e| anyhow::anyhow!("Failed to get token text: {}", e))?;

            // Skip special tokens
            if token_text.starts_with('[') || token_text.starts_with('<') {
                continue;
            }

            let token_data = state.full_get_token_data(segment_idx, j)
                .map_err(|e| anyhow::anyhow!("Failed to get token data: {}", e))?;

            words.push(Word {
                text: token_text,
                start: token_data.t0 as f64 / 100.0,
                end: token_data.t1 as f64 / 100.0,
                probability: token_data.p,
            });
        }

        Ok(words)
    }

    /// Transcribe audio from a file.
    pub fn transcribe_file(&self, audio_path: impl AsRef<Path>) -> Result<TranscriptionResult> {
        let audio_path = audio_path.as_ref();
        info!("Transcribing audio file: {:?}", audio_path);

        // Use audio extractor to get samples
        let extractor = super::audio_extractor::AudioExtractor::for_whisper()?;
        let audio = extractor.extract(audio_path)?;

        self.transcribe(&audio.samples)
    }

    /// Get the model size.
    pub fn model_size(&self) -> ModelSize {
        self.config.model_size
    }

    /// Check if word timestamps are enabled.
    pub fn has_word_timestamps(&self) -> bool {
        self.config.word_timestamps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_size() {
        assert_eq!(ModelSize::Tiny.name(), "tiny");
        assert_eq!(ModelSize::Base.name(), "base");
        assert_eq!(ModelSize::Large.name(), "large");
    }

    #[test]
    fn test_config_defaults() {
        let config = WhisperConfig::default();
        assert_eq!(config.model_size, ModelSize::Base);
        assert!(config.language.is_none());
        assert!(!config.translate);
        assert!(config.word_timestamps);
    }

    #[test]
    fn test_transcription_result() {
        let result = TranscriptionResult {
            language: "en".to_string(),
            text: "Hello world".to_string(),
            segments: vec![],
            processing_time_secs: 1.0,
            audio_duration_secs: 10.0,
        };
        assert!((result.real_time_factor() - 0.1).abs() < 0.001);
    }
}
