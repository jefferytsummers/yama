//! Yama VLM Container
//!
//! Vision Language Model inference container for video analysis.
//! Supports live video stream analysis and batch video file processing.

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use yama_container_sdk::health::ComponentStatus;
use yama_container_sdk::{EventBusClient, HealthReporter};
use yama_protocol::vlm::{VlmAnalyzeProgress, VlmAnalyzeResponse, VlmBatchProgress};

mod batch_processor;
mod config;
mod engine;
mod frame_sampler;
mod handlers;
mod image_encoder;

use batch_processor::BatchProcessor;
use config::VlmConfig;
use engine::VlmEngine;
use frame_sampler::FrameSampler;
use handlers::{DispatchResult, EventDispatcher};

/// VLM service managing model inference and event handling.
struct VlmService {
    config: VlmConfig,
    event_bus: EventBusClient,
    event_bus_shared: Arc<EventBusClient>,
    health: HealthReporter,
    engine: Arc<VlmEngine>,
    sampler: Arc<FrameSampler>,
    dispatcher: EventDispatcher,
    result_rx: mpsc::Receiver<VlmAnalyzeResponse>,
    batch_progress_rx: mpsc::Receiver<VlmBatchProgress>,
    analyze_progress_rx: mpsc::Receiver<VlmAnalyzeProgress>,
}

impl VlmService {
    /// Create a new VLM service.
    async fn new(config: VlmConfig) -> Result<Self> {
        info!("Initializing VLM service");

        // Connect to event bus (we need two connections - one for sending, one for receiving)
        // Use WebSocket URL if provided, otherwise use Unix socket
        let event_bus = if let Some(ref url) = config.event_bus_url {
            info!("Connecting to event bus via WebSocket: {}", url);
            EventBusClient::connect_ws(url, "vlm")
                .await
                .context("Failed to connect to event bus via WebSocket")?
        } else {
            info!("Connecting to event bus via Unix socket: {}", config.event_bus_socket);
            EventBusClient::connect(&config.event_bus_socket, "vlm")
                .await
                .context("Failed to connect to event bus")?
        };

        let event_bus_shared = Arc::new(if let Some(ref url) = config.event_bus_url {
            EventBusClient::connect_ws(url, "vlm-sender")
                .await
                .context("Failed to connect to event bus (sender) via WebSocket")?
        } else {
            EventBusClient::connect(&config.event_bus_socket, "vlm-sender")
                .await
                .context("Failed to connect to event bus (sender)")?
        });

        info!("Connected to event bus");

        // Set up health reporter
        let health = HealthReporter::new(event_bus_shared.clone(), "vlm");

        // Initialize VLM engine
        info!("Loading VLM model...");
        let engine = Arc::new(
            VlmEngine::new(config.model.clone())
                .await
                .context("Failed to initialize VLM engine")?,
        );
        info!("VLM model loaded");

        // Create frame sampler
        let sampler = Arc::new(FrameSampler::new(config.live_stream.clone()));

        // Create batch processor
        let batch_processor = Arc::new(BatchProcessor::new(config.batch.clone(), engine.clone()));

        // Create channels for results
        let (result_tx, result_rx) = mpsc::channel::<VlmAnalyzeResponse>(100);
        let (batch_progress_tx, batch_progress_rx) = mpsc::channel::<VlmBatchProgress>(100);
        let (analyze_progress_tx, analyze_progress_rx) = mpsc::channel::<VlmAnalyzeProgress>(100);

        // Create event dispatcher
        let dispatcher = EventDispatcher::new(
            engine.clone(),
            sampler.clone(),
            Some(batch_processor),
            result_tx,
            batch_progress_tx,
            analyze_progress_tx,
        );

        Ok(Self {
            config,
            event_bus,
            event_bus_shared,
            health,
            engine,
            sampler,
            dispatcher,
            result_rx,
            batch_progress_rx,
            analyze_progress_rx,
        })
    }

