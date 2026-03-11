//! HTTP server for the Host UI (Svelte frontend).
//!
//! Provides:
//! - REST API endpoints for service management and system metrics
//! - Static file serving for the Svelte frontend
//! - CORS support for local development

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::info;

// Import ServiceState from the crate root (lib.rs)
use crate::ServiceState;

mod handlers;
pub mod chat_handlers;
pub mod inference_handlers;
mod project_handlers;
mod routes;

/// HTTP server configuration.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HttpServerConfig {
    /// Bind address for the HTTP server.
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Port for the HTTP server.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Dev server port (Vite).
    #[serde(default = "default_dev_port")]
    pub dev_port: u16,
    /// Path to static files (Svelte build output).
    #[serde(default = "default_static_path")]
    pub static_path: PathBuf,
    /// Enable CORS for development.
    #[serde(default = "default_cors_enabled")]
    pub cors_enabled: bool,
}

fn default_bind() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_static_path() -> PathBuf {
    PathBuf::from("web/build")
}

fn default_dev_port() -> u16 {
    5173
}

fn default_cors_enabled() -> bool {
    true
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            port: default_port(),
            dev_port: default_dev_port(),
            static_path: default_static_path(),
            cors_enabled: default_cors_enabled(),
        }
    }
}

/// HTTP server for the Host UI.
pub struct HttpServer {
    config: HttpServerConfig,
    state: Arc<ServiceState>,
}

impl HttpServer {
    /// Create a new HTTP server.
    pub fn new(config: HttpServerConfig, state: Arc<ServiceState>) -> Self {
        Self { config, state }
    }

    /// Run the HTTP server.
    pub async fn run(self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.bind, self.config.port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid bind address: {}", e))?;

        // Build the API router
        let api_router = routes::api_routes(self.state.clone());

        // Build the main router
        let mut app = Router::new().nest("/api", api_router);

        // Add static file serving if path exists
        if self.config.static_path.exists() {
            let serve_dir = ServeDir::new(&self.config.static_path);
            app = app.fallback_service(serve_dir);
            info!("Serving static files from {:?}", self.config.static_path);
        } else {
            info!(
                "Static path {:?} not found, skipping static file serving",
                self.config.static_path
            );
        }

        // Add CORS if enabled
        if self.config.cors_enabled {
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);
            app = app.layer(cors);
        }

        // Add tracing
        app = app.layer(TraceLayer::new_for_http());

        info!("HTTP server listening on http://{}", addr);

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
