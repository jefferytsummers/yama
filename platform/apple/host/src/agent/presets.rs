//! Agent presets for different analysis modes.
//!
//! Presets define the behavior of the agent including:
//! - System prompt
//! - Available tools
//! - Model preferences
//! - Configuration options

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Preset identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PresetId(pub String);

impl PresetId {
    /// Video analyst preset.
    pub const VIDEO_ANALYST: &'static str = "video-analyst";
    /// Document reporter preset.
    pub const DOCUMENT_REPORTER: &'static str = "document-reporter";
    /// Quick search preset.
    pub const QUICK_SEARCH: &'static str = "quick-search";
    /// Custom preset.
    pub const CUSTOM: &'static str = "custom";

    /// Create a new preset ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PresetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for PresetId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Tool configuration within a preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Tool name/identifier.
    pub name: String,
    /// Whether the tool is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Tool-specific configuration.
    #[serde(default)]
    pub config: HashMap<String, toml::Value>,
}

fn default_true() -> bool {
    true
}

/// Model preferences for the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferences {
    /// Preferred model ID (e.g., "gpt-4-vision", "claude-3-opus").
    #[serde(default)]
    pub preferred_model: Option<String>,
    /// Fallback models in order of preference.
    #[serde(default)]
    pub fallback_models: Vec<String>,
    /// Temperature for generation (0.0 - 1.0).
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Maximum tokens in response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

impl Default for ModelPreferences {
    fn default() -> Self {
        Self {
            preferred_model: None,
            fallback_models: Vec::new(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

/// Agent preset definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreset {
    /// Unique preset identifier.
    pub id: PresetId,
    /// Display name.
    pub name: String,
    /// Short description.
    pub description: String,
    /// System prompt for the agent.
    pub system_prompt: String,
    /// Available tools.
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    /// Model preferences.
    #[serde(default)]
    pub model: ModelPreferences,
    /// Whether this is a built-in preset.
    #[serde(default)]
    pub builtin: bool,
    /// Icon name for UI display.
    #[serde(default)]
    pub icon: Option<String>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl AgentPreset {
    /// Create a new custom preset.
    pub fn custom(name: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        Self {
            id: PresetId::new(PresetId::CUSTOM),
            name: name.into(),
            description: "Custom agent preset".to_string(),
            system_prompt: system_prompt.into(),
            tools: Vec::new(),
            model: ModelPreferences::default(),
            builtin: false,
            icon: None,
            tags: vec!["custom".to_string()],
        }
    }

    /// Add a tool to the preset.
    pub fn with_tool(mut self, name: impl Into<String>) -> Self {
        self.tools.push(ToolConfig {
            name: name.into(),
            enabled: true,
            config: HashMap::new(),
        });
        self
    }

    /// Set model preferences.
    pub fn with_model(mut self, model: ModelPreferences) -> Self {
        self.model = model;
        self
    }

    /// Check if a tool is enabled.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name == name && t.enabled)
    }

    /// Get enabled tool names.
    pub fn enabled_tools(&self) -> Vec<&str> {
        self.tools
            .iter()
            .filter(|t| t.enabled)
            .map(|t| t.name.as_str())
            .collect()
    }
}

/// Registry of available presets.
#[derive(Debug, Clone)]
pub struct PresetRegistry {
    presets: HashMap<PresetId, AgentPreset>,
}

impl PresetRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            presets: HashMap::new(),
        }
    }

    /// Load built-in presets.
    pub fn load_builtin() -> Result<Self> {
        let mut registry = Self::new();

        // Load embedded preset definitions
        registry.register(Self::video_analyst_preset());
        registry.register(Self::document_reporter_preset());
        registry.register(Self::quick_search_preset());
        registry.register(Self::custom_preset());

        info!("Loaded {} built-in presets", registry.presets.len());
        Ok(registry)
    }

