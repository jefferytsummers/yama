//! Yama Event Bus Server
//!
//! WebSocket-based pub/sub server for inter-service communication.
//! Supports topic-based routing and pattern matching.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use yama_protocol::common::{Envelope, Subscribe, Unsubscribe};

/// A connected client.
struct Client {
    id: String,
    tx: mpsc::Sender<Envelope>,
    subscriptions: HashSet<String>,
    patterns: HashSet<String>,
}

/// The event bus server.
struct EventBus {
    bind_addr: String,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    broadcast_tx: broadcast::Sender<Envelope>,
}

impl EventBus {
    /// Create a new event bus.
    fn new(bind_addr: String) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);

        Self {
            bind_addr,
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }

    /// Run the event bus server.
    async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.bind_addr)
            .await
            .context("Failed to bind WebSocket server")?;

        info!("Event bus listening on {}", self.bind_addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New connection from {}", addr);

                    let clients = self.clients.clone();
                    let broadcast_tx = self.broadcast_tx.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, addr.to_string(), clients, broadcast_tx).await
                        {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

/// Handle a WebSocket connection.
async fn handle_connection(
    stream: tokio::net::TcpStream,
    client_id: String,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    broadcast_tx: broadcast::Sender<Envelope>,
) -> Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("WebSocket handshake failed")?;

    let (mut ws_tx, mut ws_rx) = ws_stream.split();
    let (client_tx, mut client_rx) = mpsc::channel::<Envelope>(100);

    // Register client
    {
        let mut clients = clients.write().await;
        clients.insert(
            client_id.clone(),
            Client {
                id: client_id.clone(),
                tx: client_tx,
                subscriptions: HashSet::new(),
                patterns: HashSet::new(),
            },
        );
    }

    let mut broadcast_rx = broadcast_tx.subscribe();
    let clients_for_broadcast = clients.clone();

    loop {
        tokio::select! {
            // Handle incoming messages
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(WsMessage::Binary(data))) => {
                        if let Ok(envelope) = Envelope::decode(data.as_ref()) {
                            handle_message(&envelope, &client_id, &clients, &broadcast_tx).await?;
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => {
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Handle outgoing messages to this client
            msg = client_rx.recv() => {
                if let Some(envelope) = msg {
                    let mut buf = Vec::new();
                    envelope.encode(&mut buf)?;
                    if let Err(e) = ws_tx.send(WsMessage::Binary(buf.into())).await {
                        error!("Failed to send message: {}", e);
                        break;
                    }
                }
            }

            // Handle broadcast messages
            msg = broadcast_rx.recv() => {
                if let Ok(envelope) = msg {
                    let clients = clients_for_broadcast.read().await;
                    if let Some(client) = clients.get(&client_id) {
                        if should_deliver(&envelope, client) {
                            let mut buf = Vec::new();
                            envelope.encode(&mut buf)?;
                            if let Err(e) = ws_tx.send(WsMessage::Binary(buf.into())).await {
                                error!("Failed to send broadcast: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Unregister client
    {
        let mut clients = clients.write().await;
        clients.remove(&client_id);
    }

    info!("Client {} disconnected", client_id);
    Ok(())
}

/// Handle an incoming message.
async fn handle_message(
    envelope: &Envelope,
    client_id: &str,
    clients: &Arc<RwLock<HashMap<String, Client>>>,
    broadcast_tx: &broadcast::Sender<Envelope>,
) -> Result<()> {
    match envelope.topic.as_str() {
        "system.subscribe" => {
            if let Some(payload) = &envelope.payload {
                if let Ok(sub) = Subscribe::decode(payload.value.as_slice()) {
                    let mut clients = clients.write().await;
                    if let Some(client) = clients.get_mut(client_id) {
                        for topic in sub.topics {
                            debug!("Client {} subscribed to {}", client_id, topic);
                            client.subscriptions.insert(topic);
                        }
                        for pattern in sub.patterns {
                            debug!("Client {} subscribed to pattern {}", client_id, pattern);
                            client.patterns.insert(pattern);
                        }
                    }
                }
            }
        }
        "system.unsubscribe" => {
            if let Some(payload) = &envelope.payload {
                if let Ok(unsub) = Unsubscribe::decode(payload.value.as_slice()) {
                    let mut clients = clients.write().await;
                    if let Some(client) = clients.get_mut(client_id) {
                        for topic in unsub.topics {
                            client.subscriptions.remove(&topic);
                        }
                        for pattern in unsub.patterns {
                            client.patterns.remove(&pattern);
                        }
                    }
                }
            }
        }
        _ => {
            // Forward to subscribers via broadcast
            debug!("Broadcasting message on topic {}", envelope.topic);
            let _ = broadcast_tx.send(envelope.clone());
        }
    }

    Ok(())
}

/// Check if a message should be delivered to a client.
fn should_deliver(envelope: &Envelope, client: &Client) -> bool {
    if !envelope.target.is_empty() && envelope.target != client.id {
        return false;
    }

    client.subscriptions.contains(&envelope.topic)
        || matches_pattern(&envelope.topic, &client.patterns)
}

/// Check if a topic matches any pattern.
fn matches_pattern(topic: &str, patterns: &HashSet<String>) -> bool {
    for pattern in patterns {
        if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            if topic.starts_with(prefix) {
                return true;
            }
        } else if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            if topic.starts_with(prefix) {
                return true;
            }
        } else if pattern == topic {
            return true;
        }
    }
    false
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .init();

    info!("Starting Yama Event Bus");

    let bind_addr = std::env::var("EVENT_BUS_BIND").unwrap_or_else(|_| "0.0.0.0:8765".to_string());

    let event_bus = EventBus::new(bind_addr);
    event_bus.run().await
}
