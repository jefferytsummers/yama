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

## Checklist

- [ ] 2.1.1 Add `AuthChallenge` / `AuthResponse` proto
- [ ] 2.1.2 Implement HMAC-SHA256 challenge-response
- [ ] 2.1.3 Add auth middleware to WebSocket
- [ ] 2.1.4 Add shared secret config
- [ ] 2.2.1 Define session proto messages
- [ ] 2.2.2 Implement session manager
- [ ] 2.2.3 Add capability negotiation
- [ ] 2.2.4 Implement session heartbeat
- [ ] 2.3.1 Implement `RemoteAIServer`
- [ ] 2.3.2 Implement `RemoteVideoProvider`
- [ ] 2.3.3 Implement `RemoteDetectionProvider`
- [ ] 2.3.4 Add reconnection logic