    /// Load presets from a directory.
    pub fn load_from_directory(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut registry = Self::load_builtin()?;

        if path.exists() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let file_path = entry.path();

                if file_path.extension().map_or(false, |e| e == "toml") {
                    match Self::load_preset_file(&file_path) {
                        Ok(preset) => {
                            info!("Loaded preset from {:?}: {}", file_path, preset.id);
                            registry.register(preset);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load preset {:?}: {}", file_path, e);
                        }
                    }
                }
            }
        }

        Ok(registry)
    }

    /// Load a preset from a TOML file.
    fn load_preset_file(path: &Path) -> Result<AgentPreset> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read preset file: {:?}", path))?;
        let preset: AgentPreset = toml::from_str(&content)
            .with_context(|| format!("Failed to parse preset file: {:?}", path))?;
        Ok(preset)
    }

    /// Register a preset.
    pub fn register(&mut self, preset: AgentPreset) {
        self.presets.insert(preset.id.clone(), preset);
    }

    /// Get a preset by ID.
    pub fn get(&self, id: &str) -> Option<&AgentPreset> {
        self.presets.get(&PresetId::new(id))
    }

    /// Get a preset by ID, returning an error if not found.
    pub fn require(&self, id: &str) -> Result<&AgentPreset> {
        self.get(id)
            .with_context(|| format!("Preset not found: {}", id))
    }

    /// List all preset IDs.
    pub fn list(&self) -> Vec<&PresetId> {
        self.presets.keys().collect()
    }

    /// List all presets.
    pub fn all(&self) -> Vec<&AgentPreset> {
        self.presets.values().collect()
    }

    /// Get built-in presets only.
    pub fn builtin(&self) -> Vec<&AgentPreset> {
        self.presets.values().filter(|p| p.builtin).collect()
    }

    /// Video Analyst preset - deep frame analysis.
    fn video_analyst_preset() -> AgentPreset {
        AgentPreset {
            id: PresetId::new(PresetId::VIDEO_ANALYST),
            name: "Video Analyst".to_string(),
            description: "Deep video analysis with frame-by-frame examination".to_string(),
            system_prompt: r#"You are an expert video analyst with deep expertise in visual content analysis. Your role is to provide thorough, detailed analysis of video content.

When analyzing videos:
- Examine frames carefully for visual details, objects, people, and actions
- Note temporal relationships and how scenes progress
- Identify key moments, transitions, and narrative elements
- Describe lighting, composition, and visual style when relevant
- Track objects and people across frames when asked

You have access to tools for:
- Searching videos by visual or text content
- Extracting specific clips or frames
- Comparing frames to identify changes
- Generating summaries of video content

Always be precise about timestamps and frame references. When uncertain, indicate your confidence level."#.to_string(),
            tools: vec![
                ToolConfig { name: "search_videos".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "extract_clip".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "compare_frames".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "summarize_video".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "get_keyframes".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "get_transcript".to_string(), enabled: true, config: HashMap::new() },
            ],
            model: ModelPreferences {
                preferred_model: Some("gpt-4-vision".to_string()),
                fallback_models: vec!["claude-3-opus".to_string(), "llava".to_string()],
                temperature: 0.3,
                max_tokens: 4096,
            },
            builtin: true,
            icon: Some("video".to_string()),
            tags: vec!["analysis".to_string(), "video".to_string(), "detailed".to_string()],
        }
    }

    /// Document Reporter preset - structured reports.
    fn document_reporter_preset() -> AgentPreset {
        AgentPreset {
            id: PresetId::new(PresetId::DOCUMENT_REPORTER),
            name: "Document Reporter".to_string(),
            description: "Generate structured reports and documentation from video content".to_string(),
            system_prompt: r#"You are a professional document creator specializing in video content documentation. Your role is to create clear, well-structured reports and documentation.

When creating reports:
- Use clear headings and sections
- Include timestamps and visual references
- Summarize key points concisely
- Format output for readability (markdown when appropriate)
- Include relevant metadata (duration, source, etc.)

Report types you can create:
- Executive summaries
- Detailed transcripts with annotations
- Scene-by-scene breakdowns
- Comparison reports
- Timeline documents

Always cite specific timestamps and frames. Structure your output for easy scanning and reference."#.to_string(),
            tools: vec![
                ToolConfig { name: "search_videos".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "summarize_video".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "get_transcript".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "extract_clip".to_string(), enabled: true, config: HashMap::new() },
            ],
            model: ModelPreferences {
                preferred_model: Some("gpt-4-vision".to_string()),
                fallback_models: vec!["claude-3-opus".to_string()],
                temperature: 0.5,
                max_tokens: 8192,
            },
            builtin: true,
            icon: Some("document".to_string()),
            tags: vec!["reports".to_string(), "documentation".to_string(), "structured".to_string()],
        }
    }

    /// Quick Search preset - fast visual search.
    fn quick_search_preset() -> AgentPreset {
        AgentPreset {
            id: PresetId::new(PresetId::QUICK_SEARCH),
            name: "Quick Search".to_string(),
            description: "Fast visual and text search across video library".to_string(),
            system_prompt: r#"You are a fast, efficient search assistant for video content. Your role is to quickly find relevant content based on user queries.

When searching:
- Prioritize speed and relevance
- Return concise results with timestamps
- Group similar results when appropriate
- Suggest related searches when helpful

Keep responses brief and actionable. Focus on finding what the user needs quickly."#.to_string(),
            tools: vec![
                ToolConfig { name: "search_videos".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "get_keyframes".to_string(), enabled: true, config: HashMap::new() },
            ],
            model: ModelPreferences {
                preferred_model: Some("gpt-4-vision".to_string()),
                fallback_models: vec!["llava".to_string()],
                temperature: 0.2,
                max_tokens: 1024,
            },
            builtin: true,
            icon: Some("search".to_string()),
            tags: vec!["search".to_string(), "fast".to_string(), "visual".to_string()],
        }
    }

    /// Custom preset - user-defined.
    fn custom_preset() -> AgentPreset {
        AgentPreset {
            id: PresetId::new(PresetId::CUSTOM),
            name: "Custom".to_string(),
            description: "User-defined agent configuration".to_string(),
            system_prompt: "You are a helpful video analysis assistant.".to_string(),
            tools: vec![
                ToolConfig { name: "search_videos".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "extract_clip".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "compare_frames".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "summarize_video".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "get_keyframes".to_string(), enabled: true, config: HashMap::new() },
                ToolConfig { name: "get_transcript".to_string(), enabled: true, config: HashMap::new() },
            ],
            model: ModelPreferences::default(),
            builtin: true,
            icon: Some("settings".to_string()),
            tags: vec!["custom".to_string()],
        }
    }
}

