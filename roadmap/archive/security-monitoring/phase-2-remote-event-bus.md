# Phase 2: Remote Event Bus & Authentication

**Duration:** 10 days
**Goal:** Enable authenticated remote connections between clients and Jetson.

---

## Overview

This phase extends the existing event bus to support authenticated remote connections. Mac clients will connect to a Jetson server over WebSocket, authenticating via shared secret (HMAC-SHA256 challenge-response). Once authenticated, clients can subscribe to video frames, detections, and VLM responses.

---

## Architecture

```
┌────────────────────────────┐         ┌────────────────────────────────┐
│  MAC (Thin Client)         │         │  JETSON (AI Server)            │
│                            │         │                                │
│  ┌─────────┐               │   WS    │  ┌────────────────────────┐   │
│  │ egui UI │◀─────────────────────────▶│ Remote Event Bus       │   │
│  └─────────┘               │         │  └──────────┬─────────────┘   │
│       │                    │         │             │                  │
│       ▼                    │         │             ▼                  │
│  ┌─────────────────┐       │ WebRTC  │  ┌────────────────────────┐   │
│  │ RemoteAIServer  │◀────────────────▶│ DeepStream + Triton    │   │
│  │ RemoteVideo     │       │         │  (Detection + VLM)       │   │
│  │ RemoteDetection │       │         │  └────────────────────────┘   │
│  └─────────────────┘       │         │                                │
└────────────────────────────┘         └────────────────────────────────┘
```

---

## Milestone 2.1: Event Bus Authentication

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 2.1.1 | Add `AuthChallenge` / `AuthResponse` proto | Authentication messages | Messages encode/decode |
| 2.1.2 | Implement HMAC-SHA256 challenge-response | Signature validation | Correct signature validation |
| 2.1.3 | Add auth middleware to WebSocket | Reject unauthenticated | Unauthenticated clients rejected |
| 2.1.4 | Add shared secret config | Read from file or env | Config loading works |

### Protocol: Authentication Messages

```protobuf
// shared/protocol/proto/remote.proto

syntax = "proto3";
package yama.remote;

import "google/protobuf/timestamp.proto";

// Server sends challenge to client
message AuthChallenge {
    bytes challenge = 1;  // Random 32 bytes from server
    uint64 timestamp = 2;
}

// Client responds with signed challenge
message AuthResponse {
    bytes signature = 1;  // HMAC-SHA256(challenge || timestamp, shared_secret)
    string client_id = 2;
    string client_type = 3;  // "mac", "web", "mobile"
}

// Server confirms or rejects
message AuthResult {
    bool success = 1;
    string error_message = 2;
    string session_id = 3;
}
```

### Code: Authentication Handler

```rust
// event_bus/auth.rs

use hmac::{Hmac, Mac};
use sha2::Sha256;
use rand::Rng;

type HmacSha256 = Hmac<Sha256>;

pub struct Authenticator {
    shared_secret: Vec<u8>,
}

impl Authenticator {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            shared_secret: secret.to_vec(),
        }
    }

    /// Generate a random challenge for a new connection.
    pub fn generate_challenge(&self) -> AuthChallenge {
        let mut challenge = [0u8; 32];
        rand::thread_rng().fill(&mut challenge);

        AuthChallenge {
            challenge: challenge.to_vec(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Verify client's response to challenge.
    pub fn verify(&self, challenge: &AuthChallenge, response: &AuthResponse) -> bool {
        // Compute expected signature
        let mut mac = HmacSha256::new_from_slice(&self.shared_secret)
            .expect("HMAC can take key of any size");
        mac.update(&challenge.challenge);
        mac.update(&challenge.timestamp.to_le_bytes());

        // Verify signature
        mac.verify_slice(&response.signature).is_ok()
    }
}
```

### Code: WebSocket Auth Middleware

