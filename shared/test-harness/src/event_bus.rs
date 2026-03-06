//! In-memory event bus for testing.
//!
//! Provides a lightweight event bus that runs entirely in memory,
//! suitable for unit and integration testing without network dependencies.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use prost::Message;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, trace};

use yama_protocol::common::Envelope;

/// In-memory event bus for testing.
///
/// Provides publish/subscribe messaging between test clients
/// without requiring an actual WebSocket server.
#[derive(Clone)]
pub struct TestEventBus {
    inner: Arc<TestEventBusInner>,
}

struct TestEventBusInner {
    /// Broadcast channel for all messages.
    broadcast_tx: broadcast::Sender<Arc<Envelope>>,
    /// All published messages (for inspection).
    messages: RwLock<Vec<Arc<Envelope>>>,
    /// Client subscriptions (client_id -> topics).
    subscriptions: RwLock<HashMap<String, Vec<String>>>,
    /// Client pattern subscriptions (client_id -> patterns).
    patterns: RwLock<HashMap<String, Vec<String>>>,
}

impl TestEventBus {
    /// Create a new test event bus.
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            inner: Arc::new(TestEventBusInner {
                broadcast_tx,
                messages: RwLock::new(Vec::new()),
                subscriptions: RwLock::new(HashMap::new()),
                patterns: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Create a test client connected to this bus.
    pub fn create_client(&self, id: impl Into<String>) -> TestClient {
        let id = id.into();
        debug!("Creating test client: {}", id);
        TestClient::new(id, self.inner.clone())
    }

    /// Inject a message directly into the bus.
    pub async fn inject(&self, envelope: Envelope) {
        let envelope = Arc::new(envelope);
        self.inner.messages.write().push(envelope.clone());
        let _ = self.inner.broadcast_tx.send(envelope);
    }

    /// Get all messages published to a specific topic.
    pub fn messages_for_topic(&self, topic: &str) -> Vec<Envelope> {
        self.inner
            .messages
            .read()
            .iter()
            .filter(|e| e.topic == topic)
            .map(|e| (**e).clone())
            .collect()
    }

    /// Get all messages published.
    pub fn all_messages(&self) -> Vec<Envelope> {
        self.inner
            .messages
            .read()
            .iter()
            .map(|e| (**e).clone())
            .collect()
    }

    /// Clear all messages.
    pub fn clear(&self) {
        self.inner.messages.write().clear();
    }

    /// Get the number of messages published.
    pub fn message_count(&self) -> usize {
        self.inner.messages.read().len()
    }
}

impl Default for TestEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// A test client for the in-memory event bus.
pub struct TestClient {
    id: String,
    bus: Arc<TestEventBusInner>,
    rx: mpsc::Receiver<Arc<Envelope>>,
    #[allow(dead_code)] // Kept for potential future use (e.g., request-response patterns)
    tx: mpsc::Sender<Arc<Envelope>>,
}

impl TestClient {
    fn new(id: String, bus: Arc<TestEventBusInner>) -> Self {
        let (tx, rx) = mpsc::channel(100);

        // Subscribe to broadcast
        let broadcast_rx = bus.broadcast_tx.subscribe();
        let client_id = id.clone();
        let bus_clone = bus.clone();
        let tx_clone = tx.clone();

        // Spawn task to filter messages for this client
        tokio::spawn(async move {
            let mut broadcast_rx = broadcast_rx;
            while let Ok(envelope) = broadcast_rx.recv().await {
                if should_deliver(&bus_clone, &client_id, &envelope) {
                    if tx_clone.send(envelope).await.is_err() {
                        break;
                    }
                }
            }
        });

        Self { id, bus, rx, tx }
    }

    /// Get the client ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Subscribe to topics.
    pub async fn subscribe(&self, topics: &[&str]) -> Result<()> {
        let mut subs = self.bus.subscriptions.write();
        let client_topics = subs.entry(self.id.clone()).or_default();
        for topic in topics {
            if !client_topics.contains(&(*topic).to_string()) {
                client_topics.push((*topic).to_string());
            }
        }
        debug!("Client {} subscribed to {:?}", self.id, topics);
        Ok(())
    }

    /// Subscribe to patterns (with wildcards).
    pub async fn subscribe_patterns(&self, patterns: &[&str]) -> Result<()> {
        let mut pats = self.bus.patterns.write();
        let client_patterns = pats.entry(self.id.clone()).or_default();
        for pattern in patterns {
            if !client_patterns.contains(&(*pattern).to_string()) {
                client_patterns.push((*pattern).to_string());
            }
        }
        debug!("Client {} subscribed to patterns {:?}", self.id, patterns);
        Ok(())
    }

    /// Unsubscribe from topics.
    pub async fn unsubscribe(&self, topics: &[&str]) -> Result<()> {
        let mut subs = self.bus.subscriptions.write();
        if let Some(client_topics) = subs.get_mut(&self.id) {
            for topic in topics {
                client_topics.retain(|t| t != *topic);
            }
        }
        Ok(())
    }

    /// Publish a message.
    pub async fn publish<M: Message>(&self, topic: &str, payload: M) -> Result<()> {
        let envelope = yama_protocol::envelope(topic, &self.id, payload);
        let envelope = Arc::new(envelope);
        self.bus.messages.write().push(envelope.clone());
        let _ = self.bus.broadcast_tx.send(envelope);
        trace!("Client {} published to {}", self.id, topic);
        Ok(())
    }

    /// Send a message to a specific target.
    pub async fn send<M: Message>(&self, topic: &str, target: &str, payload: M) -> Result<()> {
        let mut envelope = yama_protocol::envelope(topic, &self.id, payload);
        envelope.target = target.to_string();
        let envelope = Arc::new(envelope);
        self.bus.messages.write().push(envelope.clone());
        let _ = self.bus.broadcast_tx.send(envelope);
        trace!("Client {} sent to {} via {}", self.id, target, topic);
        Ok(())
    }

    /// Receive the next message.
    pub async fn recv(&mut self) -> Option<Envelope> {
        self.rx.recv().await.map(|e| (*e).clone())
    }

    /// Receive with timeout.
    pub async fn recv_timeout(&mut self, timeout_duration: Duration) -> Option<Envelope> {
        match tokio::time::timeout(timeout_duration, self.rx.recv()).await {
            Ok(Some(e)) => Some((*e).clone()),
            _ => None,
        }
    }

    /// Try to receive without blocking.
    pub fn try_recv(&mut self) -> Option<Envelope> {
        self.rx.try_recv().ok().map(|e| (*e).clone())
    }
}

/// Check if a message should be delivered to a client.
fn should_deliver(bus: &TestEventBusInner, client_id: &str, envelope: &Envelope) -> bool {
    // Don't deliver to sender
    if envelope.source == client_id {
        return false;
    }

    // Check targeted delivery
    if !envelope.target.is_empty() {
        return envelope.target == client_id;
    }

    // Check exact topic match
    let subs = bus.subscriptions.read();
    if let Some(topics) = subs.get(client_id) {
        if topics.iter().any(|t| t == &envelope.topic) {
            return true;
        }
    }

    // Check pattern match
    let pats = bus.patterns.read();
    if let Some(patterns) = pats.get(client_id) {
        if patterns.iter().any(|p| matches_pattern(p, &envelope.topic)) {
            return true;
        }
    }

    false
}

/// Check if a topic matches a pattern.
///
/// Supports simple wildcards:
/// - `*` matches any single segment
/// - `video.*` matches `video.frame`, `video.decode`, etc.
fn matches_pattern(pattern: &str, topic: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('.').collect();
    let topic_parts: Vec<&str> = topic.split('.').collect();

    if pattern_parts.len() != topic_parts.len() {
        // Check if pattern ends with * (matches rest)
        if pattern_parts.last() == Some(&"*") && topic_parts.len() >= pattern_parts.len() - 1 {
            for (i, p) in pattern_parts.iter().enumerate() {
                if *p == "*" {
                    return true;
                }
                if topic_parts.get(i) != Some(p) {
                    return false;
                }
            }
            return true;
        }
        return false;
    }

    for (p, t) in pattern_parts.iter().zip(topic_parts.iter()) {
        if *p != "*" && p != t {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use yama_protocol::common::HealthCheckRequest;

    #[tokio::test]
    async fn test_basic_pubsub() {
        let bus = TestEventBus::new();
        let client1 = bus.create_client("service-1");
        let mut client2 = bus.create_client("service-2");

        client2.subscribe(&["test.topic"]).await.unwrap();

        // Small delay to ensure subscription is set up
        tokio::time::sleep(Duration::from_millis(10)).await;

        client1
            .publish(
                "test.topic",
                HealthCheckRequest {
                    service_id: "test".to_string(),
                },
            )
            .await
            .unwrap();

        let event = client2.recv_timeout(Duration::from_secs(1)).await;
        assert!(event.is_some());
        assert_eq!(event.unwrap().topic, "test.topic");
    }

    #[tokio::test]
    async fn test_pattern_subscription() {
        let bus = TestEventBus::new();
        let client1 = bus.create_client("publisher");
        let mut client2 = bus.create_client("subscriber");

        client2.subscribe_patterns(&["video.*"]).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        client1
            .publish(
                "video.frame",
                HealthCheckRequest {
                    service_id: "test".to_string(),
                },
            )
            .await
            .unwrap();

        let event = client2.recv_timeout(Duration::from_secs(1)).await;
        assert!(event.is_some());
    }

    #[tokio::test]
    async fn test_targeted_delivery() {
        let bus = TestEventBus::new();
        let client1 = bus.create_client("sender");
        let mut client2 = bus.create_client("target");
        let mut client3 = bus.create_client("other");

        client2.subscribe(&["test.topic"]).await.unwrap();
        client3.subscribe(&["test.topic"]).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Send to specific target
        client1
            .send(
                "test.topic",
                "target",
                HealthCheckRequest {
                    service_id: "test".to_string(),
                },
            )
            .await
            .unwrap();

        // Target should receive
        let event = client2.recv_timeout(Duration::from_millis(100)).await;
        assert!(event.is_some());

        // Other should not receive
        let event = client3.recv_timeout(Duration::from_millis(100)).await;
        assert!(event.is_none());
    }

    #[tokio::test]
    async fn test_message_inspection() {
        let bus = TestEventBus::new();
        let client1 = bus.create_client("publisher");

        client1
            .publish(
                "test.a",
                HealthCheckRequest {
                    service_id: "a".to_string(),
                },
            )
            .await
            .unwrap();

        client1
            .publish(
                "test.b",
                HealthCheckRequest {
                    service_id: "b".to_string(),
                },
            )
            .await
            .unwrap();

        assert_eq!(bus.message_count(), 2);
        assert_eq!(bus.messages_for_topic("test.a").len(), 1);
        assert_eq!(bus.messages_for_topic("test.b").len(), 1);
    }

    #[test]
    fn test_pattern_matching() {
        assert!(matches_pattern("video.*", "video.frame"));
        assert!(matches_pattern("video.*", "video.decode"));
        assert!(!matches_pattern("video.*", "audio.frame"));
        assert!(matches_pattern("*.frame", "video.frame"));
        assert!(matches_pattern("video.frame", "video.frame"));
        assert!(!matches_pattern("video.frame", "video.decode"));
    }
}
