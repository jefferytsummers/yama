//! Yama Container SDK
//!
//! This crate provides client utilities for containerized services to communicate
//! with the Yama host process via the event bus.
//!
//! # Architecture
//!
//! Containers connect to the host via Unix sockets and communicate using Protocol Buffers.
//! Video frames are shared via DMA-BUF or shared memory for zero-copy transfer.
//!
//! # Example
//!
//! ```rust,ignore
//! use yama_container_sdk::{EventBusClient, HealthReporter};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Connect to the event bus
//!     let client = EventBusClient::connect("/tmp/yama-event.sock").await?;
//!
//!     // Start health reporting
//!     let health = HealthReporter::new(&client, "my-service");
//!     health.start().await?;
//!
//!     // Subscribe to topics
//!     client.subscribe(&["video.frame", "agent.request"]).await?;
//!
//!     // Process messages
//!     while let Some(message) = client.recv().await {
//!         // Handle message
//!     }
//!
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod client;
pub mod health;
pub mod video;

pub use client::EventBusClient;
pub use health::HealthReporter;
pub use video::{VideoFrameReceiver, VideoFrameSender};
