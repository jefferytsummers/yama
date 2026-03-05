//! Metal inference engine implementation for Apple Silicon.
//!
//! This module implements the platform `InferenceEngine` trait using
//! TTRA or llama.cpp with Metal acceleration.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use yama_platform_traits::{PlatformError, PlatformResult};
use yama_platform_traits::inference::{
    DataType, InferenceCapabilities, InferenceEngine, InferenceMemoryUsage,
    InferenceRequest, InferenceResponse, ModelInfo, Tensor, TensorShape, TensorSpec,
};

/// Metal inference engine using TTRA backend.
pub struct MetalInferenceEngine {
    /// Engine capabilities.
    capabilities: InferenceCapabilities,
    /// Loaded models.
    models: HashMap<String, ModelInfo>,
    /// TTRA HTTP client.
    client: Client,
    /// TTRA server URL.
    ttra_url: String,
    /// Default model.
    default_model: String,
    /// Memory tracking.
    memory_usage: InferenceMemoryUsage,
}

impl MetalInferenceEngine {
    /// Create a new Metal inference engine.
    pub fn new(ttra_url: &str, default_model: &str) -> Result<Self> {
        info!("Initializing Metal inference engine via TTRA");

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        let capabilities = InferenceCapabilities {
            name: "Metal (TTRA)".to_string(),
            backend: "Metal".to_string(),
            device: Self::detect_metal_device(),
            supported_dtypes: vec![
                DataType::Float32,
                DataType::Float16,
                DataType::BFloat16,
                DataType::Int8,
            ],
            supported_formats: vec![
                "gguf".to_string(),
                "safetensors".to_string(),
                "mlx".to_string(),
            ],
            max_batch_size: 32,
            supports_async: true,
            available_memory: Self::query_gpu_memory(),
            compute_tflops: Self::estimate_tflops(),
        };

        info!(
            "Metal inference initialized: device={}, memory={}GB, compute={:.1} TFLOPS",
            capabilities.device,
            capabilities.available_memory / (1024 * 1024 * 1024),
            capabilities.compute_tflops
        );

        Ok(Self {
            capabilities,
            models: HashMap::new(),
            client,
            ttra_url: ttra_url.to_string(),
            default_model: default_model.to_string(),
            memory_usage: InferenceMemoryUsage::default(),
        })
    }

    /// Detect Metal device name.
    fn detect_metal_device() -> String {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| {
                    if s.contains("Apple M") {
                        s.trim().to_string()
                    } else {
                        "Apple Silicon".to_string()
                    }
                })
                .unwrap_or_else(|| "Apple Silicon".to_string())
        }
        #[cfg(not(target_os = "macos"))]
        {
            "Apple Silicon".to_string()
        }
    }

    /// Query available GPU memory.
    fn query_gpu_memory() -> u64 {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            // On Apple Silicon, GPU uses unified memory
            Command::new("sysctl")
                .args(["-n", "hw.memsize"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|mem| mem / 2) // Assume half for GPU
                .unwrap_or(16 * 1024 * 1024 * 1024)
        }
        #[cfg(not(target_os = "macos"))]
        {
            16 * 1024 * 1024 * 1024
        }
    }

    /// Estimate compute TFLOPS.
    fn estimate_tflops() -> f32 {
        // Rough estimates for Apple Silicon chips
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let brand = Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_default();

            if brand.contains("M3 Max") {
                14.2 // M3 Max ~14.2 TFLOPS
            } else if brand.contains("M3 Pro") {
                7.0
            } else if brand.contains("M3") {
                4.0
            } else if brand.contains("M2 Max") {
                13.6
            } else if brand.contains("M2 Pro") {
                6.8
            } else if brand.contains("M2") {
                3.6
            } else if brand.contains("M1 Max") {
                10.4
            } else if brand.contains("M1 Pro") {
                5.2
            } else if brand.contains("M1") {
                2.6
            } else {
                5.0 // Default estimate
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            5.0
        }
    }

    /// Make chat completion request to TTRA.
    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<serde_json::Value>,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String> {
        let url = format!("{}/v1/chat/completions", self.ttra_url);

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
            "stream": false,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send request to TTRA")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("TTRA API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(content)
    }
}

#[async_trait]
impl InferenceEngine for MetalInferenceEngine {
    fn capabilities(&self) -> &InferenceCapabilities {
        &self.capabilities
    }

