# Phase 4: Jetson Thor Deployment

**Duration:** 20 days
**Goal:** Deploy the agent system to NVIDIA Jetson Thor for edge video analytics with DeepStream, Triton, and WebRTC.

---

## Overview

This phase extends the Mac-based agent system to run on NVIDIA Jetson Thor, enabling:
- Multi-stream video analytics (4-8 concurrent 1080p/4K streams)
- Real-time object detection with DeepStream
- VLM inference via Triton Inference Server
- Low-latency video streaming via WebRTC

Jetson Thor's 128GB LPDDR5X memory and Blackwell GPU enable running 70B+ VLM models for sophisticated video understanding at the edge.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    JETSON AI SERVER (Headless)                          │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Video Ingestion (DeepStream 7.x)                               │   │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐                   │   │
│  │  │RTSP #1 │ │RTSP #2 │ │RTSP #3 │ │... #8  │  (NVDEC decode)   │   │
│  │  └───┬────┘ └───┬────┘ └───┬────┘ └───┬────┘                   │   │
│  │      └──────────┴──────────┴──────────┘                         │   │
│  │                      │                                          │   │
│  │                      ▼                                          │   │
│  │  ┌──────────────────────────────────────────────────────────┐  │   │
│  │  │  NvInfer (YOLOv8) + NvDCF Tracker (batched inference)    │  │   │
│  │  └──────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                      │                                                  │
│                      ▼                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  AI Orchestrator (Rust)                                         │   │
│  │  ┌──────────────────┐  ┌──────────────────┐                     │   │
│  │  │ Detection Router │  │ VLM Trigger      │                     │   │
│  │  │ (event-based)    │  │ (on-demand)      │                     │   │
│  │  └──────────────────┘  └──────────────────┘                     │   │
│  │                                                                  │   │
│  │  ┌──────────────────────────────────────────────────────────┐  │   │
│  │  │  Triton Inference Server                                  │  │   │
│  │  │  ├── YOLOv8n.engine (TensorRT)                           │  │   │
│  │  │  ├── Qwen2.5-VL-7B (TensorRT-LLM)                        │  │   │
│  │  │  └── CLIP ViT-B/32 (TensorRT)                            │  │   │
│  │  └──────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                      │                                                  │
│                      ▼                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Event Bus (ws://jetson:8765) + WebRTC Streaming                │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                           │
                           │ WebSocket + WebRTC
                           │
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   Mac Client    │ │   Web Client    │ │   Mobile        │
│   (egui)        │ │   (SvelteKit)   │ │   (Future)      │
└─────────────────┘ └─────────────────┘ └─────────────────┘
```

---

## Jetson Thor Capabilities

| Spec | Value | Impact |
|------|-------|--------|
| Memory | 128GB LPDDR5X | Run 70B+ models |
| GPU | Blackwell 2560 cores | 7.5x AI compute over Orin |
| FP4 Support | Native | 2x model capacity vs FP8 |
| Video Decode | 10x 4Kp60 | Far exceeds 4-8 stream target |

---

## Milestone 4.1: Platform Abstraction

**Duration:** 3 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 4.1.1 | Remote provider traits | Abstract local vs remote | Same API for both modes |
| 4.1.2 | Deployment config | Mac standalone vs Jetson | Config selects mode |
| 4.1.3 | Event bus authentication | Secure remote connections | HMAC auth works |
| 4.1.4 | Session management | Track remote client sessions | Sessions persist across reconnect |

### Deployment Modes

| Mode | Client | AI Server | Use Case |
|------|--------|-----------|----------|
| Mac Standalone | Mac (egui) | Mac (local) | Development |
| Mac + Jetson | Mac (egui) | Jetson (remote) | Production |
| Jetson Standalone | Web / headless | Jetson (local) | Edge deployment |

### Code: Remote Provider

```rust
// shared/platform-traits/src/remote.rs

#[async_trait]
pub trait RemoteProvider: Send + Sync {
    async fn connect(&mut self, endpoint: &str, auth: &AuthConfig) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;

    async fn search(&self, request: SearchRequest) -> Result<SearchResponse>;
    async fn analyze(&self, request: AnalyzeRequest) -> Result<AnalyzeResponse>;
    async fn extract(&self, request: ExtractRequest) -> Result<ExtractResponse>;
}
```

### Verification

```bash
cargo test -p yama-platform-traits test_remote_provider
# AC-1: Provider connects to mock server
# AC-2: Auth failure returns clear error
```

---

## Milestone 4.2: DeepStream Pipeline

**Duration:** 5 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 4.2.1 | Multi-stream input | 4-8 RTSP sources | All streams decode |
| 4.2.2 | NvInfer integration | YOLOv8n TensorRT | Detections at 30fps |
| 4.2.3 | NvDCF tracker | Object tracking | Track IDs persist across frames |
| 4.2.4 | Metadata probe | Extract detection data | Bounding boxes accessible |
| 4.2.5 | DeepStream container | Docker deployment | Container starts cleanly |

### DeepStream Config

```ini
# deepstream_config.txt
[application]
enable-perf-measurement=1
perf-measurement-interval-sec=5

[tiled-display]
enable=1
rows=2
columns=4
width=1920
height=1080

[source0]
enable=1
type=4
uri=rtsp://camera1.local/stream
num-sources=1
gpu-id=0
cudadec-memtype=0

[primary-gie]
enable=1
model-engine-file=yolov8n.engine
batch-size=8
gpu-id=0
```

### Verification

```bash
# On Jetson
docker compose up deepstream
# AC-1: All camera streams visible in tiled display
# AC-2: Detection overlays appear at 30fps
# AC-3: Memory usage <4GB for 8 streams
```

---

## Milestone 4.3: Triton Inference Server

**Duration:** 5 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 4.3.1 | Model repository | Structure for 3 models | Triton loads all models |
| 4.3.2 | YOLOv8 TensorRT | Detection model | Inference <10ms |
| 4.3.3 | CLIP TensorRT | Embedding model | Batch of 32 <100ms |
| 4.3.4 | Qwen2.5-VL TensorRT-LLM | VLM model | Generation 30 tokens/s |
| 4.3.5 | Triton Rust client | gRPC client | Client connects and queries |

### Model Repository Structure

```
models/
├── yolov8n/
│   ├── config.pbtxt
│   └── 1/
│       └── model.plan
├── clip_visual/
│   ├── config.pbtxt
│   └── 1/
│       └── model.plan
└── qwen2_5_vl/
    ├── config.pbtxt
    └── 1/
        └── model/
            ├── config.json
            └── weights.safetensors
```

### Code: Triton Client

```rust
// platform/jetson/ai-server/src/triton/client.rs

pub struct TritonClient {
    endpoint: String,
    models: HashMap<String, ModelHandle>,
}

impl TritonClient {
    pub async fn connect(endpoint: &str) -> Result<Self>;
    pub async fn infer(&self, model: &str, inputs: Vec<Tensor>) -> Result<Vec<Tensor>>;
    pub async fn model_ready(&self, model: &str) -> Result<bool>;
    pub async fn load_model(&self, model: &str) -> Result<()>;
    pub async fn unload_model(&self, model: &str) -> Result<()>;
}
```

### Verification

```bash
# On Jetson
tritonserver --model-repository=/models
curl localhost:8000/v2/health/ready
# AC-1: All 3 models ready
# AC-2: YOLOv8 inference <10ms
# AC-3: VLM generates at 30 tokens/s
```

---

## Milestone 4.4: VLM Orchestration

**Duration:** 4 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 4.4.1 | Detection-triggered VLM | Invoke VLM on detection events | VLM called when person detected |
| 4.4.2 | Query handler | Process user queries | Natural language queries work |
| 4.4.3 | Context store | Maintain scene context | Context persists across queries |
| 4.4.4 | Multi-camera aggregation | Combine detections | Single view of all cameras |

### Code: VLM Orchestrator

```rust
// platform/jetson/ai-server/src/orchestrator/mod.rs

pub struct VlmOrchestrator {
    triton: Arc<TritonClient>,
    context_store: Arc<ContextStore>,
    detection_rx: mpsc::Receiver<Detection>,
}

impl VlmOrchestrator {
    pub async fn handle_detection(&mut self, detection: Detection) -> Result<Option<VlmAnalysis>>;
    pub async fn handle_query(&mut self, query: &str, camera_id: Option<&str>) -> Result<VlmAnalysis>;
}
```

### Verification

```bash
# Integration test
cargo run -p yama-jetson-ai-server --example vml_query -- "What is the person in camera 3 doing?"
# AC-1: Returns coherent description
# AC-2: Response time <3s
```

---

## Milestone 4.5: WebRTC Streaming

**Duration:** 3 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 4.5.1 | WebRTC server | GStreamer webrtcsink | Peer connection established |
| 4.5.2 | Detection overlay | Render boxes on stream | Overlays visible to client |
| 4.5.3 | HLS fallback | Fallback for compatibility | HLS plays in Safari |
| 4.5.4 | Client receiver | Receive in web/egui | Video plays in client |

### Code: WebRTC Server

```rust
// platform/jetson/ai-server/src/streaming/webrtc.rs

pub struct WebRtcServer {
    pipeline: gst::Pipeline,
    signaling: WebSocketServer,
}

impl WebRtcServer {
    pub async fn start(&mut self, port: u16) -> Result<()>;
    pub async fn add_peer(&mut self, peer_id: &str, sdp_offer: &str) -> Result<String>;
    pub async fn remove_peer(&mut self, peer_id: &str) -> Result<()>;
}
```

### Verification

```bash
# Open web client
# AC-1: Video streams with <500ms latency
# AC-2: Detection boxes overlay on video
# AC-3: HLS fallback works in Safari
```

---

## Dependencies

- Phase 1 (Core Infrastructure) - Event bus, config
- Phase 2 (Indexing & Embeddings) - CLIP, search
- Phase 3 (Agent System) - Tool executor, chat

## Blocks

None (final phase)

---

## Checklist

### Milestone 4.1: Platform Abstraction
- [ ] 4.1.1 Remote provider traits
- [ ] 4.1.2 Deployment config
- [ ] 4.1.3 Event bus authentication
- [ ] 4.1.4 Session management

### Milestone 4.2: DeepStream Pipeline
- [ ] 4.2.1 Multi-stream input
- [ ] 4.2.2 NvInfer integration
- [ ] 4.2.3 NvDCF tracker
- [ ] 4.2.4 Metadata probe
- [ ] 4.2.5 DeepStream container

### Milestone 4.3: Triton Inference Server
- [ ] 4.3.1 Model repository
- [ ] 4.3.2 YOLOv8 TensorRT
- [ ] 4.3.3 CLIP TensorRT
- [ ] 4.3.4 Qwen2.5-VL TensorRT-LLM
- [ ] 4.3.5 Triton Rust client

### Milestone 4.4: VLM Orchestration
- [ ] 4.4.1 Detection-triggered VLM
- [ ] 4.4.2 Query handler
- [ ] 4.4.3 Context store
- [ ] 4.4.4 Multi-camera aggregation

### Milestone 4.5: WebRTC Streaming
- [ ] 4.5.1 WebRTC server
- [ ] 4.5.2 Detection overlay
- [ ] 4.5.3 HLS fallback
- [ ] 4.5.4 Client receiver

---

## Reference Links

### NVIDIA Official
- [Jetson AI Lab](https://www.jetson-ai-lab.com/)
- [Live VLM WebUI Tutorial](https://www.jetson-ai-lab.com/tutorials/live-vlm-webui/)
- [Jetson Platform Services](https://docs.nvidia.com/jetson/jps/moj-overview.html)
- [Agentic Video Workflow Blueprint](https://developer.nvidia.com/blog/build-an-agentic-video-workflow-with-video-search-and-summarization/)
- [Jetson Thor Technical Blog](https://developer.nvidia.com/blog/introducing-nvidia-jetson-thor-the-ultimate-platform-for-physical-ai/)
- [DeepStream 7.0 Docs](https://docs.nvidia.com/metropolis/deepstream/7.0/text/DS_Overview.html)
- [TensorRT-LLM](https://github.com/NVIDIA/TensorRT-LLM)
