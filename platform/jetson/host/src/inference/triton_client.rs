//! Triton Inference Server gRPC Client
//!
//! Provides a client for communicating with NVIDIA Triton Inference Server
//! using gRPC or HTTP protocols. Supports:
//! - Model inference requests
//! - Model health checks
//! - System shared memory for zero-copy data transfer
//!
//! # Protocol
//!
//! Uses the Triton Inference Server KServe protocol:
//! - gRPC: `grpc://localhost:8001` (default)
//! - HTTP: `http://localhost:8000` (alternative)

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Triton client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TritonClientConfig {
    /// Triton server URL (gRPC endpoint).
    #[serde(default = "default_server_url")]
    pub server_url: String,
    /// Request timeout.
    #[serde(default = "default_timeout")]
    pub timeout: Duration,
    /// Whether to use system shared memory.
    #[serde(default)]
    pub use_shared_memory: bool,
    /// Maximum retries for failed requests.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Retry backoff base (milliseconds).
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
}

fn default_server_url() -> String {
    std::env::var("TRITON_SERVER_URL").unwrap_or_else(|_| "localhost:8001".to_string())
}

fn default_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_backoff_ms() -> u64 {
    500
}

impl Default for TritonClientConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            timeout: default_timeout(),
            use_shared_memory: false,
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
        }
    }
}

/// Input tensor for inference request.
#[derive(Debug, Clone)]
pub struct InferInput {
    /// Input name (must match model configuration).
    pub name: String,
    /// Tensor shape.
    pub shape: Vec<i64>,
    /// Data type ("FP32", "FP16", "INT32", "BYTES", etc.).
    pub datatype: String,
    /// Raw tensor data.
    pub data: Vec<u8>,
}

/// Output tensor from inference response.
#[derive(Debug, Clone)]
pub struct InferOutput {
    /// Output name.
    pub name: String,
    /// Tensor shape.
    pub shape: Vec<i64>,
    /// Data type.
    pub datatype: String,
    /// Raw tensor data.
    pub data: Vec<u8>,
}

/// Inference request.
#[derive(Debug, Clone)]
pub struct InferRequest {
    /// Model name.
    pub model_name: String,
    /// Model version (empty for latest).
    pub model_version: String,
    /// Request ID for correlation.
    pub request_id: String,
    /// Input tensors.
    pub inputs: Vec<InferInput>,
    /// Requested output names (empty for all).
    pub outputs: Vec<String>,
    /// Optional parameters.
    pub parameters: HashMap<String, String>,
}

/// Inference response.
#[derive(Debug, Clone)]
pub struct InferResponse {
    /// Model name.
    pub model_name: String,
    /// Model version.
    pub model_version: String,
    /// Request ID.
    pub request_id: String,
    /// Output tensors.
    pub outputs: Vec<InferOutput>,
    /// Inference statistics.
    pub stats: InferStats,
}

/// Inference statistics.
#[derive(Debug, Clone, Default)]
pub struct InferStats {
    /// Total request time (microseconds).
    pub request_time_us: u64,
    /// Model inference time (microseconds).
    pub compute_time_us: u64,
    /// Queue wait time (microseconds).
    pub queue_time_us: u64,
}

/// Model status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    /// Model name.
    pub name: String,
    /// Model version.
    pub version: String,
    /// Whether the model is ready.
    pub ready: bool,
    /// Model state ("READY", "LOADING", "UNAVAILABLE").
    pub state: String,
}

/// Triton Inference Server client.
///
/// Communicates with Triton using HTTP REST API (simpler than gRPC for initial implementation).
/// Can be upgraded to gRPC for better performance.
pub struct TritonClient {
    config: TritonClientConfig,
    http_client: reqwest::Client,
    /// Base URL for HTTP API.
    http_base_url: String,
}

