//! Triton VLM Backend
//!
//! Implements the VlmBackend trait for NVIDIA Triton Inference Server
//! with TensorRT-LLM backend for Vision Language Models.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  TritonVlmBackend                                               │
//! │  ┌───────────────────────────────────────────────────────────┐  │
//! │  │  Image Preprocessor (CUDA)                                │  │
//! │  │  - Resize, normalize, format conversion                   │  │
//! │  └───────────────────────────────────────────────────────────┘  │
//! │                              │                                   │
//! │                              ▼                                   │
//! │  ┌───────────────────────────────────────────────────────────┐  │
//! │  │  Triton Client (gRPC/HTTP)                                │  │
//! │  │  - Request formatting                                     │  │
//! │  │  - System shared memory (optional)                        │  │
//! │  └───────────────────────────────────────────────────────────┘  │
//! │                              │                                   │
//! │                              ▼                                   │
//! │  ┌───────────────────────────────────────────────────────────┐  │
//! │  │  Triton Server                                            │  │
//! │  │  - TensorRT-LLM engine                                    │  │
//! │  │  - Qwen2.5-VL model                                       │  │
//! │  └───────────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use yama_platform_traits::{
    PlatformError, PlatformResult, VlmBackend, VlmBatchRequest, VlmBatchResult, VlmCapabilities,
    VlmImageSource, VlmPixelFormat, VlmRequest, VlmResponse,
};

use super::triton_client::{InferInput, InferRequest, TritonClient, TritonClientConfig};

/// Triton VLM backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TritonVlmConfig {
    /// Triton server URL.
    #[serde(default = "default_server_url")]
    pub server_url: String,
    /// Model name in Triton.
    #[serde(default = "default_model_name")]
    pub model_name: String,
    /// Whether to use system shared memory.
    #[serde(default)]
    pub use_shared_memory: bool,
    /// Maximum concurrent requests.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: usize,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// Input image width for preprocessing.
    #[serde(default = "default_input_width")]
    pub input_width: u32,
    /// Input image height for preprocessing.
    #[serde(default = "default_input_height")]
    pub input_height: u32,
}

fn default_server_url() -> String {
    std::env::var("TRITON_SERVER_URL").unwrap_or_else(|_| "localhost:8001".to_string())
}

fn default_model_name() -> String {
    std::env::var("TRITON_MODEL_NAME").unwrap_or_else(|_| "qwen2_5_vl".to_string())
}

fn default_max_concurrent() -> usize {
    4
}

fn default_timeout_secs() -> u64 {
    60
}

fn default_input_width() -> u32 {
    448
}

fn default_input_height() -> u32 {
    448
}

impl Default for TritonVlmConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            model_name: default_model_name(),
            use_shared_memory: false,
            max_concurrent_requests: default_max_concurrent(),
            timeout_secs: default_timeout_secs(),
            input_width: default_input_width(),
            input_height: default_input_height(),
        }
    }
}

