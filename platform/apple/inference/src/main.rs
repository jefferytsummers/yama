//! Yama Inference Service - Apple Silicon
//!
//! This service provides LLM inference using TTRA or llama.cpp
//! with Metal acceleration on Apple Silicon.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use yama_container_sdk::{EventBusClient, HealthReporter};
use yama_platform_traits::inference::InferenceEngine;

mod engine;

pub use engine::{create_inference_engine, MetalInferenceEngine};

/// Service configuration.
#[derive(Debug, Clone, Deserialize)]
struct Config {
    /// Event bus socket path.
    event_bus_socket: String,
    /// TTRA server URL (if using TTRA backend).
    ttra_url: Option<String>,
    /// Model path (if using local llama.cpp).
    model_path: Option<String>,
    /// Default model to use.
    default_model: String,
    /// Maximum concurrent requests.
    max_concurrent: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            event_bus_socket: "/tmp/yama-event.sock".to_string(),
            ttra_url: Some("http://localhost:8080".to_string()),
            model_path: None,
            default_model: "llama-3.2-3b".to_string(),
            max_concurrent: 4,
        }
    }
}

/// Inference request from event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InferenceRequestMessage {
    request_id: String,
    model: Option<String>,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

/// Message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

/// Inference response chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InferenceResponseMessage {
    request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inference_time_us: Option<u64>,
}

/// Inference service using platform traits.
struct InferenceService {
    engine: Box<dyn InferenceEngine>,
    event_bus: Arc<EventBusClient>,
    health: HealthReporter,
    active_requests: Arc<RwLock<usize>>,
    max_concurrent: usize,
}

impl InferenceService {
    async fn new(config: &Config) -> Result<Self> {
        // Connect to event bus
        let event_bus = Arc::new(
            EventBusClient::connect(&config.event_bus_socket, "inference-apple")
                .await
                .context("Failed to connect to event bus")?,
        );
        info!("Connected to event bus");

        // Set up health reporter
        let health = HealthReporter::new(event_bus.clone(), "inference-apple");

        // Initialize platform-specific inference engine
        let ttra_url = config
            .ttra_url
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("http://localhost:8080");

        let engine = create_inference_engine(ttra_url, &config.default_model)?;

        info!(
            "Inference engine initialized: {} on {}",
            engine.capabilities().name,
            engine.capabilities().device
        );
        info!(
            "Available memory: {}GB, Compute: {:.1} TFLOPS",
            engine.capabilities().available_memory / (1024 * 1024 * 1024),
            engine.capabilities().compute_tflops
        );

        Ok(Self {
            engine,
            event_bus,
            health,
            active_requests: Arc::new(RwLock::new(0)),
            max_concurrent: config.max_concurrent,
        })
    }

    async fn run(mut self) -> Result<()> {
        info!("Starting inference service");

        // Start health reporting
        self.health.healthy().await;
        self.health
            .set_component(
                "engine",
                yama_container_sdk::health::ComponentStatus::healthy(),
            )
            .await;
        self.health.start().await?;

        // Subscribe to inference requests
        self.event_bus
            .subscribe(&["inference.request"])
            .await
            .context("Failed to subscribe")?;

        info!("Listening for inference requests");

        // Report memory usage periodically
        let mut stats_interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = stats_interval.tick() => {
                    let usage = self.engine.memory_usage();
                    info!(
                        "Memory usage: {}/{} MB ({} models loaded)",
                        usage.model_bytes / (1024 * 1024),
                        usage.total_bytes / (1024 * 1024),
                        usage.models_loaded
                    );
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    // Poll for events
                }
            }
        }
    }

    async fn handle_request(&mut self, request: InferenceRequestMessage) -> Result<()> {
        // Check concurrency limit
        {
            let active = *self.active_requests.read().await;
            if active >= self.max_concurrent {
                warn!("Max concurrent requests reached");
                let response = InferenceResponseMessage {
                    request_id: request.request_id,
                    text: None,
                    done: Some(true),
                    error: Some("Server at capacity".to_string()),
                    inference_time_us: None,
                };
                self.event_bus
                    .publish("inference.response", response)
                    .await?;
                return Ok(());
            }
        }

        // Increment active count
        *self.active_requests.write().await += 1;

        let request_id = request.request_id.clone();

        // Build inference request for platform trait
        let prompt = request
            .messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let mut params = HashMap::new();
        params.insert("prompt".to_string(), serde_json::json!(prompt));
        params.insert("temperature".to_string(), serde_json::json!(request.temperature));
        params.insert("max_tokens".to_string(), serde_json::json!(request.max_tokens));

        let platform_request = yama_platform_traits::inference::InferenceRequest {
            id: request.request_id.clone(),
            model: request.model.unwrap_or_default(),
            inputs: vec![],
            params,
        };

        // Run inference
        match self.engine.infer(platform_request).await {
            Ok(response) => {
                // Extract text from output tensor
                let text = response
                    .outputs
                    .first()
                    .map(|t| String::from_utf8_lossy(&t.data).to_string());

                let msg = InferenceResponseMessage {
                    request_id,
                    text,
                    done: Some(true),
                    error: None,
                    inference_time_us: Some(response.inference_time_us),
                };

                self.event_bus
                    .publish("inference.response", msg)
                    .await?;
            }
            Err(e) => {
                error!("Inference error: {}", e);
                let msg = InferenceResponseMessage {
                    request_id,
                    text: None,
                    done: Some(true),
                    error: Some(e.to_string()),
                    inference_time_us: None,
                };

                self.event_bus
                    .publish("inference.response", msg)
                    .await?;
            }
        }

        // Decrement active count
        *self.active_requests.write().await -= 1;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .pretty()
        .init();

    info!("Starting Yama Inference Service (Apple Silicon)");

    // Print platform capabilities
    let test_engine = create_inference_engine("http://localhost:8080", "test")?;
    info!("Platform: {}", test_engine.capabilities().name);
    info!("Backend: {}", test_engine.capabilities().backend);
    info!("Device: {}", test_engine.capabilities().device);
    info!(
        "Supported formats: {:?}",
        test_engine.capabilities().supported_formats
    );
    drop(test_engine);

    // Load configuration
    let config = Config::default(); // Would load from file or env

    // Create and run service
    let service = InferenceService::new(&config).await?;
    service.run().await
}
