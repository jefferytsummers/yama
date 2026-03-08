//! Configuration for the VLM service.

use std::path::PathBuf;

use serde::Deserialize;

/// VLM service configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct VlmConfig {
    /// Event bus socket path (Unix socket).
    #[serde(default = "default_event_bus_socket")]
    pub event_bus_socket: String,

    /// Event bus WebSocket URL (takes precedence over socket if set).
    #[serde(default = "default_event_bus_url")]
    pub event_bus_url: Option<String>,

    /// Model configuration.
    #[serde(default)]
    pub model: ModelConfig,

    /// Live stream configuration.
    #[serde(default)]
    pub live_stream: LiveStreamConfig,

    /// Batch processing configuration.
    #[serde(default)]
    pub batch: BatchConfig,
}

fn default_event_bus_socket() -> String {
    std::env::var("YAMA_EVENT_BUS").unwrap_or_else(|_| "/tmp/yama-event.sock".to_string())
}

fn default_event_bus_url() -> Option<String> {
    std::env::var("YAMA_EVENT_BUS_URL").ok()
}

/// Model configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    /// HuggingFace model ID or local path.
    #[serde(default = "default_model_id")]
    pub model_id: String,

    /// Local model path (takes precedence over model_id).
    pub model_path: Option<PathBuf>,

    /// In-situ quantization level.
    #[serde(default = "default_isq")]
    pub isq: String,

    /// Maximum context length.
    #[serde(default = "default_max_context")]
    pub max_context: u32,

    /// Default temperature for generation.
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Default max tokens for generation.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Device to run inference on.
    #[serde(default = "default_device")]
    pub device: String,
}

fn default_model_id() -> String {
    std::env::var("VLM_MODEL_ID").unwrap_or_else(|_| "Qwen/Qwen2.5-VL-7B-Instruct".to_string())
}

fn default_isq() -> String {
    std::env::var("VLM_ISQ").unwrap_or_else(|_| "Q4K".to_string())
}

fn default_max_context() -> u32 {
    std::env::var("VLM_MAX_CONTEXT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4096)
}

fn default_temperature() -> f32 {
    std::env::var("VLM_TEMPERATURE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.7)
}

fn default_max_tokens() -> u32 {
    std::env::var("VLM_MAX_TOKENS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(512)
}

fn default_device() -> String {
    std::env::var("VLM_DEVICE").unwrap_or_else(|_| "metal".to_string())
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_id: default_model_id(),
            model_path: None,
            isq: default_isq(),
            max_context: default_max_context(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            device: default_device(),
        }
    }
}

/// Live stream analysis configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LiveStreamConfig {
    /// Whether live stream analysis is enabled.
    #[serde(default = "default_live_enabled")]
    pub enabled: bool,

    /// Sampling interval in milliseconds.
    #[serde(default = "default_sample_interval_ms")]
    pub sample_interval_ms: u64,

    /// Default prompt for live analysis.
    #[serde(default = "default_live_prompt")]
    pub default_prompt: String,

    /// Maximum concurrent sources to analyze.
    #[serde(default = "default_max_concurrent_sources")]
    pub max_concurrent_sources: usize,

    /// Whether to automatically start analysis for new sources.
    #[serde(default)]
    pub auto_start: bool,

    /// Source IDs to auto-start analysis for (if auto_start is true).
    #[serde(default)]
    pub auto_start_sources: Vec<String>,
}

fn default_live_enabled() -> bool {
    true
}

fn default_sample_interval_ms() -> u64 {
    1000
}

fn default_live_prompt() -> String {
    "Describe what you see in this frame. Focus on any people, vehicles, or notable activities."
        .to_string()
}

fn default_max_concurrent_sources() -> usize {
    4
}

impl Default for LiveStreamConfig {
    fn default() -> Self {
        Self {
            enabled: default_live_enabled(),
            sample_interval_ms: default_sample_interval_ms(),
            default_prompt: default_live_prompt(),
            max_concurrent_sources: default_max_concurrent_sources(),
            auto_start: false,
            auto_start_sources: Vec::new(),
        }
    }
}

/// Batch processing configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchConfig {
    /// Maximum concurrent batch jobs.
    #[serde(default = "default_max_concurrent_jobs")]
    pub max_concurrent_jobs: usize,

    /// Default sample interval in frames.
    #[serde(default = "default_sample_interval_frames")]
    pub default_sample_interval_frames: u32,

    /// Temporary directory for frame extraction.
    #[serde(default = "default_temp_dir")]
    pub temp_dir: PathBuf,

    /// Maximum frames to process per video (0 = unlimited).
    #[serde(default)]
    pub max_frames_per_video: u32,

    /// Whether to generate video summaries.
    #[serde(default = "default_generate_summaries")]
    pub generate_summaries: bool,
}

fn default_max_concurrent_jobs() -> usize {
    2
}

fn default_sample_interval_frames() -> u32 {
    30
}

fn default_temp_dir() -> PathBuf {
    std::env::temp_dir().join("yama-vlm")
}

fn default_generate_summaries() -> bool {
    true
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: default_max_concurrent_jobs(),
            default_sample_interval_frames: default_sample_interval_frames(),
            temp_dir: default_temp_dir(),
            max_frames_per_video: 0,
            generate_summaries: default_generate_summaries(),
        }
    }
}

impl Default for VlmConfig {
    fn default() -> Self {
        Self {
            event_bus_socket: default_event_bus_socket(),
            event_bus_url: default_event_bus_url(),
            model: ModelConfig::default(),
            live_stream: LiveStreamConfig::default(),
            batch: BatchConfig::default(),
        }
    }
}

impl VlmConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from environment and optional file.
    ///
    /// # Errors
    ///
    /// Returns an error if an explicitly specified config file cannot be loaded.
    pub fn from_env() -> anyhow::Result<Self> {
        // Check for config file path in environment
        if let Ok(config_path) = std::env::var("VLM_CONFIG") {
            return Self::load(std::path::Path::new(&config_path));
        }

        // Check for config directory
        if let Ok(config_dir) = std::env::var("CONFIG_PATH") {
            let config_file = std::path::PathBuf::from(config_dir).join("vlm.toml");
            if config_file.exists() {
                return Self::load(&config_file);
            }
        }

        // Check default locations
        let default_paths = ["config/vlm.toml", "/etc/yama/vlm.toml"];
        for path in default_paths {
            let config_path = std::path::Path::new(path);
            if config_path.exists() {
                return Self::load(config_path);
            }
        }

        // Fall back to defaults
        Ok(Self::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VlmConfig::default();
        assert!(config.live_stream.enabled);
        assert_eq!(config.live_stream.sample_interval_ms, 1000);
        assert_eq!(config.model.isq, "Q4K");
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
[model]
model_id = "Qwen/Qwen2.5-VL-3B-Instruct"
isq = "Q8_0"

[live_stream]
enabled = true
sample_interval_ms = 500

[batch]
max_concurrent_jobs = 4
"#;

        let config: VlmConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.model.model_id, "Qwen/Qwen2.5-VL-3B-Instruct");
        assert_eq!(config.model.isq, "Q8_0");
        assert_eq!(config.live_stream.sample_interval_ms, 500);
        assert_eq!(config.batch.max_concurrent_jobs, 4);
    }
}
