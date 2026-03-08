//! Standalone Event Bus Server
//!
//! Runs the event bus server without the full Yama host.
//! Used for docker-compose development workflow.
//!
//! # Usage
//!
//! ```bash
//! yama-event-bus
//! ```

use std::process::ExitCode;

use anyhow::{Context, Result};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use yama_host_apple::event_bus::{EventBus, EventBusConfig};

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("Event bus error: {:#}", e);
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    info!("Starting Yama Event Bus Server");

    // Load config from environment or defaults
    let config = EventBusConfig {
        websocket_bind: std::env::var("EVENT_BUS_WS_BIND")
            .unwrap_or_else(|_| "0.0.0.0:8765".to_string()),
        unix_socket: std::env::var("EVENT_BUS_UNIX_SOCKET")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/yama-event.sock")),
        ..Default::default()
    };

    info!("WebSocket: {}", config.websocket_bind);
    info!("Unix socket: {:?}", config.unix_socket);

    let event_bus = EventBus::new(config)
        .await
        .context("Failed to create event bus")?;

    info!("Event bus server ready");

    // Run forever
    event_bus.run().await?;

    Ok(())
}