    async fn load_model(&mut self, path: &Path, name: Option<&str>) -> PlatformResult<ModelInfo> {
        let model_name = name
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("model")
                    .to_string()
            });

        info!("Loading model: {} from {:?}", model_name, path);

        // In a real implementation, we'd:
        // 1. Load the model file
        // 2. Initialize Metal compute pipeline
        // 3. Allocate buffers

        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("gguf")
            .to_string();

        let model_info = ModelInfo {
            name: model_name.clone(),
            model_type: "llm".to_string(),
            format,
            parameters: 3_000_000_000, // 3B example
            quantization: Some("q4_0".to_string()),
            context_length: Some(8192),
            inputs: vec![TensorSpec {
                name: "input_ids".to_string(),
                dtype: DataType::Int32,
                shape: vec![-1], // Dynamic batch
            }],
            outputs: vec![TensorSpec {
                name: "logits".to_string(),
                dtype: DataType::Float32,
                shape: vec![-1, 32000], // vocab size
            }],
            memory_required: 4 * 1024 * 1024 * 1024, // 4GB for Q4
        };

        self.models.insert(model_name.clone(), model_info.clone());
        self.memory_usage.model_bytes += model_info.memory_required;
        self.memory_usage.models_loaded += 1;

        Ok(model_info)
    }

    async fn load_model_bytes(
        &mut self,
        data: &[u8],
        format: &str,
        name: &str,
    ) -> PlatformResult<ModelInfo> {
        info!("Loading model from bytes: {} ({} format)", name, format);

        let model_info = ModelInfo {
            name: name.to_string(),
            model_type: "llm".to_string(),
            format: format.to_string(),
            parameters: 0, // Unknown
            quantization: None,
            context_length: Some(4096),
            inputs: vec![],
            outputs: vec![],
            memory_required: data.len() as u64,
        };

        self.models.insert(name.to_string(), model_info.clone());
        self.memory_usage.model_bytes += model_info.memory_required;
        self.memory_usage.models_loaded += 1;

        Ok(model_info)
    }

    fn unload_model(&mut self, name: &str) -> PlatformResult<()> {
        if let Some(info) = self.models.remove(name) {
            info!("Unloading model: {}", name);
            self.memory_usage.model_bytes =
                self.memory_usage.model_bytes.saturating_sub(info.memory_required);
            self.memory_usage.models_loaded =
                self.memory_usage.models_loaded.saturating_sub(1);
            Ok(())
        } else {
            Err(PlatformError::NotFound(format!("Model not found: {}", name)))
        }
    }

    fn model_info(&self, name: &str) -> Option<&ModelInfo> {
        self.models.get(name)
    }

    fn loaded_models(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }

    async fn infer(&mut self, request: InferenceRequest) -> PlatformResult<InferenceResponse> {
        let start = std::time::Instant::now();

        debug!("Running inference: request_id={}", request.id);

        // For LLM inference, we expect input as text prompt
        // In a real implementation, this would run the model on Metal

        // Extract prompt from inputs
        let prompt = request
            .inputs
            .first()
            .and_then(|t| t.as_f32())
            .map(|_| "default prompt".to_string())
            .unwrap_or_else(|| {
                // Try to get from params
                request
                    .params
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            });

        // Call TTRA for actual inference
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": prompt
        })];

        let model = request.model.clone();
        let model_name = if self.models.contains_key(&model) {
            model
        } else {
            self.default_model.clone()
        };

        let temperature = request
            .params
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7) as f32;

        let max_tokens = request
            .params
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(512) as u32;

        let result = self
            .chat_completion(&model_name, messages, temperature, max_tokens)
            .await
            .map_err(|e| PlatformError::InferenceError(e.to_string()))?;

        let inference_time = start.elapsed();

        // Create output tensor with result
        let output_bytes: Vec<u8> = result.bytes().collect();
        let output = Tensor {
            name: "output".to_string(),
            dtype: DataType::Uint8,
            shape: TensorShape::new(vec![output_bytes.len()]),
            data: output_bytes,
        };

        Ok(InferenceResponse {
            id: request.id,
            outputs: vec![output],
            inference_time_us: inference_time.as_micros() as u64,
        })
    }

    fn memory_usage(&self) -> InferenceMemoryUsage {
        InferenceMemoryUsage {
            total_bytes: self.capabilities.available_memory,
            model_bytes: self.memory_usage.model_bytes,
            scratch_bytes: self.memory_usage.scratch_bytes,
            models_loaded: self.memory_usage.models_loaded,
        }
    }

    async fn warmup(&mut self, model: &str) -> PlatformResult<()> {
        info!("Warming up model: {}", model);

        // Run a small inference to warm up the model
        let warmup_request = InferenceRequest {
            id: "warmup".to_string(),
            model: model.to_string(),
            inputs: vec![],
            params: {
                let mut p = HashMap::new();
                p.insert("prompt".to_string(), serde_json::json!("Hello"));
                p.insert("max_tokens".to_string(), serde_json::json!(1));
                p
            },
        };

        let _ = self.infer(warmup_request).await?;
        info!("Model warmup complete: {}", model);

        Ok(())
    }
}

/// Create the platform-appropriate inference engine.
pub fn create_inference_engine(ttra_url: &str, default_model: &str) -> Result<Box<dyn InferenceEngine>> {
    Ok(Box::new(MetalInferenceEngine::new(ttra_url, default_model)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_capabilities() {
        let engine = MetalInferenceEngine::new("http://localhost:8080", "test").unwrap();
        let caps = engine.capabilities();

        assert_eq!(caps.backend, "Metal");
        assert!(caps.supports_async);
        assert!(caps.supported_formats.contains(&"gguf".to_string()));
    }
}
