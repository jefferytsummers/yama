//! Event bus client for container communication.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use tokio::net::UnixStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;
use tracing::{debug, error, info, instrument, warn};

use yama_protocol::common::{Envelope, Subscribe, Unsubscribe};

/// Event received from the event bus.
#[derive(Debug, Clone)]
pub struct Event {
    /// The envelope containing the message.
    pub envelope: Envelope,
    /// Raw payload bytes.
    pub payload: Bytes,
}

/// Client for communicating with the Yama event bus.
pub struct EventBusClient {
    /// Service identifier.
    service_id: String,
    /// Sender for outgoing messages.
    tx: mpsc::Sender<Envelope>,
    /// Receiver for incoming messages.
    rx: mpsc::Receiver<Event>,
    /// Active subscriptions.
    subscriptions: Arc<RwLock<HashSet<String>>>,
}

impl EventBusClient {
    /// Connect to the event bus via Unix socket.
    #[instrument(skip_all, fields(socket_path = %socket_path.as_ref().display()))]
    pub async fn connect(
        socket_path: impl AsRef<Path>,
        service_id: impl Into<String>,
    ) -> Result<Self> {
        let service_id = service_id.into();
        info!("Connecting to event bus");

        let stream = UnixStream::connect(socket_path.as_ref())
            .await
            .context("Failed to connect to Unix socket")?;

        let ws_stream = tokio_tungstenite::client_async("ws://localhost/", stream)
            .await
            .context("WebSocket handshake failed")?
            .0;

        Self::from_websocket(ws_stream, service_id).await
    }

    /// Connect to the event bus via WebSocket URL.
    #[instrument(skip_all, fields(url = %url))]
    pub async fn connect_ws(url: &str, service_id: impl Into<String>) -> Result<Self> {
        let service_id = service_id.into();
        info!("Connecting to event bus via WebSocket");

        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .context("Failed to connect to WebSocket")?;

        Self::from_websocket(ws_stream, service_id).await
    }

    /// Create client from an existing WebSocket stream.
    async fn from_websocket<S>(ws_stream: WebSocketStream<S>, service_id: String) -> Result<Self>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (mut ws_tx, mut ws_rx) = ws_stream.split();

        // Channel for outgoing messages
        let (tx, mut outgoing_rx) = mpsc::channel::<Envelope>(100);
        // Channel for incoming messages
        let (incoming_tx, rx) = mpsc::channel::<Event>(100);

        let subscriptions = Arc::new(RwLock::new(HashSet::new()));

        // Spawn task to handle outgoing messages
        tokio::spawn(async move {
            while let Some(envelope) = outgoing_rx.recv().await {
                let mut buf = Vec::new();
                if envelope.encode(&mut buf).is_ok() {
                    if let Err(e) = ws_tx.send(WsMessage::Binary(buf.into())).await {
                        error!("Failed to send message: {}", e);
                        break;
                    }
                }
            }
        });

