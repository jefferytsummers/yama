# Phase 1: Abstraction Layer Foundation

**Duration:** 11 days
**Goal:** Establish platform-agnostic traits that enable all three deployment modes.

---

## Overview

This phase creates the shared abstraction layer that allows the same application code to run against local providers (Mac standalone) or remote providers (Mac + Jetson). All subsequent phases depend on these foundational traits.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        APPLICATION LAYER                                │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  egui UI / Web UI / Headless CLI                                │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        ABSTRACTION LAYER (shared/platform-traits/)      │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ AIServerProvider │  │ VideoProvider    │  │ DetectionProvider│     │
│  │                  │  │                  │  │                  │     │
│  │ • infer()        │  │ • get_frame()    │  │ • get_current()  │     │
│  │ • query()        │  │ • subscribe()    │  │ • subscribe()    │     │
│  │ • list_models()  │  │ • list_sources() │  │ • search()       │     │
│  │ • load_model()   │  │                  │  │                  │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ ToolProvider     │  │ ContextStore     │  │ StreamProvider   │     │
│  │                  │  │                  │  │                  │     │
│  │ • execute()      │  │ • store()        │  │ • get_stream()   │     │
│  │ • list_tools()   │  │ • query()        │  │ • start_webrtc() │     │
│  │                  │  │ • retention()    │  │ • start_hls()    │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
          ┌─────────────────────────┼─────────────────────────┐
          ▼                         ▼                         ▼
┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────┐
│   LOCAL IMPL        │  │   REMOTE IMPL       │  │   JETSON IMPL       │
│   (Mac Standalone)  │  │   (Mac Client)      │  │   (Jetson Local)    │
│                     │  │                     │  │                     │
│ • DirectVlmBackend  │  │ • RemoteAIServer    │  │ • TritonBackend     │
│ • LocalVideoSource  │  │ • RemoteVideo       │  │ • DeepStreamVideo   │
│ • LocalDetection    │  │ • RemoteDetection   │  │ • NvInferDetection  │
│ • Metal MPS         │  │ • WebSocket client  │  │ • CUDA/TensorRT     │
└─────────────────────┘  └─────────────────────┘  └─────────────────────┘
```

---

## Milestone 1.1: Core Trait Definitions

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 1.1.1 | Define `AIServerProvider` trait | Unified AI server interface | Compiles with `infer()`, `query()`, `list_models()`, `load_model()`, `unload_model()`, `health()` |
| 1.1.2 | Define `VideoProvider` trait | Video source abstraction | Compiles with `get_frame()`, `subscribe()`, `list_sources()`, `add_source()` |
| 1.1.3 | Define `DetectionProvider` trait | Detection source abstraction | Compiles with `get_detections()`, `subscribe()`, `search()` |
| 1.1.4 | Define `ContextStore` trait | Historical data storage | Compiles with `store()`, `query()`, `set_retention()` |
| 1.1.5 | Define `StreamProvider` trait | Video streaming abstraction | Compiles with `get_stream_url()`, `start_webrtc()`, `start_hls()` |

### Code: AIServerProvider Trait

```rust
// shared/platform-traits/src/ai_server.rs

/// Unified AI server interface for all deployment modes.
#[async_trait]
pub trait AIServerProvider: Send + Sync {
    /// Run VLM inference on an image.
    async fn infer(&self, request: InferRequest) -> Result<InferResponse>;

    /// Run a contextual video query.
    async fn query(&self, request: VideoQueryRequest) -> Result<VideoQueryResponse>;

    /// List available models.
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    /// Load a model (dynamic loading support).
    async fn load_model(&self, model_id: &str) -> Result<()>;

    /// Unload a model to free memory.
    async fn unload_model(&self, model_id: &str) -> Result<()>;

    /// Check server health.
    async fn health(&self) -> Result<HealthStatus>;
}
```

### Code: VideoProvider Trait

```rust
// shared/platform-traits/src/video.rs

/// Video source provider (local or remote).
#[async_trait]
pub trait VideoProvider: Send + Sync {
    /// Get current frame from a source.
    async fn get_frame(&self, source_id: &str) -> Result<Frame>;

    /// Subscribe to frame updates.
    async fn subscribe(&self, source_id: &str) -> Result<FrameReceiver>;

    /// List available video sources.
    async fn list_sources(&self) -> Result<Vec<SourceInfo>>;

    /// Add a new video source.
    async fn add_source(&self, config: SourceConfig) -> Result<String>;
}
```

### Code: DetectionProvider Trait

```rust
// shared/platform-traits/src/detection.rs

