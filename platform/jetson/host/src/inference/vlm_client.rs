//! VLM HTTP Client
//!
//! Client for communicating with the Yama VLM inference server.
//! Supports both the Flask-based development server and future Triton gRPC.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// VLM client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmClientConfig {
    /// Server URL (e.g., "http://localhost:8000").
    #[serde(default = "default_server_url")]
    pub server_url: String,
    /// Request timeout.
    #[serde(default = "default_timeout")]
    pub timeout: Duration,
    /// Maximum retries for failed requests.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Default max tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Default temperature for generation.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_server_url() -> String {
    std::env::var("VLM_SERVER_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

fn default_timeout() -> Duration {
    Duration::from_secs(120)
}

fn default_max_retries() -> u32 {
    2
}

fn default_max_tokens() -> u32 {
    512
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for VlmClientConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            timeout: default_timeout(),
            max_retries: default_max_retries(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
}

/// VLM inference request.
#[derive(Debug, Clone, Serialize)]
pub struct VlmInferRequest {
    /// Text prompt for the model.
    pub prompt: String,
    /// Base64-encoded image data.
    pub image: String,
    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Sampling temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// VLM inference response.
#[derive(Debug, Clone, Deserialize)]
pub struct VlmInferResponse {
    /// Generated text response.
    pub response: String,
    /// Number of tokens generated.
    #[serde(default)]
    pub tokens_generated: u32,
    /// Generation time in milliseconds.
    #[serde(default)]
    pub generation_time_ms: f32,
    /// Tokens per second.
    #[serde(default)]
    pub tokens_per_second: f32,
}

/// Health check response.
#[derive(Debug, Clone, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

/// VLM HTTP Client.
///
/// Communicates with the Yama VLM inference server using HTTP REST API.
pub struct VlmClient {
    config: VlmClientConfig,
    http_client: reqwest::Client,
}

impl VlmClient {
    /// Create a new VLM client.
    pub fn new(config: VlmClientConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .pool_max_idle_per_host(4)
            .build()
            .context("Failed to create HTTP client")?;

        info!("VLM client created for {}", config.server_url);

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Check if the server is ready.
    pub async fn is_ready(&self) -> bool {
        let url = format!("{}/v2/health/ready", self.config.server_url);
        match self.http_client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(health) = resp.json::<HealthResponse>().await {
                        return health.status == "ready";
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    /// Check if the server is live.
    pub async fn is_live(&self) -> bool {
        let url = format!("{}/v2/health/live", self.config.server_url);
        match self.http_client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Wait for the server to become ready.
    pub async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        let check_interval = Duration::from_millis(1000);

        while start.elapsed() < timeout {
            if self.is_ready().await {
                info!("VLM server is ready");
                return Ok(());
            }
            debug!("Waiting for VLM server to be ready...");
            tokio::time::sleep(check_interval).await;
        }

        anyhow::bail!("VLM server not ready after {:?}", timeout)
    }

    /// Run inference with an image.
    ///
    /// # Arguments
    /// * `prompt` - Text prompt for the model
    /// * `image_data` - Raw image bytes (JPEG or PNG)
    /// * `max_tokens` - Maximum tokens to generate (optional)
    /// * `temperature` - Sampling temperature (optional)
    pub async fn infer(
        &self,
        prompt: &str,
        image_data: &[u8],
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<VlmInferResponse> {
        let url = format!("{}/v2/models/llava/infer", self.config.server_url);

        // Encode image as base64
        let image_b64 = base64::engine::general_purpose::STANDARD.encode(image_data);

        let request = VlmInferRequest {
            prompt: prompt.to_string(),
            image: image_b64,
            max_tokens: max_tokens.or(Some(self.config.max_tokens)),
            temperature: temperature.or(Some(self.config.temperature)),
        };

        let start = Instant::now();

        let mut last_error = None;
        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                warn!("Retrying inference request (attempt {})", attempt + 1);
                tokio::time::sleep(backoff).await;
            }

            match self.http_client.post(&url).json(&request).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let mut response: VlmInferResponse = resp
                            .json()
                            .await
                            .context("Failed to parse inference response")?;

                        // Fill in timing if not provided by server
                        if response.generation_time_ms == 0.0 {
                            response.generation_time_ms = start.elapsed().as_secs_f32() * 1000.0;
                        }

                        debug!(
                            "Inference completed: {} tokens in {:.0}ms",
                            response.tokens_generated, response.generation_time_ms
                        );

                        return Ok(response);
                    } else {
                        let error_text = resp.text().await.unwrap_or_default();
                        last_error = Some(format!("Server error: {}", error_text));
                    }
                }
                Err(e) => {
                    last_error = Some(format!("Request failed: {}", e));
                }
            }
        }

        anyhow::bail!(
            "Inference failed after {} attempts: {}",
            self.config.max_retries + 1,
            last_error.unwrap_or_else(|| "Unknown error".to_string())
        )
    }

    /// Run inference with a JPEG image.
    pub async fn infer_jpeg(
        &self,
        prompt: &str,
        jpeg_data: &[u8],
        max_tokens: Option<u32>,
    ) -> Result<VlmInferResponse> {
        self.infer(prompt, jpeg_data, max_tokens, None).await
    }

    /// Run inference with a PNG image.
    pub async fn infer_png(
        &self,
        prompt: &str,
        png_data: &[u8],
        max_tokens: Option<u32>,
    ) -> Result<VlmInferResponse> {
        self.infer(prompt, png_data, max_tokens, None).await
    }

    /// Run inference with raw RGB image data.
    ///
    /// Converts RGB to JPEG before sending.
    pub async fn infer_rgb(
        &self,
        prompt: &str,
        width: u32,
        height: u32,
        rgb_data: &[u8],
        max_tokens: Option<u32>,
    ) -> Result<VlmInferResponse> {
        // Convert RGB to JPEG
        let img = image::RgbImage::from_raw(width, height, rgb_data.to_vec())
            .context("Invalid RGB dimensions")?;

        let mut jpeg_data = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut jpeg_data);
        img.write_to(&mut cursor, image::ImageFormat::Jpeg)
            .context("Failed to encode JPEG")?;

        self.infer(prompt, &jpeg_data, max_tokens, None).await
    }

    /// Get server configuration.
    pub fn config(&self) -> &VlmClientConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VlmClientConfig::default();
        assert!(!config.server_url.is_empty());
        assert_eq!(config.timeout, Duration::from_secs(120));
    }

    #[test]
    fn test_infer_request_serialization() {
        let request = VlmInferRequest {
            prompt: "Describe this image.".to_string(),
            image: "base64data".to_string(),
            max_tokens: Some(256),
            temperature: Some(0.7),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("prompt"));
        assert!(json.contains("image"));
    }
}
