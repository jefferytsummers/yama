//! Model registry definitions.
//!
//! This module defines the available models and their metadata.

use serde::{Deserialize, Serialize};

/// Model type categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelType {
    /// Vision-Language Model (VLM).
    Vlm,
    /// CLIP visual embeddings.
    Clip,
    /// Whisper audio transcription.
    Whisper,
    /// Text embeddings.
    TextEmbedding,
    /// Object detection.
    Detection,
    /// Custom/other.
    Custom,
}

impl ModelType {
    /// Human-readable name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Vlm => "Vision-Language Model",
            Self::Clip => "CLIP Embeddings",
            Self::Whisper => "Whisper Transcription",
            Self::TextEmbedding => "Text Embeddings",
            Self::Detection => "Object Detection",
            Self::Custom => "Custom",
        }
    }
}

/// Model source types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelSource {
    /// HuggingFace Hub.
    HuggingFace {
        repo_id: String,
        filename: Option<String>,
        revision: Option<String>,
    },
    /// Direct URL download.
    Url(String),
    /// Local file path.
    Local(String),
}

impl ModelSource {
    /// Create a HuggingFace source.
    pub fn huggingface(repo_id: impl Into<String>) -> Self {
        Self::HuggingFace {
            repo_id: repo_id.into(),
            filename: None,
            revision: None,
        }
    }

    /// Create a HuggingFace source with specific filename.
    pub fn huggingface_file(repo_id: impl Into<String>, filename: impl Into<String>) -> Self {
        Self::HuggingFace {
            repo_id: repo_id.into(),
            filename: Some(filename.into()),
            revision: None,
        }
    }
}

/// Model format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormat {
    /// GGUF quantized format.
    Gguf,
    /// SafeTensors format.
    SafeTensors,
    /// ONNX format.
    Onnx,
    /// Core ML format.
    CoreMl,
    /// PyTorch format.
    PyTorch,
}

impl ModelFormat {
    /// File extensions for this format.
    pub fn extensions(&self) -> &[&'static str] {
        match self {
            Self::Gguf => &["gguf"],
            Self::SafeTensors => &["safetensors"],
            Self::Onnx => &["onnx"],
            Self::CoreMl => &["mlmodel", "mlmodelc", "mlpackage"],
            Self::PyTorch => &["pt", "pth", "bin"],
        }
    }
}

/// Model quantization level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quantization {
    /// No quantization (full precision).
    None,
    /// 16-bit floating point.
    F16,
    /// Brain float 16.
    Bf16,
    /// 8-bit integer.
    Q8,
    /// 4-bit integer (most common for GGUF).
    Q4,
    /// 4-bit K-quants.
    Q4K,
    /// Mixed precision.
    Mixed,
}

/// Model definition in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDefinition {
    /// Unique model identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Model type.
    pub model_type: ModelType,
    /// Model format.
    pub format: ModelFormat,
    /// Quantization level.
    pub quantization: Quantization,
    /// Model source.
    pub source: ModelSource,
    /// Approximate size in bytes.
    pub size_bytes: u64,
    /// Memory required when loaded (approximate).
    pub memory_bytes: u64,
    /// Whether this model is recommended/default.
    #[serde(default)]
    pub recommended: bool,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl ModelDefinition {
    /// Create a new model definition.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        model_type: ModelType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            model_type,
            format: ModelFormat::Gguf,
            quantization: Quantization::Q4,
            source: ModelSource::Local(String::new()),
            size_bytes: 0,
            memory_bytes: 0,
            recommended: false,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set format.
    pub fn with_format(mut self, format: ModelFormat) -> Self {
        self.format = format;
        self
    }

    /// Set quantization.
    pub fn with_quantization(mut self, quant: Quantization) -> Self {
        self.quantization = quant;
        self
    }

    /// Set source.
    pub fn with_source(mut self, source: ModelSource) -> Self {
        self.source = source;
        self
    }

    /// Set size.
    pub fn with_size(mut self, size_bytes: u64, memory_bytes: u64) -> Self {
        self.size_bytes = size_bytes;
        self.memory_bytes = memory_bytes;
        self
    }

    /// Mark as recommended.
    pub fn recommended(mut self) -> Self {
        self.recommended = true;
        self
    }
}

/// Built-in model registry.
pub struct ModelRegistry {
    models: Vec<ModelDefinition>,
}