```rust
// event_bus/ws_handler.rs

async fn handle_websocket_connection(
    stream: TcpStream,
    authenticator: Arc<Authenticator>,
    clients: Arc<RwLock<HashMap<String, Client>>>,
) -> Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Step 1: Send challenge
    let challenge = authenticator.generate_challenge();
    let challenge_msg = encode(&challenge)?;
    ws_tx.send(Message::Binary(challenge_msg)).await?;

    // Step 2: Wait for response (with timeout)
    let response_msg = tokio::time::timeout(
        Duration::from_secs(10),
        ws_rx.next()
    ).await??;

    let response: AuthResponse = match response_msg {
        Some(Message::Binary(data)) => decode(&data)?,
        _ => return Err(anyhow!("Invalid auth response")),
    };

    // Step 3: Verify
    if !authenticator.verify(&challenge, &response) {
        let result = AuthResult {
            success: false,
            error_message: "Authentication failed".to_string(),
            session_id: String::new(),
        };
        ws_tx.send(Message::Binary(encode(&result)?)).await?;
        return Err(anyhow!("Authentication failed"));
    }

    // Step 4: Create session
    let session_id = uuid::Uuid::new_v4().to_string();
    let result = AuthResult {
        success: true,
        error_message: String::new(),
        session_id: session_id.clone(),
    };
    ws_tx.send(Message::Binary(encode(&result)?)).await?;

    tracing::info!(
        client_id = %response.client_id,
        session_id = %session_id,
        "Client authenticated"
    );

    // Continue to normal message handling...
    handle_authenticated_client(session_id, response.client_id, ws_tx, ws_rx, clients).await
}
```

### Verification

```bash
# Unauthenticated - rejected
websocat ws://localhost:8765  # Disconnects

# Authenticated - succeeds
cargo run --example auth_client -- --secret "test123"
```

### Deliverable

`event_bus/auth.rs`

---

## Milestone 2.2: Remote Session Protocol

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 2.2.1 | Define session proto messages | `RemoteSessionRequest/Response` | Messages compile |
| 2.2.2 | Implement session manager | Track active sessions | Sessions tracked correctly |
| 2.2.3 | Add capability negotiation | Client declares capabilities | Capabilities negotiated |
| 2.2.4 | Implement session heartbeat | Stale sessions cleaned up | Cleanup works |

### Protocol: Session Messages

```protobuf
// shared/protocol/proto/remote.proto (continued)

// Remote client session management
message RemoteSessionRequest {
    string client_id = 1;
    string client_type = 2;  // "mac", "web", "mobile"
    repeated string capabilities = 3;  // "video", "detection", "vlm"
    AuthResponse auth = 4;
}

message RemoteSessionResponse {
    string session_id = 1;
    bool accepted = 2;
    VideoStreamConfig video_stream = 3;
    repeated string subscriptions = 4;  // Auto-subscribed topics
    ServerCapabilities server_caps = 5;
}

message VideoStreamConfig {
    oneof protocol {
        WebRtcOffer webrtc = 1;
        string hls_url = 2;
    }
    uint32 width = 3;
    uint32 height = 4;
    uint32 fps = 5;
}

message ServerCapabilities {
    repeated string available_models = 1;
    uint32 max_streams = 2;
    bool vlm_available = 3;
    bool detection_available = 4;
}

// Session heartbeat
message SessionHeartbeat {
    string session_id = 1;
    uint64 timestamp = 2;
}

message SessionHeartbeatAck {
    string session_id = 1;
    uint64 server_timestamp = 2;
}
```

### Code: Session Manager

```rust
// event_bus/session.rs

pub struct SessionManager {
    sessions: RwLock<HashMap<String, Session>>,
    heartbeat_timeout: Duration,
}

pub struct Session {
    pub session_id: String,
    pub client_id: String,
    pub client_type: String,
    pub capabilities: Vec<String>,
    pub subscriptions: Vec<String>,
    pub last_heartbeat: Instant,
    pub created_at: Instant,
}

impl SessionManager {
    pub fn new(heartbeat_timeout: Duration) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            heartbeat_timeout,
        }
    }

    pub async fn create_session(
        &self,
        request: RemoteSessionRequest,
    ) -> Result<RemoteSessionResponse> {
        let session_id = uuid::Uuid::new_v4().to_string();

        let session = Session {
            session_id: session_id.clone(),
            client_id: request.client_id,
            client_type: request.client_type,
            capabilities: request.capabilities.clone(),
            subscriptions: self.auto_subscriptions(&request.capabilities),
            last_heartbeat: Instant::now(),
            created_at: Instant::now(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        Ok(RemoteSessionResponse {
            session_id,
            accepted: true,
            video_stream: Some(self.configure_video_stream(&request)?),
            subscriptions: self.auto_subscriptions(&request.capabilities),
            server_caps: Some(self.server_capabilities()),
        })
    }

    pub async fn heartbeat(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_heartbeat = Instant::now();
            Ok(())
        } else {
            Err(anyhow!("Session not found"))
        }
    }

    pub async fn cleanup_stale(&self) {
        let mut sessions = self.sessions.write().await;
        let now = Instant::now();

        sessions.retain(|id, session| {
            let stale = now.duration_since(session.last_heartbeat) > self.heartbeat_timeout;
            if stale {
                tracing::info!(session_id = %id, "Cleaning up stale session");
            }
            !stale
        });
    }

    fn auto_subscriptions(&self, capabilities: &[String]) -> Vec<String> {
        let mut subs = vec!["system.*".to_string()];

        for cap in capabilities {
            match cap.as_str() {
                "video" => subs.push("video.frame.*".to_string()),
                "detection" => subs.push("detection.*".to_string()),
                "vlm" => subs.push("vlm.response.*".to_string()),
                _ => {}
            }
        }

        subs
    }
}
```

