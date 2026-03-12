//! Yama Compositor - VLM Video Inference MVP
//!
//! Native egui application for video analysis using Vision Language Models.
//! Features a chat-centric interface for conversational video analysis.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use eframe::egui;
use tokio::sync::RwLock;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use yama_theme::YamaTheme;

mod compositor;
mod ui;

use ui::{app::AppState, ChatApp};

// Use modules from the library crate
use yama_host_apple::db::Database;
use yama_host_apple::event_bus::{EventBus, EventBusConfig};
use yama_host_apple::http_server::{HttpServer, HttpServerConfig};
use yama_host_apple::inference::{VlmInferenceConfig, VlmInferenceService};
use yama_host_apple::orchestrator::{Orchestrator, OrchestratorConfig};
use yama_host_apple::ServiceState;

/// Application configuration (extends library config with compositor).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub compositor: compositor::CompositorConfig,
    #[serde(default)]
    pub event_bus: EventBusConfig,
    #[serde(default)]
    pub orchestrator: OrchestratorConfig,
    #[serde(default)]
    pub http_server: HttpServerConfig,
    #[serde(default)]
    pub inference: VlmInferenceConfig,
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
            dirs::config_dir()
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

    /// Convert to library config.
    pub fn to_lib_config(&self) -> yama_host_apple::Config {
        yama_host_apple::Config {
            event_bus: self.event_bus.clone(),
            orchestrator: self.orchestrator.clone(),
            http_server: self.http_server.clone(),
            inference: self.inference.clone(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            compositor: compositor::CompositorConfig::default(),
            event_bus: EventBusConfig::default(),
            orchestrator: OrchestratorConfig::default(),
            http_server: HttpServerConfig::default(),
            inference: VlmInferenceConfig::default(),
        }
    }
}

// Use ServiceState from library

fn main() -> Result<()> {
    // Initialize logging
    let _guard = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .pretty()
        .try_init();

    info!("Starting Yama compositor (Apple Silicon)");
    info!("Platform: {} {}", std::env::consts::OS, std::env::consts::ARCH);

    // Create tokio runtime for async services
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("Failed to create tokio runtime")?,
    );

    // Load configuration
    let config = Config::load_default().context("Failed to load configuration")?;
    info!("Configuration loaded");

    // Initialize services in the runtime
    let (service_state, app_state) = runtime.block_on(async {
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
        let inference_service = match VlmInferenceService::new(config.inference.clone()).await {
            Ok(service) => {
                info!("VLM inference service initialized");
                Some(Arc::new(service))
            }
            Err(e) => {
                error!("Failed to initialize VLM inference service: {}", e);
                None
            }
        };

        // Create service state for HTTP server (using library's ServiceState)
        let service_state = Arc::new(ServiceState {
            config: config.to_lib_config(),
            database,
            event_bus,
            orchestrator,
            inference_service: inference_service.clone(),
        });

        // Create app state for ChatApp
        let app_state = Arc::new(AppState {
            inference_service,
        });

        Ok::<_, anyhow::Error>((service_state, app_state))
    })?;

    // Start background services
    let orchestrator = service_state.orchestrator.clone();

    // Event bus is already started above, just start orchestrator
    runtime.spawn(async move {
        let mut orch = orchestrator.write().await;
        if let Err(e) = orch.start().await {
            error!("Orchestrator error: {}", e);
        }
    });

    // Start HTTP server for Host UI
    let http_config = service_state.config.http_server.clone();
    let http_state = service_state.clone();
    runtime.spawn(async move {
        let server = HttpServer::new(http_config, http_state);
        if let Err(e) = server.run().await {
            error!("HTTP server error: {}", e);
        }
    });

    // Configure eframe
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([600.0, 500.0])
            .with_title("Yama - Video Analysis"),
        ..Default::default()
    };

    info!("Starting chat UI");

    // Run eframe with ChatApp
    eframe::run_native(
        "Yama",
        options,
        Box::new(move |cc| {
            // Apply Yama theme
            YamaTheme::new().apply(&cc.egui_ctx);
            Ok(Box::new(ChatApp::new(app_state, runtime)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    info!("Shutdown complete");
    Ok(())
}

// Re-export for use in other modules
mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
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
                .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
        }
    }
}
