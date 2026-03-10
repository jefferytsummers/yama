//! Event bus implementation for inter-service communication.
//!
//! The event bus provides:
//! - WebSocket server for external clients
//! - Unix socket server for container IPC
//! - Topic-based pub/sub routing
//! - Protocol Buffer message serialization
//!
//! Uses DashMap for lock-free concurrent client management.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use tokio::net::{TcpListener, UnixListener};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use yama_protocol::common::{Envelope, Subscribe, Unsubscribe};

/// Event bus configuration.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventBusConfig {
    /// WebSocket bind address.
    #[serde(default = "default_ws_bind")]
    pub websocket_bind: String,
    /// Unix socket path.
    #[serde(default = "default_unix_socket")]
    pub unix_socket: PathBuf,
    /// Maximum message size.
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,
}

fn default_ws_bind() -> String {
    "0.0.0.0:8765".to_string()
}

fn default_unix_socket() -> PathBuf {
    PathBuf::from("/tmp/yama-event.sock")
}

fn default_max_message_size() -> usize {
    16 * 1024 * 1024 // 16MB
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            websocket_bind: default_ws_bind(),
            unix_socket: default_unix_socket(),
            max_message_size: default_max_message_size(),
        }
    }
}

/// A connected client.
struct Client {
    id: String,
    tx: mpsc::Sender<Envelope>,
    subscriptions: HashSet<String>,
    patterns: HashSet<String>,
}

/// Type alias for the client registry using DashMap for lock-free access.
type ClientRegistry = Arc<DashMap<String, Client>>;

/// The event bus server.
pub struct EventBus {
    config: EventBusConfig,
    clients: ClientRegistry,
    broadcast_tx: broadcast::Sender<Envelope>,
    shutdown_tx: mpsc::Sender<()>,
    #[allow(dead_code)]
    shutdown_rx: mpsc::Receiver<()>,
}

impl EventBus {
    /// Create a new event bus.
    pub async fn new(config: EventBusConfig) -> Result<Self> {
        let (broadcast_tx, _) = broadcast::channel(1000);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Ok(Self {
            config,
            clients: Arc::new(DashMap::new()),
            broadcast_tx,
            shutdown_tx,
            shutdown_rx,
        })
    }