impl TritonClient {
    /// Create a new Triton client.
    pub async fn new(config: TritonClientConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .pool_max_idle_per_host(4)
            .build()
            .context("Failed to create HTTP client")?;

        // Convert gRPC URL to HTTP (port 8001 -> 8000)
        let http_base_url = if config.server_url.starts_with("http") {
            config.server_url.clone()
        } else {
            // Assume it's just host:port, default to HTTP on port 8000
            let parts: Vec<&str> = config.server_url.split(':').collect();
            let host = parts.first().unwrap_or(&"localhost");
            format!("http://{}:8000", host)
        };

        info!("Triton client created for {}", http_base_url);

        Ok(Self {
            config,
            http_client,
            http_base_url,
        })
    }

    /// Check if the server is live.
    pub async fn is_live(&self) -> bool {
        let url = format!("{}/v2/health/live", self.http_base_url);
        match self.http_client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Check if the server is ready.
    pub async fn is_ready(&self) -> bool {
        let url = format!("{}/v2/health/ready", self.http_base_url);
        match self.http_client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Check if a specific model is ready.
    pub async fn is_model_ready(&self, model_name: &str) -> bool {
        let url = format!("{}/v2/models/{}/ready", self.http_base_url, model_name);
        match self.http_client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Get model metadata.
    pub async fn get_model_metadata(&self, model_name: &str) -> Result<serde_json::Value> {
        let url = format!("{}/v2/models/{}", self.http_base_url, model_name);
        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to get model metadata")?;

        if !resp.status().is_success() {
            anyhow::bail!("Model metadata request failed: {}", resp.status());
        }

        resp.json()
            .await
            .context("Failed to parse model metadata")
    }

    /// Run inference on a model.
    pub async fn infer(&self, request: InferRequest) -> Result<InferResponse> {
        let url = format!(
            "{}/v2/models/{}/infer",
            self.http_base_url, request.model_name
        );

        // Build JSON request body
        let mut body = serde_json::json!({
            "id": request.request_id,
            "inputs": request.inputs.iter().map(|input| {
                serde_json::json!({
                    "name": input.name,
                    "shape": input.shape,
                    "datatype": input.datatype,
                    "data": self.encode_tensor_data(&input.data, &input.datatype),
                })
            }).collect::<Vec<_>>(),
        });

        if !request.outputs.is_empty() {
            body["outputs"] = serde_json::json!(
                request.outputs.iter().map(|name| {
                    serde_json::json!({"name": name})
                }).collect::<Vec<_>>()
            );
        }

        if !request.parameters.is_empty() {
            body["parameters"] = serde_json::to_value(&request.parameters)?;
        }

        debug!("Sending inference request to {}", url);

        let start = std::time::Instant::now();

        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Inference request failed")?;

        let request_time_us = start.elapsed().as_micros() as u64;

        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Inference failed: {}", error_text);
        }

        let response_json: serde_json::Value =
            resp.json().await.context("Failed to parse response")?;

        // Parse response
        let outputs = self.parse_outputs(&response_json)?;

        Ok(InferResponse {
            model_name: request.model_name,
            model_version: response_json["model_version"]
                .as_str()
                .unwrap_or("1")
                .to_string(),
            request_id: response_json["id"]
                .as_str()
                .unwrap_or(&request.request_id)
                .to_string(),
            outputs,
            stats: InferStats {
                request_time_us,
                compute_time_us: 0, // Not available in HTTP response
                queue_time_us: 0,
            },
        })
    }

    /// Encode tensor data for JSON transmission.
    fn encode_tensor_data(&self, data: &[u8], datatype: &str) -> serde_json::Value {
        match datatype {
            "BYTES" => {
                // For string/bytes data, encode as base64
                serde_json::json!([base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    data
                )])
            }
            "FP32" => {
                // Interpret as f32 array
                let floats: Vec<f32> = data
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                serde_json::json!(floats)
            }
            "INT32" => {
                // Interpret as i32 array
                let ints: Vec<i32> = data
                    .chunks_exact(4)
                    .map(|chunk| i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                serde_json::json!(ints)
            }
            "UINT8" => {
                // Raw bytes
                serde_json::json!(data.to_vec())
            }
            _ => {
                // Default: encode as base64
                serde_json::json!([base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    data
                )])
            }
        }
    }

    /// Parse output tensors from JSON response.
    fn parse_outputs(&self, response: &serde_json::Value) -> Result<Vec<InferOutput>> {
        let outputs = response["outputs"]
            .as_array()
            .context("Missing outputs in response")?;

        let mut result = Vec::new();

        for output in outputs {
            let name = output["name"]
                .as_str()
                .context("Missing output name")?
                .to_string();
            let shape: Vec<i64> = output["shape"]
                .as_array()
                .context("Missing output shape")?
                .iter()
                .filter_map(|v| v.as_i64())
                .collect();
            let datatype = output["datatype"]
                .as_str()
                .context("Missing output datatype")?
                .to_string();

            let data = self.decode_tensor_data(&output["data"], &datatype)?;

            result.push(InferOutput {
                name,
                shape,
                datatype,
                data,
            });
        }

        Ok(result)
    }

    /// Decode tensor data from JSON response.
    fn decode_tensor_data(&self, data: &serde_json::Value, datatype: &str) -> Result<Vec<u8>> {
        match datatype {
            "BYTES" => {
                // Decode from base64
                if let Some(arr) = data.as_array() {
                    if let Some(s) = arr.first().and_then(|v| v.as_str()) {
                        return base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
                            .context("Failed to decode base64");
                    }
                }
                // Try as string array
                if let Some(arr) = data.as_array() {
                    let strings: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    return Ok(strings.join("").into_bytes());
                }
                Ok(Vec::new())
            }
            "FP32" => {
                let arr = data.as_array().context("Expected array for FP32")?;
                let mut bytes = Vec::with_capacity(arr.len() * 4);
                for val in arr {
                    let f = val.as_f64().unwrap_or(0.0) as f32;
                    bytes.extend_from_slice(&f.to_le_bytes());
                }
                Ok(bytes)
            }
            "INT32" => {
                let arr = data.as_array().context("Expected array for INT32")?;
                let mut bytes = Vec::with_capacity(arr.len() * 4);
                for val in arr {
                    let i = val.as_i64().unwrap_or(0) as i32;
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                Ok(bytes)
            }
            "UINT8" => {
                let arr = data.as_array().context("Expected array for UINT8")?;
                Ok(arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
            }
            _ => {
                // Default: try as bytes array
                if let Some(arr) = data.as_array() {
                    Ok(arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
                } else {
                    Ok(Vec::new())
                }
            }
        }
    }

    /// Wait for server to become ready with retries.
    pub async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        let check_interval = Duration::from_millis(500);

        while start.elapsed() < timeout {
            if self.is_ready().await {
                info!("Triton server is ready");
                return Ok(());
            }
            tokio::time::sleep(check_interval).await;
        }

        anyhow::bail!("Triton server not ready after {:?}", timeout)
    }

    /// Wait for a specific model to become ready.
    pub async fn wait_model_ready(&self, model_name: &str, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        let check_interval = Duration::from_millis(500);

        while start.elapsed() < timeout {
            if self.is_model_ready(model_name).await {
                info!("Model {} is ready", model_name);
                return Ok(());
            }
            tokio::time::sleep(check_interval).await;
        }

        anyhow::bail!("Model {} not ready after {:?}", model_name, timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TritonClientConfig::default();
        assert!(!config.server_url.is_empty());
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_infer_input() {
        let input = InferInput {
            name: "image".to_string(),
            shape: vec![1, 3, 224, 224],
            datatype: "FP32".to_string(),
            data: vec![0; 4 * 3 * 224 * 224],
        };

        assert_eq!(input.name, "image");
        assert_eq!(input.shape.len(), 4);
    }
}
