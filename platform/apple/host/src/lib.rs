//! Yama Host Library
//!
//! Shared components for the Yama host application.
//! This module is used by both the GUI (`main.rs`), headless (`bin/headless.rs`), and Tauri binaries.
//!
//! Note: Embedding, transcription, and indexer modules have been archived to `_archived/`
//! as the project has shifted focus to chat-centric VLM interactions.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::RwLock;
use tracing::{error, info};

pub mod agent;
pub mod artifact;
pub mod db;
pub mod event_bus;
pub mod http_server;
pub mod inference;
pub mod models;
pub mod orchestrator;

// Archived modules (moved to _archived/):
// - embedding: CLIP embeddings (unused in chat mode)
// - transcription: Whisper transcription (unused in chat mode)
// - indexer: Keyframe extraction (unused in chat mode)

// Re-export commonly used types
pub use db::{Database, DatabaseStats};
pub use event_bus::{EventBus, EventBusConfig};
pub use inference::{
    BackendType, InferenceBackend, InferenceChunk, InferenceJob, JobStatus, MockBackend,
    UploadInfo, VlmInferenceConfig, VlmInferenceService, VlmModelInfo,
};
pub use models::{ModelManager, ModelManagerConfig, ModelRegistry, ModelType};

// Agent re-exports
pub use agent::{
    AgentPreset, AuditEntry, ChatChunk, ChatSession, PresetId, PresetRegistry, RateLimitConfig,
    SessionConfig, Tool, ToolConfig, ToolContext, ToolDefinition, ToolExecutor, ToolRegistry,
    ToolResult, ToolValidator, ValidationError, ValidationResult,
};

// Chat history re-exports
pub use db::{ChatHistory, ChatMessageRow, ChatSessionRow, MessageRole, StoredToolCall};

// Artifact re-exports
pub use artifact::{
    Artifact, ArtifactMetadata, ArtifactQuery, ArtifactStore, ArtifactStoreConfig, ArtifactType,
    NewArtifact,
};

// HTTP server and orchestrator re-exports
pub use http_server::{HttpServer, HttpServerConfig};
pub use orchestrator::{Orchestrator, OrchestratorConfig};

/// Configuration for the Yama host.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub event_bus: EventBusConfig,
    #[serde(default)]
    pub orchestrator: OrchestratorConfig,
    #[serde(default)]
    pub http_server: HttpServerConfig,
    #[serde(default)]
    pub inference: inference::VlmInferenceConfig,
}

impl Config {
    /// Load configuration from file.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {:?}", path))?;
        toml::from_str(&content).context("Failed to parse config")
    }

    /// Load configuration from default locations.
    pub fn load_default() -> Result<Self> {
        let paths = [
            PathBuf::from("yama.toml"),
            PathBuf::from("/etc/yama/yama.toml"),
            config_dir()
                .map(|p| p.join("yama/yama.toml"))
                .unwrap_or_default(),
        ];

        for path in &paths {
            if path.exists() {
                return Self::load(path);
            }
        }

        Ok(Self::default())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            event_bus: EventBusConfig::default(),
            orchestrator: OrchestratorConfig::default(),
            http_server: HttpServerConfig::default(),
            inference: inference::VlmInferenceConfig::default(),
        }
    }
}

/// Shared state for services (HTTP server, etc.).
pub struct ServiceState {
    pub config: Config,
    pub database: Arc<Database>,
    pub event_bus: Arc<EventBus>,
    pub orchestrator: Arc<RwLock<Orchestrator>>,
    pub inference_service: Option<Arc<inference::VlmInferenceService>>,
}

/// Run Yama host services without a GUI window.
///
/// This is used by the Tauri wrapper to run backend services while the
/// SvelteKit frontend handles the UI in a webview.
///
/// The function starts:
/// - Event bus (WebSocket + Unix socket)
/// - Orchestrator (container management)
/// - VLM inference service
/// - HTTP server (blocks on this)
pub async fn run_headless() -> Result<()> {
    info!("Starting Yama host (headless mode)");

    // Load configuration
    let config = Config::load_default().context("Failed to load configuration")?;
    info!("Configuration loaded");

    // Initialize database
    let database = Arc::new(
        Database::open_default()
            .await
            .context("Failed to initialize database")?,
    );
    info!("Database initialized at {:?}", database.path);

    // Initialize event bus
    let event_bus = Arc::new(
        EventBus::new(config.event_bus.clone())
            .await
            .context("Failed to initialize event bus")?,
    );
    info!("Event bus initialized");

    // Start event bus server BEFORE other services try to connect
    let event_bus_runner = event_bus.clone();
    tokio::spawn(async move {
        if let Err(e) = event_bus_runner.run().await {
            error!("Event bus error: {}", e);
        }
    });

    // Give the event bus server time to start listening
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    info!("Event bus server started");

    // Initialize orchestrator
    let orchestrator = Arc::new(RwLock::new(
        Orchestrator::new(config.orchestrator.clone(), event_bus.clone())
            .await
            .context("Failed to initialize orchestrator")?,
    ));
    info!("Orchestrator initialized");

    // Initialize VLM inference service (now event bus is ready)
    let inference_service = match inference::VlmInferenceService::new(config.inference.clone()).await
    {
        Ok(service) => {
            info!("VLM inference service initialized");
            Some(Arc::new(service))
        }
        Err(e) => {
            error!("Failed to initialize VLM inference service: {}", e);
            None
        }
    };

    // Create service state for HTTP server
    let service_state = Arc::new(ServiceState {
        config: config.clone(),
        database,
        event_bus,
        orchestrator: orchestrator.clone(),
        inference_service,
    });

    // Start orchestrator in background
    let orch_handle = orchestrator.clone();
    tokio::spawn(async move {
        let mut orch = orch_handle.write().await;
        if let Err(e) = orch.start().await {
            error!("Orchestrator error: {}", e);
        }
    });

    // Start HTTP server (this blocks)
    let http_config = service_state.config.http_server.clone();
    let server = HttpServer::new(http_config, service_state);
    server.run().await?;

    Ok(())
}

/// Get the user's config directory.
fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("Library/Application Support"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
    }
}