    /// Run the event bus servers.
    pub async fn run(&self) -> Result<()> {
        // Start WebSocket server
        let ws_handle = self.start_websocket_server().await?;

        // Start Unix socket server
        let unix_handle = self.start_unix_server().await?;

        // Wait for shutdown or server errors
        tokio::select! {
            result = ws_handle => {
                if let Err(e) = result {
                    error!("WebSocket server error: {}", e);
                }
            }
            result = unix_handle => {
                if let Err(e) = result {
                    error!("Unix socket server error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Start the WebSocket server.
    async fn start_websocket_server(
        &self,
    ) -> Result<tokio::task::JoinHandle<Result<()>>> {
        let listener = TcpListener::bind(&self.config.websocket_bind)
            .await
            .context("Failed to bind WebSocket server")?;

        info!("WebSocket server listening on {}", self.config.websocket_bind);

        let clients = self.clients.clone();
        let broadcast_tx = self.broadcast_tx.clone();

        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        info!("New WebSocket connection from {}", addr);

                        let clients = clients.clone();
                        let broadcast_tx = broadcast_tx.clone();

                        tokio::spawn(async move {
                            if let Err(e) =
                                handle_websocket_connection(stream, addr.to_string(), clients, broadcast_tx)
                                    .await
                            {
                                error!("WebSocket connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
        });

        Ok(handle)
    }

    /// Start the Unix socket server.
    async fn start_unix_server(&self) -> Result<tokio::task::JoinHandle<Result<()>>> {
        // Remove existing socket file
        if self.config.unix_socket.exists() {
            std::fs::remove_file(&self.config.unix_socket)?;
        }

        let listener = UnixListener::bind(&self.config.unix_socket)
            .context("Failed to bind Unix socket server")?;

        info!("Unix socket server listening on {:?}", self.config.unix_socket);

        let clients = self.clients.clone();
        let broadcast_tx = self.broadcast_tx.clone();

        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        let client_id = format!("unix-{}", Uuid::new_v4());
                        info!("New Unix socket connection: {}", client_id);

                        let clients = clients.clone();
                        let broadcast_tx = broadcast_tx.clone();

                        tokio::spawn(async move {
                            if let Err(e) =
                                handle_unix_connection(stream, client_id, clients, broadcast_tx).await
                            {
                                error!("Unix socket connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept Unix connection: {}", e);
                    }
                }
            }
        });

        Ok(handle)
    }

    /// Publish a message to the event bus.
    pub async fn publish(&self, envelope: Envelope) -> Result<()> {
        let topic = envelope.topic.clone();
        let target = envelope.target.clone();

        if target.is_empty() {
            // Broadcast to all matching subscribers using DashMap iteration
            for client_ref in self.clients.iter() {
                let client = client_ref.value();
                if client.subscriptions.contains(&topic) || matches_pattern(&topic, &client.patterns)
                {
                    if let Err(e) = client.tx.send(envelope.clone()).await {
                        warn!("Failed to send to client {}: {}", client.id, e);
                    }
                }
            }
        } else {
            // Send to specific target
            if let Some(client_ref) = self.clients.get(&target) {
                if let Err(e) = client_ref.tx.send(envelope).await {
                    warn!("Failed to send to client {}: {}", target, e);
                }
            }
        }

        Ok(())
    }

    /// Shutdown the event bus.
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down event bus");
        let _ = self.shutdown_tx.send(()).await;
        Ok(())
    }
}

/// Handle a WebSocket connection.
async fn handle_websocket_connection(
    stream: tokio::net::TcpStream,
    client_id: String,
    clients: ClientRegistry,
    broadcast_tx: broadcast::Sender<Envelope>,
) -> Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("WebSocket handshake failed")?;

    let (mut ws_tx, mut ws_rx) = ws_stream.split();
    let (client_tx, mut client_rx) = mpsc::channel::<Envelope>(100);

    // Register client using DashMap (no write lock needed)
    clients.insert(
        client_id.clone(),
        Client {
            id: client_id.clone(),
            tx: client_tx,
            subscriptions: HashSet::new(),
            patterns: HashSet::new(),
        },
    );

    let mut broadcast_rx = broadcast_tx.subscribe();

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
                        error!("Failed to send WebSocket message: {}", e);
                        break;
                    }
                }
            }

            // Handle broadcast messages
            msg = broadcast_rx.recv() => {
                match msg {
                    Ok(envelope) => {
                        // Check delivery using DashMap get (no lock)
                        if let Some(client_ref) = clients.get(&client_id) {
                            if should_deliver(&envelope, client_ref.value()) {
                                debug!("EVENT BUS: Delivering '{}' to client '{}'", envelope.topic, client_id);
                                let mut buf = Vec::new();
                                envelope.encode(&mut buf)?;
                                if let Err(e) = ws_tx.send(WsMessage::Binary(buf.into())).await {
                                    error!("Failed to send broadcast message: {}", e);
                                    break;
                                }
                            }
                        } else {
                            warn!("EVENT BUS: Client {} not found in registry for broadcast", client_id);
                        }
                    }
                    Err(e) => {
                        warn!("EVENT BUS: Broadcast recv error for {}: {}", client_id, e);
                    }
                }
            }
        }
    }

    // Unregister client using DashMap (no write lock needed)
    clients.remove(&client_id);

    info!("Client {} disconnected", client_id);
    Ok(())
}

