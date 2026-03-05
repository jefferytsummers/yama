//! Yama Protocol Library
//!
//! This crate provides Protocol Buffer definitions and generated Rust types
//! for the Yama event bus and inter-service communication.
//!
//! # Modules
//!
//! - `common`: Shared types used across all protocol messages (envelopes, errors, health)
//! - `agent`: LLM agent communication (conversations, tools, hooks)
//! - `system`: System management (orchestration, video sources, UI)
//!
//! # Example
//!
//! ```rust
//! use yama_protocol::common::{Envelope, ServiceStatus};
//! use yama_protocol::agent::{ConversationRequest, Message, MessageRole};
//!
//! // Create a conversation request
//! let request = ConversationRequest {
//!     agent_id: "vision-agent".to_string(),
//!     connection_id: "conn-123".to_string(),
//!     messages: vec![
//!         Message {
//!             role: MessageRole::User as i32,
//!             content: Some(message::Content::Text("Describe the scene".to_string())),
//!             ..Default::default()
//!         },
//!     ],
//!     ..Default::default()
//! };
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

use prost::Message;

/// Common types shared across all protocol messages.
pub mod common {
    include!(concat!(env!("OUT_DIR"), "/yama.common.rs"));
}

/// Agent communication types.
pub mod agent {
    include!(concat!(env!("OUT_DIR"), "/yama.agent.rs"));
}

/// System management types.
pub mod system {
    include!(concat!(env!("OUT_DIR"), "/yama.system.rs"));
}

/// Re-export commonly used types at the crate root.
pub use common::{Envelope, Error, ErrorCode, ServiceInfo, ServiceStatus, ServiceType};

/// Encode a message to bytes.
///
/// # Errors
///
/// Returns an error if encoding fails.
pub fn encode<M: Message>(message: &M) -> Result<Vec<u8>, prost::EncodeError> {
    let mut buf = Vec::with_capacity(message.encoded_len());
    message.encode(&mut buf)?;
    Ok(buf)
}

/// Decode a message from bytes.
///
/// # Errors
///
/// Returns an error if decoding fails.
pub fn decode<M: Message + Default>(buf: &[u8]) -> Result<M, prost::DecodeError> {
    M::decode(buf)
}

/// Create a new envelope for a message.
pub fn envelope(
    topic: impl Into<String>,
    source: impl Into<String>,
    payload: impl Message,
) -> common::Envelope {
    use prost_types::Any;

    let payload_bytes = {
        let mut buf = Vec::new();
        payload.encode(&mut buf).expect("encoding should succeed");
        buf
    };

    common::Envelope {
        message_id: uuid_v4(),
        topic: topic.into(),
        source: source.into(),
        target: String::new(),
        timestamp: Some(now()),
        correlation_id: String::new(),
        payload: Some(Any {
            type_url: String::new(), // TODO: Set proper type URL
            value: payload_bytes,
        }),
        priority: 0,
        ttl_ms: 0,
    }
}

/// Generate a UUID v4.
fn uuid_v4() -> String {
    // Simple UUID v4 generation without external dependency
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        now.as_secs() as u32,
        (now.subsec_nanos() >> 16) as u16,
        (now.subsec_nanos() & 0xFFF) as u16,
        0x8000 | (now.as_nanos() as u16 & 0x3FFF),
        now.as_nanos() as u64 & 0xFFFFFFFFFFFF
    )
}

/// Get current timestamp.
fn now() -> prost_types::Timestamp {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    prost_types::Timestamp {
        seconds: duration.as_secs() as i64,
        nanos: duration.subsec_nanos() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_creation() {
        let env = envelope("test.topic", "test-service", common::HealthCheckRequest {
            service_id: "svc-1".to_string(),
        });

        assert_eq!(env.topic, "test.topic");
        assert_eq!(env.source, "test-service");
        assert!(env.payload.is_some());
    }

    #[test]
    fn test_encode_decode() {
        let request = common::HealthCheckRequest {
            service_id: "test-service".to_string(),
        };

        let encoded = encode(&request).expect("encoding should succeed");
        let decoded: common::HealthCheckRequest =
            decode(&encoded).expect("decoding should succeed");

        assert_eq!(decoded.service_id, "test-service");
    }
}
