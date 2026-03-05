//! Inference engine trait for ML model execution.
//!
//! This trait abstracts over different ML backends:
//! - Metal Performance Shaders / Core ML (Apple)
//! - TensorRT / CUDA (NVIDIA)
//! - ONNX Runtime (cross-platform)
//! - CPU fallback

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::PlatformResult;

/// Tensor data type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DataType {
    /// 32-bit floating point.
    Float32,
    /// 16-bit floating point.
    Float16,
    /// Brain floating point (bfloat16).
    BFloat16,
    /// 32-bit signed integer.
    Int32,
    /// 64-bit signed integer.
    Int64,
    /// 8-bit signed integer.
    Int8,
    /// 8-bit unsigned integer.
    Uint8,
    /// Boolean.
    Bool,
}

impl DataType {
    /// Get size in bytes.
    #[must_use]
    pub const fn size_bytes(&self) -> usize {
        match self {
            Self::Float32 | Self::Int32 => 4,
            Self::Float16 | Self::BFloat16 => 2,
            Self::Int64 => 8,
            Self::Int8 | Self::Uint8 | Self::Bool => 1,
        }
    }
}

/// Tensor shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TensorShape {
    /// Dimensions.
    pub dims: Vec<usize>,
}

impl TensorShape {
    /// Create a new shape.
    #[must_use]
    pub fn new(dims: impl Into<Vec<usize>>) -> Self {
        Self { dims: dims.into() }
    }

    /// Get total number of elements.
    #[must_use]
    pub fn num_elements(&self) -> usize {
        self.dims.iter().product()
    }

    /// Get rank (number of dimensions).
    #[must_use]
    pub fn rank(&self) -> usize {
        self.dims.len()
    }

    /// Get size in bytes for a given data type.
    #[must_use]
    pub fn size_bytes(&self, dtype: DataType) -> usize {
        self.num_elements() * dtype.size_bytes()
    }
}

/// A tensor (multi-dimensional array).
#[derive(Debug, Clone)]
pub struct Tensor {
    /// Tensor name.
    pub name: String,
    /// Data type.
    pub dtype: DataType,
    /// Shape.
    pub shape: TensorShape,
    /// Raw data (in native byte order).
    pub data: Vec<u8>,
}

impl Tensor {
    /// Create a new float32 tensor.
    #[must_use]
    pub fn from_f32(name: impl Into<String>, shape: impl Into<Vec<usize>>, data: &[f32]) -> Self {
        let shape = TensorShape::new(shape);
        assert_eq!(shape.num_elements(), data.len());

        let data: Vec<u8> = data.iter().flat_map(|f| f.to_ne_bytes()).collect();

        Self {
            name: name.into(),
            dtype: DataType::Float32,
            shape,
            data,
        }
    }

    /// Get data as f32 slice.
    #[must_use]
    pub fn as_f32(&self) -> Option<Vec<f32>> {
        if self.dtype != DataType::Float32 {
            return None;
        }

        Some(
            self.data
                .chunks_exact(4)
                .map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
        )
    }
}

/// Model information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name.
    pub name: String,
    /// Model type (e.g., "llm", "vision", "embedding").
    pub model_type: String,
    /// Model format (e.g., "gguf", "safetensors", "onnx").
    pub format: String,
    /// Number of parameters.
    pub parameters: u64,
    /// Quantization (e.g., "f16", "q4_0", "q8_0").
    pub quantization: Option<String>,
    /// Context length (for LLMs).
    pub context_length: Option<u32>,
    /// Input specifications.
    pub inputs: Vec<TensorSpec>,
    /// Output specifications.
    pub outputs: Vec<TensorSpec>,
    /// Memory required (bytes).
    pub memory_required: u64,
}

/// Tensor specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorSpec {
    /// Tensor name.
    pub name: String,
    /// Data type.
    pub dtype: DataType,
    /// Shape (-1 for dynamic dimensions).
    pub shape: Vec<i64>,
}

/// Inference request.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    /// Request ID.
    pub id: String,
    /// Model to use.
    pub model: String,
    /// Input tensors.
    pub inputs: Vec<Tensor>,
    /// Optional parameters.
    pub params: HashMap<String, serde_json::Value>,
}

/// Inference response.
#[derive(Debug, Clone)]
pub struct InferenceResponse {
    /// Request ID.
    pub id: String,
    /// Output tensors.
    pub outputs: Vec<Tensor>,
    /// Inference time (microseconds).
    pub inference_time_us: u64,
}

/// Inference engine capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceCapabilities {
    /// Engine name.
    pub name: String,
    /// Backend (Metal, CUDA, CPU, etc.).
    pub backend: String,
    /// Device name.
    pub device: String,
    /// Supported data types.
    pub supported_dtypes: Vec<DataType>,
    /// Supported model formats.
    pub supported_formats: Vec<String>,
    /// Maximum batch size.
    pub max_batch_size: u32,
    /// Whether async inference is supported.
    pub supports_async: bool,
    /// Available memory (bytes).
    pub available_memory: u64,
    /// Compute capability (TFLOPS, 0 if unknown).
    pub compute_tflops: f32,
}

/// Inference engine abstraction.
///
/// This trait defines the interface for platform-specific ML inference engines.
/// Implementations handle model loading, inference execution, and memory management.
#[async_trait]
pub trait InferenceEngine: Send + Sync {
    /// Get engine capabilities.
    fn capabilities(&self) -> &InferenceCapabilities;

    /// Load a model from file.
    async fn load_model(&mut self, path: &Path, name: Option<&str>) -> PlatformResult<ModelInfo>;

    /// Load a model from bytes.
    async fn load_model_bytes(
        &mut self,
        data: &[u8],
        format: &str,
        name: &str,
    ) -> PlatformResult<ModelInfo>;

    /// Unload a model.
    fn unload_model(&mut self, name: &str) -> PlatformResult<()>;

    /// Get info about a loaded model.
    fn model_info(&self, name: &str) -> Option<&ModelInfo>;

    /// List loaded models.
    fn loaded_models(&self) -> Vec<String>;

    /// Run inference.
    async fn infer(&mut self, request: InferenceRequest) -> PlatformResult<InferenceResponse>;

    /// Run inference on multiple requests (batched).
    async fn infer_batch(
        &mut self,
        requests: Vec<InferenceRequest>,
    ) -> PlatformResult<Vec<InferenceResponse>> {
        let mut results = Vec::with_capacity(requests.len());
        for request in requests {
            results.push(self.infer(request).await?);
        }
        Ok(results)
    }

    /// Get memory usage.
    fn memory_usage(&self) -> InferenceMemoryUsage;

    /// Warmup a model (run dummy inference to initialize).
    async fn warmup(&mut self, model: &str) -> PlatformResult<()>;
}

/// Inference memory usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InferenceMemoryUsage {
    /// Total memory available.
    pub total_bytes: u64,
    /// Memory used by models.
    pub model_bytes: u64,
    /// Memory used for activations/scratch.
    pub scratch_bytes: u64,
    /// Number of models loaded.
    pub models_loaded: u32,
}

impl InferenceMemoryUsage {
    /// Get free memory.
    #[must_use]
    pub const fn free_bytes(&self) -> u64 {
        self.total_bytes
            .saturating_sub(self.model_bytes)
            .saturating_sub(self.scratch_bytes)
    }
}