/// Handle a Unix socket connection.
async fn handle_unix_connection(
    stream: tokio::net::UnixStream,
    client_id: String,
    clients: ClientRegistry,
    broadcast_tx: broadcast::Sender<Envelope>,
) -> Result<()> {
    // Use accept_async to accept the incoming WebSocket connection
    // (client sends HTTP upgrade request, server responds)
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("Unix socket WebSocket handshake failed")?;

    let (mut ws_tx, mut ws_rx) = ws_stream.split();
    let (client_tx, mut client_rx) = mpsc::channel::<Envelope>(100);

    // Register client using DashMap (no write lock needed)
    clients.insert(
        client_id.clone(),
        Client {
            id: client_id.clone(),
            tx: client_tx,
            subscriptions: HashSet::new(),
            patterns: HashSet::new(),
        },
    );

    let mut broadcast_rx = broadcast_tx.subscribe();

    loop {
        tokio::select! {
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
                    _ => {}
                }
            }

            msg = client_rx.recv() => {
                if let Some(envelope) = msg {
                    let mut buf = Vec::new();
                    envelope.encode(&mut buf)?;
                    if ws_tx.send(WsMessage::Binary(buf.into())).await.is_err() {
                        break;
                    }
                }
            }

            msg = broadcast_rx.recv() => {
                match msg {
                    Ok(envelope) => {
                        // Check delivery using DashMap get (no lock)
                        if let Some(client_ref) = clients.get(&client_id) {
                            if should_deliver(&envelope, client_ref.value()) {
                                debug!("EVENT BUS: Delivering '{}' to Unix client '{}'", envelope.topic, client_id);
                                let mut buf = Vec::new();
                                envelope.encode(&mut buf)?;
                                if ws_tx.send(WsMessage::Binary(buf.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("EVENT BUS: Unix broadcast recv error for {}: {}", client_id, e);
                    }
                }
            }
        }
    }

    // Unregister client using DashMap (no write lock needed)
    clients.remove(&client_id);

    info!("Unix client {} disconnected", client_id);
    Ok(())
}

/// Handle an incoming message and route it to subscribers.
async fn handle_message(
    envelope: &Envelope,
    client_id: &str,
    clients: &ClientRegistry,
    broadcast_tx: &broadcast::Sender<Envelope>,
) -> Result<()> {
    match envelope.topic.as_str() {
        "system.subscribe" => {
            if let Some(payload) = &envelope.payload {
                if let Ok(sub) = Subscribe::decode(payload.value.as_slice()) {
                    // Use DashMap get_mut for in-place modification
                    if let Some(mut client_ref) = clients.get_mut(client_id) {
                        let client = client_ref.value_mut();
                        for topic in sub.topics {
                            info!("Client {} subscribed to topic: {}", client_id, topic);
                            client.subscriptions.insert(topic);
                        }
                        for pattern in sub.patterns {
                            info!("Client {} subscribed to pattern: {}", client_id, pattern);
                            client.patterns.insert(pattern);
                        }
                    }
                }
            }
        }
        "system.unsubscribe" => {
            if let Some(payload) = &envelope.payload {
                if let Ok(unsub) = Unsubscribe::decode(payload.value.as_slice()) {
                    // Use DashMap get_mut for in-place modification
                    if let Some(mut client_ref) = clients.get_mut(client_id) {
                        let client = client_ref.value_mut();
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
            // Forward message to all matching subscribers via broadcast channel
            debug!("EVENT BUS: Broadcasting message on topic '{}' from client '{}'", envelope.topic, client_id);
            match broadcast_tx.send(envelope.clone()) {
                Ok(receiver_count) => {
                    debug!("EVENT BUS: Message broadcast to {} receivers", receiver_count);
                }
                Err(e) => {
                    warn!("EVENT BUS: Failed to broadcast message on topic {}: {}", envelope.topic, e);
                }
            }
        }
    }

    Ok(())
}

/// Check if a message should be delivered to a client.
fn should_deliver(envelope: &Envelope, client: &Client) -> bool {
    if !envelope.target.is_empty() && envelope.target != client.id {
        return false;
    }

    let matches_topic = client.subscriptions.contains(&envelope.topic);
    let matches_patterns = matches_pattern(&envelope.topic, &client.patterns);

    matches_topic || matches_patterns
}

/// Check if a topic matches any pattern.
fn matches_pattern(topic: &str, patterns: &HashSet<String>) -> bool {
    for pattern in patterns {
        if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            if topic.starts_with(prefix) {
                return true;
            }
        } else if pattern.ends_with("*") {
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