    /// Run the VLM service.
    async fn run(mut self) -> Result<()> {
        info!("Starting VLM service");

        // Start health reporting
        self.health.healthy().await;
        self.health
            .set_component("engine", ComponentStatus::healthy())
            .await;
        self.health.start().await?;

        // Subscribe to relevant topics
        self.event_bus
            .subscribe(&[
                "vlm.analyze.request",
                "vlm.batch.request",
                "vlm.config.request",
            ])
            .await
            .context("Failed to subscribe to VLM topics")?;

        // Subscribe to video frame topics if live streaming is enabled
        if self.config.live_stream.enabled {
            self.event_bus
                .subscribe_patterns(&["video.frame.*"])
                .await
                .context("Failed to subscribe to video topics")?;
            info!("Live stream analysis enabled");
        }

        // Warm up the model
        if let Err(e) = self.engine.warmup().await {
            warn!("Model warmup failed: {}", e);
            self.health
                .set_component(
                    "engine",
                    ComponentStatus::unhealthy(format!("Warmup failed: {}", e)),
                )
                .await;
        } else {
            info!("Model warmup complete");
        }

        // Register auto-start sources
        for source_id in &self.config.live_stream.auto_start_sources {
            self.sampler.register_source_default(source_id).await;
            info!("Auto-registered source for analysis: {}", source_id);
        }

        info!("VLM service ready");

        // Main event loop
        loop {
            tokio::select! {
                // Handle incoming events
                event = self.event_bus.recv() => {
                    if let Some(event) = event {
                        match self.dispatcher.dispatch(event).await {
                            Ok(Some(result)) => {
                                if let Err(e) = self.publish_result(result).await {
                                    error!("Failed to publish result: {}", e);
                                }
                            }
                            Ok(None) => {
                                // No response needed
                            }
                            Err(e) => {
                                error!("Error handling event: {}", e);
                            }
                        }
                    }
                }

                // Publish analysis results from live stream
                Some(response) = self.result_rx.recv() => {
                    if let Err(e) = self.event_bus_shared.publish("vlm.analyze.result", response).await {
                        error!("Failed to publish analyze result: {}", e);
                    }
                }

                // Publish batch progress updates
                Some(progress) = self.batch_progress_rx.recv() => {
                    if let Err(e) = self.event_bus_shared.publish("vlm.batch.progress", progress).await {
                        error!("Failed to publish batch progress: {}", e);
                    }
                }

                // Publish per-frame analysis progress heartbeats
                Some(progress) = self.analyze_progress_rx.recv() => {
                    if let Err(e) = self.event_bus_shared.publish("vlm.analyze.progress", progress).await {
                        debug!("Failed to publish analyze progress: {}", e);
                    }
                }
            }
        }
    }

    /// Publish a dispatch result to the event bus.
    async fn publish_result(&self, result: DispatchResult) -> Result<()> {
        match result {
            DispatchResult::AnalyzeResponse(response) => {
                self.event_bus_shared
                    .publish("vlm.analyze.result", response)
                    .await?;
            }
            DispatchResult::BatchResult(result) => {
                self.event_bus_shared
                    .publish("vlm.batch.result", result)
                    .await?;
            }
            DispatchResult::ConfigResponse(response) => {
                self.event_bus_shared
                    .publish("vlm.config.response", response)
                    .await?;
            }
        }
        Ok(())
    }
}

/// Load configuration from environment and files.
fn load_config() -> Result<VlmConfig> {
    VlmConfig::from_env()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .pretty()
        .init();

    info!("Starting Yama VLM Container");

    // Load configuration
    let config = load_config()?;
    info!("Configuration loaded");
    debug!("Model: {}", config.model.model_id);
    debug!("ISQ: {}", config.model.isq);
    debug!("Live stream enabled: {}", config.live_stream.enabled);

    // Create and run service
    let service = VlmService::new(config).await?;
    service.run().await
}
