//! Yama Test Harness
//!
//! Test utilities for Yama services including:
//! - In-memory event bus for unit testing
//! - Mock video sources
//! - Test fixtures and helpers
//!
//! # Example
//!
//! ```rust,ignore
//! use yama_test_harness::{TestEventBus, TestClient};
//!
//! #[tokio::test]
//! async fn test_event_routing() {
//!     let bus = TestEventBus::new();
//!     let client1 = bus.create_client("service-1");
//!     let client2 = bus.create_client("service-2");
//!
//!     client2.subscribe(&["test.topic"]).await.unwrap();
//!     client1.publish("test.topic", my_message).await.unwrap();
//!
//!     let event = client2.recv_timeout(Duration::from_secs(1)).await;
//!     assert!(event.is_some());
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

mod event_bus;
pub mod video_source;
mod vlm_backend;

pub use event_bus::{TestClient, TestEventBus};
pub use video_source::{FramePattern, TestFrame, TestVideoConfig, TestVideoSource};
pub use vlm_backend::TestVlmBackend;