        // Spawn task to handle incoming messages
        tokio::spawn(async move {
            while let Some(msg_result) = ws_rx.next().await {
                match msg_result {
                    Ok(WsMessage::Binary(data)) => {
                        match Envelope::decode(data.as_ref()) {
                            Ok(envelope) => {
                                let payload = envelope
                                    .payload
                                    .as_ref()
                                    .map(|p| Bytes::from(p.value.clone()))
                                    .unwrap_or_default();

                                let event = Event { envelope, payload };
                                if incoming_tx.send(event).await.is_err() {
                                    debug!("Receiver dropped, stopping");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to decode envelope: {}", e);
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        info!("WebSocket closed");
                        break;
                    }
                    Ok(_) => {
                        // Ignore other message types
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            service_id,
            tx,
            rx,
            subscriptions,
        })
    }

    /// Subscribe to topics.
    #[instrument(skip(self), fields(service_id = %self.service_id))]
    pub async fn subscribe(&self, topics: &[&str]) -> Result<()> {
        let topics: Vec<String> = topics.iter().map(|s| (*s).to_string()).collect();
        debug!("Subscribing to topics: {:?}", topics);

        let subscribe = Subscribe {
            subscriber_id: self.service_id.clone(),
            topics: topics.clone(),
            patterns: Vec::new(),
        };

        let envelope = yama_protocol::envelope("system.subscribe", &self.service_id, subscribe);
        self.tx
            .send(envelope)
            .await
            .context("Failed to send subscribe")?;

        let mut subs = self.subscriptions.write().await;
        for topic in topics {
            subs.insert(topic);
        }

        Ok(())
    }

    /// Subscribe to topic patterns (with wildcards).
    #[instrument(skip(self), fields(service_id = %self.service_id))]
    pub async fn subscribe_patterns(&self, patterns: &[&str]) -> Result<()> {
        let patterns: Vec<String> = patterns.iter().map(|s| (*s).to_string()).collect();
        debug!("Subscribing to patterns: {:?}", patterns);

        let subscribe = Subscribe {
            subscriber_id: self.service_id.clone(),
            topics: Vec::new(),
            patterns: patterns.clone(),
        };

        let envelope = yama_protocol::envelope("system.subscribe", &self.service_id, subscribe);
        self.tx
            .send(envelope)
            .await
            .context("Failed to send subscribe")?;

        let mut subs = self.subscriptions.write().await;
        for pattern in patterns {
            subs.insert(pattern);
        }

        Ok(())
    }

    /// Unsubscribe from topics.
    #[instrument(skip(self), fields(service_id = %self.service_id))]
    pub async fn unsubscribe(&self, topics: &[&str]) -> Result<()> {
        let topics: Vec<String> = topics.iter().map(|s| (*s).to_string()).collect();
        debug!("Unsubscribing from topics: {:?}", topics);

        let unsubscribe = Unsubscribe {
            subscriber_id: self.service_id.clone(),
            topics: topics.clone(),
            patterns: Vec::new(),
        };

        let envelope = yama_protocol::envelope("system.unsubscribe", &self.service_id, unsubscribe);
        self.tx
            .send(envelope)
            .await
            .context("Failed to send unsubscribe")?;

        let mut subs = self.subscriptions.write().await;
        for topic in &topics {
            subs.remove(topic);
        }

        Ok(())
    }

    /// Publish a message to a topic.
    #[instrument(skip(self, payload), fields(service_id = %self.service_id, topic = %topic))]
    pub async fn publish<M: Message>(&self, topic: &str, payload: M) -> Result<()> {
        debug!("Publishing to topic");

        let envelope = yama_protocol::envelope(topic, &self.service_id, payload);
        self.tx
            .send(envelope)
            .await
            .context("Failed to send message")?;

        Ok(())
    }

    /// Publish a message to a specific target.
    #[instrument(skip(self, payload), fields(service_id = %self.service_id, topic = %topic, target = %target))]
    pub async fn send<M: Message>(&self, topic: &str, target: &str, payload: M) -> Result<()> {
        debug!("Sending to target");

        let mut envelope = yama_protocol::envelope(topic, &self.service_id, payload);
        envelope.target = target.to_string();
        self.tx
            .send(envelope)
            .await
            .context("Failed to send message")?;

        Ok(())
    }

    /// Receive the next event.
    pub async fn recv(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    /// Get the service ID.
    pub fn service_id(&self) -> &str {
        &self.service_id
    }

    /// Get active subscriptions.
    pub async fn subscriptions(&self) -> Vec<String> {
        self.subscriptions.read().await.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would require a running event bus server
    // Unit tests for message encoding/decoding

    #[test]
    fn test_envelope_encoding() {
        let envelope = yama_protocol::envelope(
            "test.topic",
            "test-service",
            yama_protocol::common::HealthCheckRequest {
                service_id: "test".to_string(),
            },
        );

        let encoded = yama_protocol::encode(&envelope).expect("should encode");
        let decoded: Envelope = yama_protocol::decode(&encoded).expect("should decode");

        assert_eq!(decoded.topic, "test.topic");
        assert_eq!(decoded.source, "test-service");
    }
}