impl Default for PresetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_id() {
        let id = PresetId::new("test");
        assert_eq!(id.as_str(), "test");
        assert_eq!(id.to_string(), "test");
    }

    #[test]
    fn test_load_builtin() {
        let registry = PresetRegistry::load_builtin().unwrap();

        assert!(registry.get(PresetId::VIDEO_ANALYST).is_some());
        assert!(registry.get(PresetId::DOCUMENT_REPORTER).is_some());
        assert!(registry.get(PresetId::QUICK_SEARCH).is_some());
        assert!(registry.get(PresetId::CUSTOM).is_some());

        assert_eq!(registry.builtin().len(), 4);
    }

    #[test]
    fn test_video_analyst_preset() {
        let registry = PresetRegistry::load_builtin().unwrap();
        let preset = registry.require(PresetId::VIDEO_ANALYST).unwrap();

        assert_eq!(preset.name, "Video Analyst");
        assert!(preset.builtin);
        assert!(preset.has_tool("search_videos"));
        assert!(preset.has_tool("extract_clip"));
        assert!(!preset.has_tool("nonexistent_tool"));
    }

    #[test]
    fn test_custom_preset_builder() {
        let preset = AgentPreset::custom("My Agent", "You are a test agent.")
            .with_tool("search_videos")
            .with_tool("extract_clip");

        assert_eq!(preset.name, "My Agent");
        assert!(preset.has_tool("search_videos"));
        assert!(preset.has_tool("extract_clip"));
        assert_eq!(preset.enabled_tools().len(), 2);
    }

    #[test]
    fn test_model_preferences_defaults() {
        let prefs = ModelPreferences::default();
        assert!(prefs.preferred_model.is_none());
        assert_eq!(prefs.temperature, 0.7);
        assert_eq!(prefs.max_tokens, 4096);
    }

    #[test]
    fn test_preset_serialization() {
        let preset = AgentPreset::custom("Test", "System prompt")
            .with_tool("search_videos");

        let toml_str = toml::to_string(&preset).unwrap();
        assert!(toml_str.contains("Test"));

        let parsed: AgentPreset = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.name, "Test");
    }
}