### Verification

```bash
cargo run --example session_client -- --url ws://localhost:8765
# Output: "Session established: session_id=abc123"
```

### Deliverable

`shared/protocol/proto/remote.proto`, `event_bus/session.rs`

---

## Milestone 2.3: Remote Provider Implementations

**Duration:** 4 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 2.3.1 | Implement `RemoteAIServer` | Forward requests over event bus | Requests forwarded correctly |
| 2.3.2 | Implement `RemoteVideoProvider` | Subscribe to video frames | Frames received |
| 2.3.3 | Implement `RemoteDetectionProvider` | Subscribe to detections | Detections received |
| 2.3.4 | Add reconnection logic | Exponential backoff | Reconnection works |

### Code: Remote AI Server

```rust
// shared/ai-client/src/remote.rs

pub struct RemoteAIServer {
    client: EventBusClient,
    session_id: String,
    pending_requests: Arc<RwLock<HashMap<String, oneshot::Sender<InferResponse>>>>,
}

#[async_trait]
impl AIServerProvider for RemoteAIServer {
    async fn infer(&self, request: InferRequest) -> Result<InferResponse> {
        // Create response channel
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request.request_id.clone(), tx);
        }

        // Send request via event bus
        let envelope = envelope(
            "vlm.request",
            &self.session_id,
            VlmRequest::from(request.clone()),
        );
        self.client.publish(envelope).await?;

        // Wait for response (with timeout)
        let response = tokio::time::timeout(
            Duration::from_secs(30),
            rx
        ).await??;

        Ok(response)
    }

    async fn query(&self, request: VideoQueryRequest) -> Result<VideoQueryResponse> {
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request.request_id.clone(), tx);
        }

        let envelope = envelope(
            "vlm.query",
            &self.session_id,
            VideoQuery::from(request.clone()),
        );
        self.client.publish(envelope).await?;

        let response = tokio::time::timeout(
            Duration::from_secs(30),
            rx
        ).await??;

        Ok(response)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // Request model list from server
        let envelope = envelope(
            "model.list.request",
            &self.session_id,
            ModelListRequest {},
        );
        self.client.publish(envelope).await?;

        // Wait for response
        // ... similar pattern
        todo!()
    }

    async fn health(&self) -> Result<HealthStatus> {
        if self.client.is_connected() {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Unhealthy)
        }
    }
}

impl RemoteAIServer {
    /// Handle incoming response messages.
    async fn handle_response(&self, envelope: Envelope) {
        if let Ok(response) = decode::<VlmResponse>(&envelope.payload) {
            let mut pending = self.pending_requests.write().await;
            if let Some(tx) = pending.remove(&response.request_id) {
                let _ = tx.send(InferResponse::from(response));
            }
        }
    }
}
```

### Code: Reconnection Logic

```rust
// shared/ai-client/src/remote.rs

pub struct RemoteConnection {
    config: RemoteConfig,
    client: Arc<RwLock<Option<EventBusClient>>>,
    reconnect_state: Arc<RwLock<ReconnectState>>,
}

struct ReconnectState {
    attempts: u32,
    last_attempt: Instant,
    backoff: Duration,
}

impl RemoteConnection {
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

    pub async fn connect(&self) -> Result<EventBusClient> {
        loop {
            match self.try_connect().await {
                Ok(client) => {
                    self.reset_backoff().await;
                    return Ok(client);
                }
                Err(e) => {
                    let backoff = self.next_backoff().await;
                    tracing::warn!(
                        error = %e,
                        backoff_secs = backoff.as_secs(),
                        "Connection failed, retrying"
                    );
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    async fn try_connect(&self) -> Result<EventBusClient> {
        let client = EventBusClient::connect(&self.config.url).await?;

        // Authenticate
        let authenticator = ClientAuthenticator::new(&self.config.auth_secret);
        authenticator.authenticate(&client).await?;

        Ok(client)
    }

    async fn next_backoff(&self) -> Duration {
        let mut state = self.reconnect_state.write().await;
        state.attempts += 1;
        state.backoff = std::cmp::min(
            state.backoff * 2,
            Self::MAX_BACKOFF,
        );
        state.last_attempt = Instant::now();
        state.backoff
    }

    async fn reset_backoff(&self) {
        let mut state = self.reconnect_state.write().await;
        state.attempts = 0;
        state.backoff = Self::INITIAL_BACKOFF;
    }
}
```

