//! Flask VLM Backend
//!
//! Implements the VlmBackend trait for the Flask-based LLaVA inference server.
//! This backend communicates with the Python VLM server via HTTP REST API.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use yama_platform_traits::{
    PlatformError, PlatformResult, VlmBackend, VlmBatchRequest, VlmBatchResult, VlmCapabilities,
    VlmImageSource, VlmPixelFormat, VlmRequest, VlmResponse,
};

use super::vlm_client::{VlmClient, VlmClientConfig};

/// Flask VLM backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlaskVlmConfig {
    /// Server URL (e.g., "http://localhost:8000").
    #[serde(default = "default_server_url")]
    pub server_url: String,
    /// Maximum concurrent requests.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: usize,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// Maximum tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Default temperature for generation.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_server_url() -> String {
    std::env::var("VLM_SERVER_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

fn default_max_concurrent() -> usize {
    4
}

fn default_timeout_secs() -> u64 {
    120
}

fn default_max_tokens() -> u32 {
    512
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for FlaskVlmConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            max_concurrent_requests: default_max_concurrent(),
            timeout_secs: default_timeout_secs(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
}

/// Flask-based VLM backend for Jetson.
///
/// Uses the Flask VLM server with HuggingFace Transformers for LLaVA inference.
pub struct FlaskVlmBackend {
    /// VLM HTTP client.
    client: VlmClient,
    /// Backend configuration.
    config: FlaskVlmConfig,
    /// Backend capabilities.
    capabilities: VlmCapabilities,
    /// Whether the backend is ready.
    ready: Arc<RwLock<bool>>,
    /// Concurrent request semaphore.
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl FlaskVlmBackend {
    /// Create a new Flask VLM backend.
    pub async fn new(config: FlaskVlmConfig) -> Result<Self> {
        let client_config = VlmClientConfig {
            server_url: config.server_url.clone(),
            timeout: std::time::Duration::from_secs(config.timeout_secs),
            max_retries: 2,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        };

        let client = VlmClient::new(client_config).context("Failed to create VLM client")?;

        let capabilities = VlmCapabilities {
            name: "flask-llava".to_string(),
            model: "llava-1.5-7b-hf".to_string(),
            max_image_width: 4096,
            max_image_height: 4096,
            max_tokens: 4096,
            supported_formats: vec![
                VlmPixelFormat::Jpeg,
                VlmPixelFormat::Png,
                VlmPixelFormat::Rgb,
                VlmPixelFormat::Rgba,
                VlmPixelFormat::Nv12,
            ],
            supports_batch: false,
            supports_streaming: false,
            supports_shared_memory: false,
            accelerator: "CUDA".to_string(),
            estimated_throughput: 8.0, // ~8 tokens/second on Thor
        };

        let backend = Self {
            client,
            config,
            capabilities,
            ready: Arc::new(RwLock::new(false)),
            semaphore: Arc::new(tokio::sync::Semaphore::new(default_max_concurrent())),
        };

        // Check initial readiness
        if backend.client.is_ready().await {
            *backend.ready.write().await = true;
            info!("Flask VLM backend ready");
        } else {
            warn!("Flask VLM server not ready - inference will fail until server starts");
        }

        Ok(backend)
    }

    /// Convert image source to JPEG bytes.
    fn image_to_jpeg(&self, source: &VlmImageSource) -> Result<Vec<u8>> {
        match source {
            VlmImageSource::Jpeg(data) => Ok(data.clone()),
            VlmImageSource::Png(data) => {
                // Convert PNG to JPEG
                let img = image::load_from_memory_with_format(data, image::ImageFormat::Png)
                    .context("Failed to decode PNG")?;
                let mut jpeg_data = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut jpeg_data);
                img.write_to(&mut cursor, image::ImageFormat::Jpeg)
                    .context("Failed to encode JPEG")?;
                Ok(jpeg_data)
            }
            VlmImageSource::RawRgb { width, height, data } => {
                let img = image::RgbImage::from_raw(*width, *height, data.clone())
                    .context("Invalid RGB dimensions")?;
                let mut jpeg_data = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut jpeg_data);
                img.write_to(&mut cursor, image::ImageFormat::Jpeg)
                    .context("Failed to encode JPEG")?;
                Ok(jpeg_data)
            }
            VlmImageSource::RawRgba { width, height, data } => {
                let img = image::RgbaImage::from_raw(*width, *height, data.clone())
                    .context("Invalid RGBA dimensions")?;
                let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
                let mut jpeg_data = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut jpeg_data);
                rgb.write_to(&mut cursor, image::ImageFormat::Jpeg)
                    .context("Failed to encode JPEG")?;
                Ok(jpeg_data)
            }
            VlmImageSource::Nv12 { width, height, data } => {
                // Convert NV12 to RGB then to JPEG
                let rgb = self.nv12_to_rgb(data, *width, *height)?;
                let img = image::RgbImage::from_raw(*width, *height, rgb)
                    .context("Failed to create RGB image")?;
                let mut jpeg_data = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut jpeg_data);
                img.write_to(&mut cursor, image::ImageFormat::Jpeg)
                    .context("Failed to encode JPEG")?;
                Ok(jpeg_data)
            }
            VlmImageSource::SharedMemory { .. } => {
                anyhow::bail!("Shared memory not supported by Flask backend")
            }
        }
    }

    /// Convert NV12 to RGB (CPU implementation).
    fn nv12_to_rgb(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        let y_size = (width * height) as usize;
        let uv_size = y_size / 2;

        if data.len() < y_size + uv_size {
            anyhow::bail!(
                "NV12 data too small: expected {}, got {}",
                y_size + uv_size,
                data.len()
            );
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
}

#[async_trait]
impl VlmBackend for FlaskVlmBackend {
    fn capabilities(&self) -> &VlmCapabilities {
        &self.capabilities
    }

    async fn analyze(&self, request: VlmRequest) -> PlatformResult<VlmResponse> {
        // Acquire semaphore permit
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            PlatformError::Other(format!("Failed to acquire semaphore: {}", e))
        })?;

        let start = Instant::now();

        // Convert image to JPEG
        let jpeg_data = self
            .image_to_jpeg(&request.image)
            .map_err(|e| PlatformError::Other(format!("Image conversion failed: {}", e)))?;

        debug!(
            "Converted image: {} bytes -> {} bytes JPEG",
            request.image.data_size(),
            jpeg_data.len()
        );

        // Send inference request
        let response = self
            .client
            .infer(
                &request.prompt,
                &jpeg_data,
                request.max_tokens,
                request.temperature,
            )
            .await
            .map_err(|e| PlatformError::Other(format!("VLM inference failed: {}", e)))?;

        let inference_time_ms = if response.generation_time_ms > 0.0 {
            response.generation_time_ms
        } else {
            start.elapsed().as_secs_f32() * 1000.0
        };

        Ok(VlmResponse {
            request_id: request.request_id,
            analysis: response.response,
            inference_time_ms,
            tokens_generated: response.tokens_generated,
            model: "llava-1.5-7b-hf".to_string(),
        })
    }

    async fn analyze_batch(&self, request: VlmBatchRequest) -> PlatformResult<VlmBatchResult> {
        let start = Instant::now();
        let mut result = VlmBatchResult::new(&request.request_id);

        // Flask backend doesn't support true batching, process sequentially
        for req in request.requests {
            let req_id = req.request_id.clone();
            match self.analyze(req).await {
                Ok(response) => result.add_response(response),
                Err(e) => result.add_error(req_id, e.to_string()),
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

        // Re-check server status
        if self.client.is_ready().await {
            *self.ready.write().await = true;
            true
        } else {
            false
        }
    }

    async fn warmup(&self) -> PlatformResult<()> {
        info!("Warming up Flask VLM backend...");

        // Wait for server to be ready
        self.client
            .wait_ready(std::time::Duration::from_secs(120))
            .await
            .map_err(|e| PlatformError::Other(format!("Server not ready: {}", e)))?;

        *self.ready.write().await = true;

        // Run a warmup inference with a small test image
        let warmup_image = create_test_jpeg(64, 64)?;
        let warmup_request = VlmRequest::new(
            "warmup",
            "What do you see?",
            VlmImageSource::Jpeg(warmup_image),
        );

        match self.analyze(warmup_request).await {
            Ok(resp) => info!(
                "Flask warmup successful: {} tokens in {:.0}ms",
                resp.tokens_generated, resp.inference_time_ms
            ),
            Err(e) => warn!("Flask warmup inference failed (non-fatal): {}", e),
        }

        info!("Flask VLM backend warmup complete");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "flask-llava"
    }

    async fn shutdown(&self) -> PlatformResult<()> {
        info!("Shutting down Flask VLM backend");
        *self.ready.write().await = false;
        Ok(())
    }
}

/// Create a small test JPEG image for warmup.
fn create_test_jpeg(width: u32, height: u32) -> PlatformResult<Vec<u8>> {
    let img = image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([
            ((x * 255) / width) as u8,
            ((y * 255) / height) as u8,
            128,
        ])
    });

    let mut jpeg_data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut jpeg_data);
    img.write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| PlatformError::Other(format!("Failed to create test JPEG: {}", e)))?;

    Ok(jpeg_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FlaskVlmConfig::default();
        assert!(!config.server_url.is_empty());
        assert!(config.max_tokens > 0);
    }

    #[test]
    fn test_create_test_jpeg() {
        let jpeg = create_test_jpeg(64, 64).unwrap();
        assert!(!jpeg.is_empty());
        // JPEG magic bytes
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
    }
}