/// Detection provider (local YOLO or remote DeepStream).
#[async_trait]
pub trait DetectionProvider: Send + Sync {
    /// Get current detections for a source.
    async fn get_detections(&self, source_id: &str) -> Result<Vec<Detection>>;

    /// Subscribe to detection updates.
    async fn subscribe(&self, source_ids: &[&str]) -> Result<DetectionReceiver>;

    /// Search detection history.
    async fn search(&self, query: DetectionQuery) -> Result<Vec<DetectionEvent>>;
}
```

### Verification

```bash
cargo build -p yama-platform-traits
cargo test -p yama-platform-traits
cargo clippy -p yama-platform-traits -- -D warnings
```

### Deliverable

`shared/platform-traits/src/providers.rs`

---

## Milestone 1.2: Request/Response Types

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 1.2.1 | Define `InferRequest` / `InferResponse` | VLM inference types | Covers image data, prompt, model selection |
| 1.2.2 | Define `VideoQueryRequest` / `VideoQueryResponse` | Contextual query types | Covers contextual queries |
| 1.2.3 | Define `Frame` type | Video frame representation | Includes data, format, dimensions, timestamp |
| 1.2.4 | Define `Detection` and `TrackedObject` | Detection types | Includes bbox, class, confidence, track_id |
| 1.2.5 | Define `SourceConfig` and `SourceInfo` | Video source types | RTSP URL, credentials, resolution |

### Code: Core Types

```rust
// shared/platform-traits/src/types.rs

/// VLM inference request.
pub struct InferRequest {
    pub request_id: String,
    pub image: ImageData,
    pub prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

/// VLM inference response.
pub struct InferResponse {
    pub request_id: String,
    pub text: String,
    pub model: String,
    pub inference_time_ms: u64,
    pub tokens_generated: u32,
}

/// Video frame.
pub struct Frame {
    pub source_id: String,
    pub frame_number: u64,
    pub timestamp: Timestamp,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub data: FrameData,
}

/// Detection result.
pub struct Detection {
    pub class_name: String,
    pub class_id: u32,
    pub confidence: f32,
    pub bbox: BoundingBox,
    pub track_id: Option<u64>,
}

/// Tracked object with motion data.
pub struct TrackedObject {
    pub track_id: u64,
    pub class_name: String,
    pub bbox: BoundingBox,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub state: TrackState,
}

pub enum TrackState {
    New,
    Active,
    Lost,
}
```

### Verification

```bash
cargo test -p yama-platform-traits -- types::tests
```

### Deliverable

`shared/platform-traits/src/types.rs`

---

## Milestone 1.3: Apple Local Implementations

**Duration:** 4 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 1.3.1 | Wrap `DirectVlmBackend` as `AIServerProvider` | Existing VLM through new trait | Existing VLM works through new trait |
| 1.3.2 | Implement `LocalVideoProvider` | Local video source | Can list sources, get frames |
| 1.3.3 | Implement `LocalDetectionProvider` (stub) | Placeholder for local detection | Returns empty (placeholder) |
| 1.3.4 | Implement `LocalContextStore` | In-memory ring buffer | Stores and queries history |

### Code: Local AI Server Wrapper

```rust
// platform/apple/host/src/providers/ai_server.rs

pub struct LocalAIServer {
    backend: Arc<DirectVlmBackend>,
}

#[async_trait]
impl AIServerProvider for LocalAIServer {
    async fn infer(&self, request: InferRequest) -> Result<InferResponse> {
        // Delegate to existing DirectVlmBackend
        self.backend.infer(request).await
    }