### Verification

```bash
cargo run --example mock_ai_server &
cargo run -p yama-host-apple -- --config client.toml
# Connects and receives mock data
```

### Deliverable

`shared/ai-client/src/remote.rs`

---

## Dependencies

- Phase 1 (Abstraction Layer) - trait definitions

## Blocks

- Phase 5 (VLM Orchestration) - server-side handling
- Phase 6 (Video Streaming) - WebRTC signaling
- Phase 8 (E2E Integration) - client-server communication

---

## Agent Review Additions

The following tasks were added based on agent review findings (Architecture, UX, Agent Design).

### Milestone 2.4: Architecture Hardening

| ID | Task | Rationale | Days |
|----|------|-----------|------|
| 2.4.1 | Add message schema versioning | Enable breaking changes without disruption | 0.5 |
| 2.4.2 | Implement backpressure/flow control | Prevent 240 msg/s from overwhelming clients | 1 |
| 2.4.3 | Add graceful shutdown with CancellationToken | Clean shutdown of background tasks | 0.5 |
| 2.4.4 | Replace RwLock<HashMap> with DashMap | Lock contention at 240 msg/sec identified | 0.5 |
| 2.4.5 | Handle broadcast receiver lag | Slow clients must not block fast publishers | 0.5 |

### Code: Graceful Shutdown Pattern

```rust
// event_bus/mod.rs

use tokio_util::sync::CancellationToken;

pub struct EventBus {
    config: EventBusConfig,
    clients: Arc<DashMap<String, Client>>,
    broadcast_tx: broadcast::Sender<Envelope>,
    cancel_token: CancellationToken,
}

impl EventBus {
    pub async fn run(&self) -> Result<()> {
        let ws_listener = TcpListener::bind(&self.config.websocket_bind).await?;

        loop {
            tokio::select! {
                // Graceful shutdown
                _ = self.cancel_token.cancelled() => {
                    tracing::info!("Event bus shutting down gracefully");
                    self.drain_connections().await;
                    break;
                }

                // Accept new connections
                Ok((stream, addr)) = ws_listener.accept() => {
                    let child_token = self.cancel_token.child_token();
                    tokio::spawn(async move {
                        handle_connection(stream, addr, child_token).await
                    });
                }
            }
        }
        Ok(())
    }

    pub fn shutdown(&self) {
        self.cancel_token.cancel();
    }

    async fn drain_connections(&self) {
        // Give clients 5 seconds to finish pending work
        tokio::time::sleep(Duration::from_secs(5)).await;
        self.clients.clear();
    }
}
```

### Code: Backpressure / Flow Control

```rust
// event_bus/flow_control.rs

use tokio::sync::Semaphore;

pub struct FlowController {
    /// Maximum pending messages per client
    semaphore: Arc<Semaphore>,
    /// High-water mark for warnings
    high_water: usize,
}

impl FlowController {
    pub fn new(max_pending: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_pending)),
            high_water: max_pending * 80 / 100, // 80%
        }
    }

    /// Acquire a slot before sending. Returns error if client is overwhelmed.
    pub async fn acquire(&self) -> Result<SemaphorePermit<'_>> {
        match self.semaphore.try_acquire() {
            Ok(permit) => Ok(permit),
            Err(_) => {
                // Client is backpressured - drop low-priority messages
                tracing::warn!("Client backpressured, dropping message");
                Err(anyhow!("Client overwhelmed"))
            }
        }
    }

    /// Check if nearing capacity
    pub fn is_near_capacity(&self) -> bool {
        self.semaphore.available_permits() < self.high_water
    }
}

// Usage in message dispatch
async fn dispatch_to_client(
    client: &Client,
    envelope: Envelope,
    flow: &FlowController,
) -> Result<()> {
    // Skip low-priority messages when backpressured
    if flow.is_near_capacity() && envelope.priority < 5 {
        return Ok(()); // Drop silently
    }

    let _permit = flow.acquire().await?;
    client.send(envelope).await
}
```

### Code: Broadcast Receiver Lag Handling

