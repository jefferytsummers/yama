//! CUDA inference engine implementation for NVIDIA Jetson.
//!
//! This module implements the platform `InferenceEngine` trait using
//! TensorRT-LLM or llama.cpp with CUDA acceleration.

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

/// CUDA inference engine for Jetson.
pub struct CudaInferenceEngine {
    /// Engine capabilities.
    capabilities: InferenceCapabilities,
    /// Loaded models.
    models: HashMap<String, ModelInfo>,
    /// HTTP client for API calls.
    client: Client,
    /// Server URL (for TensorRT-LLM server or llama.cpp server).
    server_url: String,
    /// Default model.
    default_model: String,
    /// Memory tracking.
    memory_usage: InferenceMemoryUsage,
}

impl CudaInferenceEngine {
    /// Create a new CUDA inference engine.
    pub fn new(server_url: &str, default_model: &str) -> Result<Self> {
        info!("Initializing CUDA inference engine for Jetson");

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        let (device_name, gpu_memory, compute_tflops) = Self::detect_gpu();

        let capabilities = InferenceCapabilities {
            name: "CUDA (Jetson)".to_string(),
            backend: "CUDA".to_string(),
            device: device_name,
            supported_dtypes: vec![
                DataType::Float32,
                DataType::Float16,
                DataType::Int8,
            ],
            supported_formats: vec![
                "gguf".to_string(),
                "tensorrt".to_string(),
                "onnx".to_string(),
            ],
            max_batch_size: 16,
            supports_async: true,
            available_memory: gpu_memory,
            compute_tflops,
        };

        info!(
            "CUDA inference initialized: device={}, memory={}GB, compute={:.1} TFLOPS",
            capabilities.device,
            capabilities.available_memory / (1024 * 1024 * 1024),
            capabilities.compute_tflops
        );

        Ok(Self {
            capabilities,
            models: HashMap::new(),
            client,
            server_url: server_url.to_string(),
            default_model: default_model.to_string(),
            memory_usage: InferenceMemoryUsage::default(),
        })
    }

    /// Detect GPU information.
    fn detect_gpu() -> (String, u64, f32) {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            use std::process::Command;

            // Try nvidia-smi
            if let Ok(output) = Command::new("nvidia-smi")
                .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
                .output()
            {
                if let Ok(info) = String::from_utf8(output.stdout) {
                    let parts: Vec<&str> = info.trim().split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 2 {
                        let name = parts[0].to_string();
                        let memory_mb = parts[1].parse::<u64>().unwrap_or(8192);
                        let memory = memory_mb * 1024 * 1024;

                        // Estimate TFLOPS based on Jetson model
                        let tflops = if name.contains("Orin") {
                            if name.contains("AGX") {
                                275.0 / 8.0 // Orin AGX ~275 INT8 TOPS, roughly /8 for FP16
                            } else if name.contains("NX") {
                                100.0 / 8.0
                            } else {
                                70.0 / 8.0
                            }
                        } else if name.contains("Xavier") {
                            if name.contains("AGX") {
                                32.0 / 8.0
                            } else {
                                21.0 / 8.0
                            }
                        } else {
                            10.0
                        };

                        return (name, memory, tflops);
                    }
                }
            }

            // Fallback: check Jetson model
            if let Ok(model) = fs::read_to_string("/proc/device-tree/model") {
                let model = model.trim().to_string();
                if model.contains("Orin") {
                    return ("NVIDIA Jetson Orin".to_string(), 16 * 1024 * 1024 * 1024, 34.0);
                } else if model.contains("Xavier") {
                    return ("NVIDIA Jetson Xavier".to_string(), 8 * 1024 * 1024 * 1024, 4.0);
                }
            }
        }

        // Default for unknown Jetson
        ("NVIDIA Jetson".to_string(), 8 * 1024 * 1024 * 1024, 10.0)
    }

    /// Make chat completion request to inference server.
    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<serde_json::Value>,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String> {
        let url = format!("{}/v1/chat/completions", self.server_url);

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
            .context("Failed to send request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("API error: {}", response.status()));
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
impl InferenceEngine for CudaInferenceEngine {
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

        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("gguf")
            .to_string();

        let model_info = ModelInfo {
            name: model_name.clone(),
            model_type: "llm".to_string(),
            format,
            parameters: 7_000_000_000, // 7B example
            quantization: Some("q4_0".to_string()),
            context_length: Some(4096),
            inputs: vec![TensorSpec {
                name: "input_ids".to_string(),
                dtype: DataType::Int32,
                shape: vec![-1],
            }],
            outputs: vec![TensorSpec {
                name: "logits".to_string(),
                dtype: DataType::Float16,
                shape: vec![-1, 32000],
            }],
            memory_required: 8 * 1024 * 1024 * 1024, // 8GB for 7B Q4
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
            parameters: 0,
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

        debug!("Running CUDA inference: request_id={}", request.id);

        // Extract prompt from params
        let prompt = request
            .params
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

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
pub fn create_inference_engine(server_url: &str, default_model: &str) -> Result<Box<dyn InferenceEngine>> {
    Ok(Box::new(CudaInferenceEngine::new(server_url, default_model)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_capabilities() {
        let engine = CudaInferenceEngine::new("http://localhost:8080", "test").unwrap();
        let caps = engine.capabilities();

        assert_eq!(caps.backend, "CUDA");
        assert!(caps.supports_async);
        assert!(caps.supported_formats.contains(&"tensorrt".to_string()));
    }
}