impl ModelRegistry {
    /// Create a new registry with default models.
    pub fn new() -> Self {
        let models = vec![
            // CLIP models
            ModelDefinition::new("clip-vit-base-patch32", "CLIP ViT-B/32", ModelType::Clip)
                .with_description("OpenAI CLIP model for visual embeddings (512-dim)")
                .with_format(ModelFormat::Onnx)
                .with_quantization(Quantization::F16)
                .with_source(ModelSource::huggingface_file(
                    "openai/clip-vit-base-patch32",
                    "model.onnx",
                ))
                .with_size(350_000_000, 400_000_000)
                .recommended(),

            ModelDefinition::new("clip-vit-large-patch14", "CLIP ViT-L/14", ModelType::Clip)
                .with_description("Larger CLIP model for better embeddings (768-dim)")
                .with_format(ModelFormat::Onnx)
                .with_quantization(Quantization::F16)
                .with_source(ModelSource::huggingface_file(
                    "openai/clip-vit-large-patch14",
                    "model.onnx",
                ))
                .with_size(900_000_000, 1_200_000_000),

            // Whisper models
            ModelDefinition::new("whisper-tiny", "Whisper Tiny", ModelType::Whisper)
                .with_description("Smallest Whisper model, fastest but lowest accuracy")
                .with_format(ModelFormat::Gguf)
                .with_quantization(Quantization::Q4)
                .with_source(ModelSource::huggingface_file(
                    "ggerganov/whisper.cpp",
                    "ggml-tiny.bin",
                ))
                .with_size(75_000_000, 150_000_000),

            ModelDefinition::new("whisper-base", "Whisper Base", ModelType::Whisper)
                .with_description("Base Whisper model, good balance of speed/accuracy")
                .with_format(ModelFormat::Gguf)
                .with_quantization(Quantization::Q4)
                .with_source(ModelSource::huggingface_file(
                    "ggerganov/whisper.cpp",
                    "ggml-base.bin",
                ))
                .with_size(142_000_000, 250_000_000)
                .recommended(),

            ModelDefinition::new("whisper-small", "Whisper Small", ModelType::Whisper)
                .with_description("Small Whisper model, better accuracy")
                .with_format(ModelFormat::Gguf)
                .with_quantization(Quantization::Q4)
                .with_source(ModelSource::huggingface_file(
                    "ggerganov/whisper.cpp",
                    "ggml-small.bin",
                ))
                .with_size(466_000_000, 750_000_000),

            ModelDefinition::new("whisper-medium", "Whisper Medium", ModelType::Whisper)
                .with_description("Medium Whisper model, high accuracy")
                .with_format(ModelFormat::Gguf)
                .with_quantization(Quantization::Q4)
                .with_source(ModelSource::huggingface_file(
                    "ggerganov/whisper.cpp",
                    "ggml-medium.bin",
                ))
                .with_size(1_500_000_000, 2_500_000_000),

            ModelDefinition::new("whisper-large", "Whisper Large", ModelType::Whisper)
                .with_description("Largest Whisper model, best accuracy")
                .with_format(ModelFormat::Gguf)
                .with_quantization(Quantization::Q4)
                .with_source(ModelSource::huggingface_file(
                    "ggerganov/whisper.cpp",
                    "ggml-large.bin",
                ))
                .with_size(2_900_000_000, 4_500_000_000),

            // Text embedding models
            ModelDefinition::new("all-minilm-l6-v2", "MiniLM L6 v2", ModelType::TextEmbedding)
                .with_description("Fast text embeddings (384-dim)")
                .with_format(ModelFormat::Onnx)
                .with_quantization(Quantization::F16)
                .with_source(ModelSource::huggingface_file(
                    "sentence-transformers/all-MiniLM-L6-v2",
                    "model.onnx",
                ))
                .with_size(90_000_000, 150_000_000)
                .recommended(),
        ];

        Self { models }
    }

    /// Get all model definitions.
    pub fn all(&self) -> &[ModelDefinition] {
        &self.models
    }

    /// Get models by type.
    pub fn by_type(&self, model_type: ModelType) -> Vec<&ModelDefinition> {
        self.models
            .iter()
            .filter(|m| m.model_type == model_type)
            .collect()
    }

    /// Get a model by ID.
    pub fn get(&self, id: &str) -> Option<&ModelDefinition> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Get recommended model for a type.
    pub fn recommended(&self, model_type: ModelType) -> Option<&ModelDefinition> {
        self.models
            .iter()
            .find(|m| m.model_type == model_type && m.recommended)
    }

    /// Add a custom model definition.
    pub fn add(&mut self, model: ModelDefinition) {
        self.models.push(model);
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_default() {
        let registry = ModelRegistry::new();
        assert!(!registry.all().is_empty());
    }

    #[test]
    fn test_registry_by_type() {
        let registry = ModelRegistry::new();
        let whisper_models = registry.by_type(ModelType::Whisper);
        assert!(whisper_models.len() >= 3);
    }

    #[test]
    fn test_registry_get() {
        let registry = ModelRegistry::new();
        let model = registry.get("whisper-base");
        assert!(model.is_some());
        assert_eq!(model.unwrap().model_type, ModelType::Whisper);
    }

    #[test]
    fn test_registry_recommended() {
        let registry = ModelRegistry::new();
        let clip = registry.recommended(ModelType::Clip);
        assert!(clip.is_some());
        assert!(clip.unwrap().recommended);
    }
}
