---
name: inference
description: ML inference with Metal MPS, CUDA, TensorRT, and llama.cpp backends
compatibility:
  - platform/apple/inference/
  - platform/jetson/inference/
  - shared/platform-traits/src/inference.rs
---

# ML Inference

Hardware-accelerated ML inference using platform-specific backends.

## Backends

| Platform | Primary | Secondary | Models |
|----------|---------|-----------|--------|
| Apple | Metal MPS | TTRA | GGUF, MLX, SafeTensors |
| Jetson | TensorRT | CUDA | GGUF, TensorRT engines |

## InferenceEngine Trait

```rust
#[async_trait]
pub trait InferenceEngine: Send + Sync {
    fn capabilities(&self) -> &InferenceCapabilities;
    async fn load_model(&mut self, path: &Path, name: Option<&str>) -> PlatformResult<ModelInfo>;
    async fn load_model_bytes(&mut self, data: &[u8], format: &str, name: &str) -> PlatformResult<ModelInfo>;
    fn unload_model(&mut self, name: &str) -> PlatformResult<()>;
    fn model_info(&self, name: &str) -> Option<&ModelInfo>;
    fn loaded_models(&self) -> Vec<String>;
    async fn infer(&mut self, request: InferenceRequest) -> PlatformResult<InferenceResponse>;
    async fn infer_batch(&mut self, requests: Vec<InferenceRequest>) -> PlatformResult<Vec<InferenceResponse>>;
    fn memory_usage(&self) -> InferenceMemoryUsage;
    async fn warmup(&mut self, model: &str) -> PlatformResult<()>;
}
```

## Key Types

### DataType

```rust
pub enum DataType {
    Float32,   // Full precision
    Float16,   // Half precision
    BFloat16,  // Brain float
    Int32,
    Int64,
    Int8,      // Quantized
    Uint8,
    Bool,
}
```

### Tensor

```rust
pub struct Tensor {
    pub name: String,
    pub dtype: DataType,
    pub shape: TensorShape,
    pub data: Vec<u8>,  // Raw bytes in native order
}

impl Tensor {
    pub fn from_f32(name: impl Into<String>, shape: impl Into<Vec<usize>>, data: &[f32]) -> Self;
    pub fn as_f32(&self) -> Option<Vec<f32>>;
}
```

### InferenceRequest

```rust
pub struct InferenceRequest {
    pub id: String,
    pub model: String,
    pub inputs: Vec<Tensor>,
    pub params: HashMap<String, serde_json::Value>,
}

// Common params:
// - "prompt": String - text prompt for LLMs
// - "temperature": f64 - sampling temperature
// - "max_tokens": u64 - max output tokens
```

### ModelInfo

```rust
pub struct ModelInfo {
    pub name: String,
    pub model_type: String,      // "llm", "vision", "embedding"
    pub format: String,          // "gguf", "safetensors", "onnx"
    pub parameters: u64,         // Parameter count
    pub quantization: Option<String>,  // "q4_0", "q8_0", "f16"
    pub context_length: Option<u32>,
    pub inputs: Vec<TensorSpec>,
    pub outputs: Vec<TensorSpec>,
    pub memory_required: u64,
}
```

## Key Files

- `platform/apple/inference/src/engine.rs` - Metal/TTRA implementation
- `platform/jetson/inference/src/engine.rs` - CUDA/TensorRT implementation
- `shared/platform-traits/src/inference.rs` - InferenceEngine trait

## Apple Implementation

Uses TTRA (local LLM server) with Metal acceleration:

```rust
pub struct MetalInferenceEngine {
    capabilities: InferenceCapabilities,
    models: HashMap<String, ModelInfo>,
    client: reqwest::Client,  // HTTP client for TTRA
    ttra_url: String,
    default_model: String,
}

// TTRA API (OpenAI-compatible)
let response = client.post(&format!("{}/v1/chat/completions", ttra_url))
    .json(&json!({
        "model": model_name,
        "messages": messages,
        "temperature": temperature,
        "max_tokens": max_tokens,
        "stream": false,
    }))
    .send()
    .await?;
```

## Jetson Implementation

Uses TensorRT for optimized inference:

```rust
pub struct CudaInferenceEngine {
    capabilities: InferenceCapabilities,
    models: HashMap<String, TensorRTModel>,
    cuda_device: i32,
}

// TensorRT engine loading
let engine = tensorrt::Engine::load_from_file(path)?;
```

## Memory Management

```rust
pub struct InferenceMemoryUsage {
    pub total_bytes: u64,      // Available GPU memory
    pub model_bytes: u64,      // Memory for model weights
    pub scratch_bytes: u64,    // Activation/scratch memory
    pub models_loaded: u32,
}
```

## Model Formats

### GGUF (Recommended for LLMs)

- Quantized models (q4_0, q8_0)
- Single file, portable
- llama.cpp compatible

### SafeTensors

- Full precision or BFloat16
- Multi-file sharded models
- Used by HuggingFace

### TensorRT (Jetson)

- Pre-compiled engine files
- Platform-specific optimization
- Fastest inference

## Quantization

| Type | Bits | Memory | Quality | Use Case |
|------|------|--------|---------|----------|
| q4_0 | 4 | Lowest | Good | Production |
| q8_0 | 8 | Medium | Better | High quality |
| f16 | 16 | High | Best | Development |

## Usage Pattern

```rust
// Initialize engine
let engine = create_inference_engine(ttra_url, default_model)?;

// Load model
let model_info = engine.load_model(Path::new("model.gguf"), Some("my-model")).await?;

// Warmup
engine.warmup("my-model").await?;

// Run inference
let request = InferenceRequest {
    id: "req-1".to_string(),
    model: "my-model".to_string(),
    inputs: vec![],
    params: {
        let mut p = HashMap::new();
        p.insert("prompt".to_string(), json!("Hello, world!"));
        p.insert("max_tokens".to_string(), json!(256));
        p
    },
};

let response = engine.infer(request).await?;
```