/// Triton-based VLM backend for Jetson.
///
/// Uses Triton Inference Server with TensorRT-LLM for optimized VLM inference.
pub struct TritonVlmBackend {
    /// Triton client.
    client: Arc<TritonClient>,
    /// Backend configuration.
    config: TritonVlmConfig,
    /// Backend capabilities.
    capabilities: VlmCapabilities,
    /// Whether the backend is ready.
    ready: Arc<RwLock<bool>>,
    /// Concurrent request semaphore.
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl TritonVlmBackend {
    /// Create a new Triton VLM backend.
    pub async fn new(config: TritonVlmConfig) -> Result<Self> {
        let client_config = TritonClientConfig {
            server_url: config.server_url.clone(),
            timeout: std::time::Duration::from_secs(config.timeout_secs),
            use_shared_memory: config.use_shared_memory,
            ..Default::default()
        };

        let client = TritonClient::new(client_config)
            .await
            .context("Failed to create Triton client")?;

        let capabilities = VlmCapabilities::triton(&config.model_name);

        let backend = Self {
            client: Arc::new(client),
            config,
            capabilities,
            ready: Arc::new(RwLock::new(false)),
            semaphore: Arc::new(tokio::sync::Semaphore::new(default_max_concurrent())),
        };

        // Check initial readiness
        if backend.client.is_ready().await {
            if backend.client.is_model_ready(&backend.config.model_name).await {
                *backend.ready.write().await = true;
                info!("Triton backend ready with model {}", backend.config.model_name);
            } else {
                warn!(
                    "Triton server ready but model {} not loaded",
                    backend.config.model_name
                );
            }
        } else {
            warn!("Triton server not ready - inference requests will fail until server starts");
        }

        Ok(backend)
    }

    /// Preprocess image for VLM input.
    ///
    /// Converts image to the format expected by the VLM model.
    fn preprocess_image(&self, source: &VlmImageSource) -> Result<Vec<u8>> {
        match source {
            VlmImageSource::Jpeg(data) => {
                // Decode JPEG and resize
                let img = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
                    .context("Failed to decode JPEG")?;

                // Resize to model input size
                let resized = img.resize_exact(
                    self.config.input_width,
                    self.config.input_height,
                    image::imageops::FilterType::Lanczos3,
                );

                // Convert to RGB bytes
                Ok(resized.to_rgb8().into_raw())
            }
            VlmImageSource::Png(data) => {
                let img = image::load_from_memory_with_format(data, image::ImageFormat::Png)
                    .context("Failed to decode PNG")?;

                let resized = img.resize_exact(
                    self.config.input_width,
                    self.config.input_height,
                    image::imageops::FilterType::Lanczos3,
                );

                Ok(resized.to_rgb8().into_raw())
            }
            VlmImageSource::RawRgb { width, height, data } => {
                // Create image from raw RGB
                let img = image::RgbImage::from_raw(*width, *height, data.clone())
                    .context("Invalid RGB dimensions")?;

                if *width == self.config.input_width && *height == self.config.input_height {
                    Ok(data.clone())
                } else {
                    let resized = image::imageops::resize(
                        &img,
                        self.config.input_width,
                        self.config.input_height,
                        image::imageops::FilterType::Lanczos3,
                    );
                    Ok(resized.into_raw())
                }
            }
            VlmImageSource::RawRgba { width, height, data } => {
                let img = image::RgbaImage::from_raw(*width, *height, data.clone())
                    .context("Invalid RGBA dimensions")?;

                let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();

                if *width == self.config.input_width && *height == self.config.input_height {
                    Ok(rgb.into_raw())
                } else {
                    let resized = image::imageops::resize(
                        &rgb,
                        self.config.input_width,
                        self.config.input_height,
                        image::imageops::FilterType::Lanczos3,
                    );
                    Ok(resized.into_raw())
                }
            }
            VlmImageSource::Nv12 { width, height, data } => {
                // Convert NV12 to RGB
                // This is a simplified implementation - real CUDA conversion would be faster
                let rgb = self.nv12_to_rgb(data, *width, *height)?;

                // Resize if needed
                let img = image::RgbImage::from_raw(*width, *height, rgb)
                    .context("Failed to create RGB image from NV12")?;

                let resized = image::imageops::resize(
                    &img,
                    self.config.input_width,
                    self.config.input_height,
                    image::imageops::FilterType::Lanczos3,
                );

                Ok(resized.into_raw())
            }
            VlmImageSource::SharedMemory { .. } => {
                // TODO: Implement shared memory support
                anyhow::bail!("Shared memory not yet implemented")
            }
        }
    }

    /// Convert NV12 to RGB (simple CPU implementation).
    ///
    /// In production, this should use CUDA for hardware acceleration.
    fn nv12_to_rgb(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        let y_size = (width * height) as usize;
        let uv_size = y_size / 2;

        if data.len() < y_size + uv_size {
            anyhow::bail!("NV12 data too small: expected {}, got {}", y_size + uv_size, data.len());
        }

        let y_plane = &data[..y_size];
        let uv_plane = &data[y_size..];

        let mut rgb = vec![0u8; (width * height * 3) as usize];

        for row in 0..height {
            for col in 0..width {
                let y_idx = (row * width + col) as usize;
                let uv_row = row / 2;
                let uv_col = (col / 2) * 2;
                let uv_idx = (uv_row * width + uv_col) as usize;

                let y = y_plane[y_idx] as f32;
                let u = uv_plane[uv_idx] as f32 - 128.0;
                let v = uv_plane[uv_idx + 1] as f32 - 128.0;

                // YUV to RGB conversion
                let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
                let g = (y - 0.344 * u - 0.714 * v).clamp(0.0, 255.0) as u8;
                let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;

                let rgb_idx = y_idx * 3;
                rgb[rgb_idx] = r;
                rgb[rgb_idx + 1] = g;
                rgb[rgb_idx + 2] = b;
            }
        }

        Ok(rgb)
    }

    /// Build inference request for VLM model.
    fn build_infer_request(
        &self,
        request_id: &str,
        prompt: &str,
        image_data: Vec<u8>,
    ) -> InferRequest {
        // Build prompt input (as bytes/string)
        let prompt_bytes = prompt.as_bytes().to_vec();

        // Build image input (RGB tensor)
        let image_input = InferInput {
            name: "image".to_string(),
            shape: vec![
                1,
                3,
                self.config.input_height as i64,
                self.config.input_width as i64,
            ],
            datatype: "UINT8".to_string(),
            data: image_data,
        };

        let prompt_input = InferInput {
            name: "prompt".to_string(),
            shape: vec![1],
            datatype: "BYTES".to_string(),
            data: prompt_bytes,
        };

        InferRequest {
            model_name: self.config.model_name.clone(),
            model_version: String::new(), // Use latest
            request_id: request_id.to_string(),
            inputs: vec![image_input, prompt_input],
            outputs: vec!["output".to_string()],
            parameters: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl VlmBackend for TritonVlmBackend {
    fn capabilities(&self) -> &VlmCapabilities {
        &self.capabilities
    }

    async fn analyze(&self, request: VlmRequest) -> PlatformResult<VlmResponse> {
        // Acquire semaphore permit
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            PlatformError::Other(format!("Failed to acquire semaphore: {}", e))
        })?;

        let start = Instant::now();

        // Preprocess image
        let image_data = self
            .preprocess_image(&request.image)
            .map_err(|e| PlatformError::Other(format!("Image preprocessing failed: {}", e)))?;

        debug!(
            "Preprocessed image: {} bytes -> {} bytes",
            request.image.data_size(),
            image_data.len()
        );

        // Build and send inference request
        let infer_request = self.build_infer_request(&request.request_id, &request.prompt, image_data);

        let response = self
            .client
            .infer(infer_request)
            .await
            .map_err(|e| PlatformError::Other(format!("Triton inference failed: {}", e)))?;

        // Extract output text
        let output_text = if let Some(output) = response.outputs.first() {
            String::from_utf8_lossy(&output.data).to_string()
        } else {
            String::new()
        };

        let inference_time_ms = start.elapsed().as_secs_f32() * 1000.0;

        // Estimate token count
        let tokens_generated = (output_text.len() / 4) as u32;

        Ok(VlmResponse {
            request_id: request.request_id,
            analysis: output_text,
            inference_time_ms,
            tokens_generated,
            model: self.config.model_name.clone(),
        })
    }

    async fn analyze_batch(&self, request: VlmBatchRequest) -> PlatformResult<VlmBatchResult> {
        let start = Instant::now();
        let mut result = VlmBatchResult::new(&request.request_id);

        if request.parallel {
            // Process requests in parallel (up to semaphore limit)
            let futures: Vec<_> = request
                .requests
                .into_iter()
                .map(|req| {
                    let backend = self;
                    async move {
                        let req_id = req.request_id.clone();
                        match backend.analyze(req).await {
                            Ok(response) => (req_id, Ok(response)),
                            Err(e) => (req_id, Err(e)),
                        }
                    }
                })
                .collect();

            let results = futures::future::join_all(futures).await;

            for (req_id, res) in results {
                match res {
                    Ok(response) => result.add_response(response),
                    Err(e) => result.add_error(req_id, e.to_string()),
                }
            }
        } else {
            // Process requests sequentially
            for req in request.requests {
                let req_id = req.request_id.clone();
                match self.analyze(req).await {
                    Ok(response) => result.add_response(response),
                    Err(e) => result.add_error(req_id, e.to_string()),
                }
            }
        }

        result.total_time_ms = start.elapsed().as_secs_f32() * 1000.0;
        Ok(result)
    }

    async fn is_ready(&self) -> bool {
        let ready = *self.ready.read().await;
        if ready {
            return true;
        }

        // Re-check server and model status
        if self.client.is_ready().await && self.client.is_model_ready(&self.config.model_name).await
        {
            *self.ready.write().await = true;
            true
        } else {
            false
        }
    }

    async fn warmup(&self) -> PlatformResult<()> {
        info!("Warming up Triton VLM backend...");

        // Wait for server to be ready
        self.client
            .wait_ready(std::time::Duration::from_secs(30))
            .await
            .map_err(|e| PlatformError::Other(format!("Server not ready: {}", e)))?;

        // Wait for model to be ready
        self.client
            .wait_model_ready(&self.config.model_name, std::time::Duration::from_secs(60))
            .await
            .map_err(|e| PlatformError::Other(format!("Model not ready: {}", e)))?;

        *self.ready.write().await = true;

        // Run a warmup inference
        let warmup_image = vec![128u8; (self.config.input_width * self.config.input_height * 3) as usize];
        let warmup_request = VlmRequest::new(
            "warmup",
            "Describe this image briefly.",
            VlmImageSource::RawRgb {
                width: self.config.input_width,
                height: self.config.input_height,
                data: warmup_image,
            },
        );

        match self.analyze(warmup_request).await {
            Ok(_) => info!("Triton warmup inference successful"),
            Err(e) => warn!("Triton warmup inference failed (non-fatal): {}", e),
        }

        info!("Triton VLM backend warmup complete");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "triton"
    }

    async fn shutdown(&self) -> PlatformResult<()> {
        info!("Shutting down Triton VLM backend");
        *self.ready.write().await = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TritonVlmConfig::default();
        assert!(!config.server_url.is_empty());
        assert!(!config.model_name.is_empty());
    }

    #[test]
    fn test_nv12_conversion() {
        // Create a small test NV12 image
        let width = 4u32;
        let height = 4u32;
        let y_size = (width * height) as usize;
        let uv_size = y_size / 2;

        let mut nv12_data = vec![128u8; y_size + uv_size]; // Mid-gray

        let config = TritonVlmConfig::default();
        let backend = TritonVlmBackend {
            client: Arc::new(futures::executor::block_on(async {
                TritonClient::new(TritonClientConfig::default()).await.unwrap()
            })),
            config,
            capabilities: VlmCapabilities::triton("test"),
            ready: Arc::new(RwLock::new(false)),
            semaphore: Arc::new(tokio::sync::Semaphore::new(4)),
        };

        let rgb = backend.nv12_to_rgb(&nv12_data, width, height).unwrap();
        assert_eq!(rgb.len(), (width * height * 3) as usize);
    }
}
