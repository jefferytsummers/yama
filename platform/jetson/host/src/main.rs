//! Yama Host Process - NVIDIA Jetson
//!
//! This is the main entry point for the Yama compositor on NVIDIA Jetson.
//! It initializes the Smithay compositor with EGL/DRM rendering,
//! starts the event bus, and launches the service orchestrator.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::RwLock;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

mod compositor;
mod event_bus;
mod inference;
mod orchestrator;

use compositor::Compositor;
use event_bus::EventBus;
use orchestrator::Orchestrator;

/// Application configuration.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub compositor: compositor::CompositorConfig,
    pub event_bus: event_bus::EventBusConfig,
    pub orchestrator: orchestrator::OrchestratorConfig,
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
            PathBuf::from("/home/nvidia/.config/yama/yama.toml"),
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
            compositor: compositor::CompositorConfig::default(),
            event_bus: event_bus::EventBusConfig::default(),
            orchestrator: orchestrator::OrchestratorConfig::default(),
        }
    }
}

/// Shared application state.
pub struct AppState {
    pub config: Config,
    pub event_bus: Arc<EventBus>,
    pub orchestrator: Arc<RwLock<Orchestrator>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .pretty()
        .init();

    info!("Starting Yama compositor (NVIDIA Jetson)");
    info!("Platform: {} {}", std::env::consts::OS, std::env::consts::ARCH);

    // Check for NVIDIA runtime
    if std::path::Path::new("/usr/local/cuda").exists() {
        info!("CUDA installation detected");
    }

    // Load configuration
    let config = Config::load_default().context("Failed to load configuration")?;
    info!("Configuration loaded");

    // Initialize event bus
    let event_bus = Arc::new(
        EventBus::new(config.event_bus.clone())
            .await
            .context("Failed to initialize event bus")?,
    );
    info!("Event bus initialized");

    // Initialize orchestrator
    let orchestrator = Arc::new(RwLock::new(
        Orchestrator::new(config.orchestrator.clone(), event_bus.clone())
            .await
            .context("Failed to initialize orchestrator")?,
    ));
    info!("Orchestrator initialized");

    // Create shared state
    let state = Arc::new(AppState {
        config: config.clone(),
        event_bus: event_bus.clone(),
        orchestrator: orchestrator.clone(),
    });

    // Start the event bus server
    let event_bus_handle = {
        let bus = event_bus.clone();
        tokio::spawn(async move {
            if let Err(e) = bus.run().await {
                error!("Event bus error: {}", e);
            }
        })
    };

    // Start the orchestrator
    let orchestrator_handle = {
        let orch = orchestrator.clone();
        tokio::spawn(async move {
            let mut orch = orch.write().await;
            if let Err(e) = orch.start().await {
                error!("Orchestrator error: {}", e);
            }
        })
    };

    // Initialize and run the compositor
    info!("Starting compositor");
    let compositor = Compositor::new(config.compositor, state)
        .context("Failed to initialize compositor")?;

    // Run the compositor (this blocks until exit)
    compositor.run().await?;

    // Cleanup
    info!("Shutting down");

    {
        let mut orch = orchestrator.write().await;
        orch.stop().await?;
    }

    event_bus.shutdown().await?;

    let _ = event_bus_handle.await;
    let _ = orchestrator_handle.await;

    info!("Shutdown complete");
    Ok(())
}
