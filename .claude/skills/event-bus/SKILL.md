---
name: event-bus
description: WebSocket pub/sub, Protocol Buffers, and inter-service communication
compatibility:
  - platform/apple/host/src/event_bus/
  - platform/jetson/host/src/event_bus/
  - shared/protocol/
  - shared/container-sdk/
---

# Event Bus IPC

Binary Protobuf pub/sub over WebSocket and Unix socket.

## Endpoints

| Endpoint | Protocol | Purpose |
|----------|----------|---------|
| ws://localhost:8765 | WebSocket | External clients, web UI |
| /tmp/yama-event.sock | Unix Socket | Container IPC (zero-copy) |

## Architecture

```
┌─────────────────────────────────────────┐
│  Event Bus Server (Tokio)               │
├─────────────────────────────────────────┤
│  WebSocket Server  │  Unix Socket Server│
├─────────────────────────────────────────┤
│  Topic Router + Pattern Matching        │
├─────────────────────────────────────────┤
│  Client Registry + Subscriptions        │
└─────────────────────────────────────────┘
```

## Protocol

All messages use the `Envelope` wrapper:

```protobuf
message Envelope {
    string message_id = 1;
    string topic = 2;
    string source = 3;
    string target = 4;        // Empty = broadcast
    Timestamp timestamp = 5;
    string correlation_id = 6;
    google.protobuf.Any payload = 7;
    int32 priority = 8;
    int64 ttl_ms = 9;
}
```

## Key Files

- `platform/{apple,jetson}/host/src/event_bus/mod.rs` - Server implementation
- `shared/protocol/src/lib.rs` - Protocol definitions
- `shared/protocol/proto/*.proto` - Protobuf schemas
- `shared/container-sdk/src/client.rs` - Client SDK

## Topic Naming

```
{domain}.{entity}.{action}

Examples:
- video.frame.decoded
- agent.request.create
- system.service.started
- system.health.check
```

### System Topics

| Topic | Direction | Description |
|-------|-----------|-------------|
| system.subscribe | Client→Server | Subscribe to topics |
| system.unsubscribe | Client→Server | Unsubscribe |
| system.health.request | Server→Client | Health check |
| system.health.response | Client→Server | Health response |

### Agent Topics

| Topic | Direction | Description |
|-------|-----------|-------------|
| agent.request | Client→Server | Conversation request |
| agent.response | Server→Client | Agent response |
| agent.tool.call | Server→Client | Tool execution |
| agent.tool.result | Client→Server | Tool result |

### Video Topics

| Topic | Direction | Description |
|-------|-----------|-------------|
| video.frame.raw | Producer→Bus | Raw frame |
| video.frame.decoded | Decoder→Bus | Decoded frame |
| video.detection | Inference→Bus | Detection results |

## Subscription

### Subscribe Message

```protobuf
message Subscribe {
    repeated string topics = 1;    // Exact topics
    repeated string patterns = 2;  // Wildcards: "video.*"
}
```

### Pattern Matching

- `video.*` - All video topics
- `system.*` - All system topics
- `agent.request` - Exact match

## Client Implementation

```rust
use yama_container_sdk::EventBusClient;

// Connect
let client = EventBusClient::connect("/tmp/yama-event.sock").await?;

// Subscribe
client.subscribe(&["video.frame.*", "agent.request"]).await?;

// Publish
let envelope = yama_protocol::envelope(
    "video.frame.decoded",
    "video-decoder",
    frame_message,
);
client.publish(envelope).await?;

// Receive
while let Some(envelope) = client.recv().await {
    match envelope.topic.as_str() {
        "agent.request" => handle_agent_request(&envelope),
        topic if topic.starts_with("video.") => handle_video(&envelope),
        _ => {}
    }
}
```

## Server Implementation

```rust
pub struct EventBus {
    config: EventBusConfig,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    broadcast_tx: broadcast::Sender<Envelope>,
}

impl EventBus {
    pub async fn new(config: EventBusConfig) -> Result<Self>;
    pub async fn run(&self) -> Result<()>;
    pub async fn publish(&self, envelope: Envelope) -> Result<()>;
    pub async fn shutdown(&self) -> Result<()>;
}
```

## Configuration

```toml
[event_bus]
websocket_bind = "0.0.0.0:8765"
unix_socket = "/tmp/yama-event.sock"
max_message_size = 16777216  # 16MB
```

## Encoding/Decoding

```rust
use yama_protocol::{encode, decode, envelope};

// Encode
let bytes = encode(&message)?;

// Decode
let message: MyMessage = decode(&bytes)?;

// Create envelope
let env = envelope("my.topic", "my-service", payload_message);
```

## Health Checks

```rust
use yama_container_sdk::HealthReporter;

let health = HealthReporter::new(&client, "my-service");
health.start().await?;

// Auto-responds to system.health.request
```

## WebSocket Handler

```rust
async fn handle_websocket_connection(
    stream: TcpStream,
    client_id: String,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    broadcast_tx: broadcast::Sender<Envelope>,
) -> Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (ws_tx, ws_rx) = ws_stream.split();

    // Message loop
    tokio::select! {
        msg = ws_rx.next() => { /* handle incoming */ }
        msg = client_rx.recv() => { /* handle outgoing */ }
        msg = broadcast_rx.recv() => { /* handle broadcast */ }
    }
}
```

## Dependencies

```toml
tokio-tungstenite = "0.24"
prost = "0.13"
prost-types = "0.13"
```