```rust
// event_bus/broadcast.rs

async fn run_client_broadcast_loop(
    mut rx: broadcast::Receiver<Envelope>,
    client_tx: mpsc::Sender<Envelope>,
    cancel: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,

            result = rx.recv() => {
                match result {
                    Ok(envelope) => {
                        if client_tx.send(envelope).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Client was too slow, missed n messages
                        tracing::warn!(
                            missed = n,
                            "Client lagged, messages dropped"
                        );
                        // Send lag notification to client
                        let _ = client_tx.send(Envelope {
                            topic: "system.lag".into(),
                            payload: Some(LagNotification { missed: n }.into()),
                            ..Default::default()
                        }).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}
```

### Code: Schema Version Validation

```rust
// event_bus/validation.rs

const CURRENT_SCHEMA_VERSION: &str = "1.0.0";
const MIN_COMPATIBLE_VERSION: &str = "1.0.0";

pub fn validate_envelope(envelope: &Envelope) -> Result<()> {
    // Check schema version compatibility
    if envelope.schema_version.is_empty() {
        // Legacy client - allow but log warning
        tracing::warn!("Received envelope without schema_version");
        return Ok(());
    }

    let client_version = semver::Version::parse(&envelope.schema_version)?;
    let min_version = semver::Version::parse(MIN_COMPATIBLE_VERSION)?;

    if client_version < min_version {
        return Err(anyhow!(
            "Schema version {} is below minimum {}",
            envelope.schema_version,
            MIN_COMPATIBLE_VERSION
        ));
    }

    Ok(())
}
```

### Milestone 2.5: UX Improvements

| ID | Task | Rationale | Days |
|----|------|-----------|------|
| 2.5.1 | Add connection state indicators | Users need visual feedback on connection health | 0.5 |
| 2.5.2 | Show authentication failure reasons | "Silent failures" confuse users | 0.5 |
| 2.5.3 | Add reconnection progress UI | User should see backoff attempts | 0.5 |
| 2.5.4 | Add latency indicator | Show round-trip time to server | 0.5 |

### UX: Connection State Model

```
┌──────────┐   connect    ┌─────────────┐   auth success   ┌───────────┐
│Disconn-  │──────────────▶│Authenticat- │──────────────────▶│Connected  │
│ected     │               │ing          │                   │           │
└────▲─────┘               └─────────────┘                   └─────┬─────┘
     │                           │                                 │
     │ max retries               │ auth failed                     │ connection lost
     │ exceeded                  ▼                                 ▼
     │                     ┌─────────────┐                   ┌───────────┐
     └─────────────────────│Auth Failed  │                   │Reconnect- │
                           │(show reason)│                   │ing (n/max)│
                           └─────────────┘                   └───────────┘
```

### UX: Connection Status Component

| State | Icon | Text | Color |
|-------|------|------|-------|
| Disconnected | ○ | "Disconnected" | Ash (#71717A) |
| Authenticating | ◐ (spin) | "Connecting..." | Amber (#F59E0B) |
| Auth Failed | ✕ | "Auth failed: {reason}" | Ember (#EF4444) |
| Connected | ● | "Connected ({latency}ms)" | Jade (#10B981) |
| Reconnecting | ◐ (spin) | "Reconnecting ({n}/{max})..." | Amber (#F59E0B) |

---

## Checklist

### Milestone 2.1: Event Bus Authentication
- [ ] 2.1.1 Add `AuthChallenge` / `AuthResponse` proto
- [ ] 2.1.2 Implement HMAC-SHA256 challenge-response
- [ ] 2.1.3 Add auth middleware to WebSocket
- [ ] 2.1.4 Add shared secret config

### Milestone 2.2: Remote Session Protocol
- [ ] 2.2.1 Define session proto messages
- [ ] 2.2.2 Implement session manager
- [ ] 2.2.3 Add capability negotiation
- [ ] 2.2.4 Implement session heartbeat

### Milestone 2.3: Remote Provider Implementations
- [ ] 2.3.1 Implement `RemoteAIServer`
- [ ] 2.3.2 Implement `RemoteVideoProvider`
- [ ] 2.3.3 Implement `RemoteDetectionProvider`
- [ ] 2.3.4 Add reconnection logic

### Milestone 2.4: Architecture Hardening (Agent Review)
- [ ] 2.4.1 Add message schema versioning to Envelope
- [ ] 2.4.2 Implement backpressure/flow control
- [ ] 2.4.3 Add graceful shutdown with CancellationToken
- [ ] 2.4.4 Replace RwLock<HashMap> with DashMap
- [ ] 2.4.5 Handle broadcast receiver lag

### Milestone 2.5: UX Improvements (Agent Review)
- [ ] 2.5.1 Add connection state indicators
- [ ] 2.5.2 Show authentication failure reasons
- [ ] 2.5.3 Add reconnection progress UI
- [ ] 2.5.4 Add latency indicator