    async fn query(&self, request: VideoQueryRequest) -> Result<VideoQueryResponse> {
        // Build prompt with context and delegate
        let prompt = self.build_context_prompt(&request);
        let infer_req = InferRequest {
            request_id: request.request_id,
            image: request.frame.into(),
            prompt,
            model: None,
            max_tokens: Some(512),
            temperature: Some(0.7),
        };
        let response = self.backend.infer(infer_req).await?;
        Ok(VideoQueryResponse {
            request_id: response.request_id,
            analysis: response.text,
            sources_used: vec![],
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![self.backend.model_info()])
    }

    async fn load_model(&self, _model_id: &str) -> Result<()> {
        // Single model in local mode
        Ok(())
    }

    async fn unload_model(&self, _model_id: &str) -> Result<()> {
        Ok(())
    }

    async fn health(&self) -> Result<HealthStatus> {
        Ok(HealthStatus::Healthy)
    }
}
```

### Verification

```bash
cargo run -p yama-host-apple -- --video test.mp4 --prompt "Describe"
# Produces identical output to current behavior
```

### Deliverable

`platform/apple/host/src/providers/`

---

## Milestone 1.4: Deployment Configuration

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 1.4.1 | Define deployment mode enum | `Standalone`, `Client`, `Server` | Enum compiles |
| 1.4.2 | Create configuration schema | TOML parsing | TOML parsing works |
| 1.4.3 | Implement provider factory | `create_providers(config)` | Returns correct types |
| 1.4.4 | Add mode detection | Auto-detect from config | Auto-detect works |

### Code: Configuration Schema

```toml
# Mode 1: Mac Standalone
[deployment]
mode = "standalone"

[ai_server]
backend = "direct"  # In-process mistral.rs

[video]
backend = "local"   # Local GStreamer/VideoToolbox

[detection]
backend = "local"   # Local YOLO via CoreML or in-process

# Mode 2: Mac Client + Jetson Server
[deployment]
mode = "client"

[ai_server]
backend = "remote"
url = "ws://jetson.local:8765"
auth_secret = "..."

[video]
backend = "remote"
webrtc_enabled = true

[detection]
backend = "remote"

# Mode 3: Jetson Standalone
[deployment]
mode = "standalone"

[ai_server]
backend = "triton"
url = "localhost:8001"

[video]
backend = "deepstream"

[detection]
backend = "deepstream"
```

### Code: Provider Factory

```rust
// shared/ai-client/src/config.rs

pub enum DeploymentMode {
    Standalone,
    Client,
    Server,
}

pub struct DeploymentConfig {
    pub mode: DeploymentMode,
    pub ai_server: AIServerConfig,
    pub video: VideoConfig,
    pub detection: DetectionConfig,
}

pub fn create_providers(config: &DeploymentConfig) -> Result<Providers> {
    match config.mode {
        DeploymentMode::Standalone => {
            // Create local providers
            Ok(Providers {
                ai_server: Arc::new(LocalAIServer::new(&config.ai_server)?),
                video: Arc::new(LocalVideoProvider::new(&config.video)?),
                detection: Arc::new(LocalDetectionProvider::new()?),
            })
        }
        DeploymentMode::Client => {
            // Create remote providers
            Ok(Providers {
                ai_server: Arc::new(RemoteAIServer::new(&config.ai_server)?),
                video: Arc::new(RemoteVideoProvider::new(&config.video)?),
                detection: Arc::new(RemoteDetectionProvider::new()?),
            })
        }
        DeploymentMode::Server => {
            // Jetson server mode
            Ok(Providers {
                ai_server: Arc::new(TritonAIServer::new(&config.ai_server)?),
                video: Arc::new(DeepStreamVideoProvider::new(&config.video)?),
                detection: Arc::new(DeepStreamDetectionProvider::new()?),
            })
        }
    }
}
```

### Verification

```bash
cargo run -p yama-host-apple -- --config standalone.toml  # Works
cargo run -p yama-host-apple -- --config client.toml     # Fails gracefully (no server)
```

### Deliverable

`shared/ai-client/src/config.rs`

---

## Dependencies

This phase has no dependencies on other phases.

## Blocks

- Phase 2 (Remote Event Bus)
- Phase 3 (DeepStream Pipeline) - for trait implementation
- Phase 4 (Triton Serving) - for trait implementation
- Phase 5 (VLM Orchestration) - for provider usage

---

## Checklist

- [ ] 1.1.1 Define `AIServerProvider` trait
- [ ] 1.1.2 Define `VideoProvider` trait
- [ ] 1.1.3 Define `DetectionProvider` trait
- [ ] 1.1.4 Define `ContextStore` trait
- [ ] 1.1.5 Define `StreamProvider` trait
- [ ] 1.2.1 Define `InferRequest` / `InferResponse`
- [ ] 1.2.2 Define `VideoQueryRequest` / `VideoQueryResponse`
- [ ] 1.2.3 Define `Frame` type
- [ ] 1.2.4 Define `Detection` and `TrackedObject`
- [ ] 1.2.5 Define `SourceConfig` and `SourceInfo`
- [ ] 1.3.1 Wrap `DirectVlmBackend` as `AIServerProvider`
- [ ] 1.3.2 Implement `LocalVideoProvider`
- [ ] 1.3.3 Implement `LocalDetectionProvider` (stub)
- [ ] 1.3.4 Implement `LocalContextStore`
- [ ] 1.4.1 Define deployment mode enum
- [ ] 1.4.2 Create configuration schema
- [ ] 1.4.3 Implement provider factory
- [ ] 1.4.4 Add mode detection
