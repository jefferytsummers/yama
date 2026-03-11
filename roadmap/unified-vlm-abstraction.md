# Unified VLM Abstraction: Apple + Jetson Triton

**Status:** Implemented
**Date:** 2026-03-11

## Objective

Create a unified VLM inference abstraction that provides consistent UX across:
- **Apple**: mistral.rs + Metal MPS (existing)
- **Jetson**: Triton Inference Server + TensorRT-LLM (new)

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    VlmService (Unified)                          │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  VlmBackend Trait                                          │  │
│  │  - async analyze(VlmRequest) -> VlmResponse                │  │
│  │  - async analyze_batch(VlmBatchRequest) -> VlmBatchResult  │  │
│  │  - fn capabilities() -> VlmCapabilities                    │  │
│  └────────────────────────────────────────────────────────────┘  │
│                              │                                    │
│      ┌───────────────────────┼───────────────────────┐           │
│      ▼                       ▼                       ▼           │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐   │
│  │DirectBackend │    │EventBusBackend│   │TritonVlmBackend │   │
│  │(Apple Metal) │    │(Container IPC)│   │(Jetson gRPC)    │   │
│  └──────┬───────┘    └──────┬───────┘   └────────┬─────────┘   │
│         │                   │                     │              │
└─────────┼───────────────────┼─────────────────────┼──────────────┘
          ▼                   ▼                     ▼
   ┌─────────────┐    ┌─────────────┐    ┌────────────────────┐
   │ mistral.rs  │    │ yama-vlm    │    │ Triton Server      │
   │ + Metal MPS │    │ Container   │    │ + TensorRT-LLM     │
   └─────────────┘    └─────────────┘    │ + Qwen2.5-VL       │
                                         └────────────────────┘
```

## Implementation

### Phase 1: VlmBackend Trait (shared/platform-traits) ✅

**Files created:**
- `shared/platform-traits/src/vlm.rs`

**Key types:**
```rust
pub struct VlmRequest {
    pub request_id: String,
    pub prompt: String,
    pub image: VlmImageSource,  // JPEG, PNG, Raw, or GPU buffer
    pub params: VlmGenerationParams,
}

pub struct VlmResponse {
    pub request_id: String,
    pub analysis: String,
    pub inference_time_ms: f32,
    pub tokens_generated: u32,
}

#[async_trait]
pub trait VlmBackend: Send + Sync {
    fn capabilities(&self) -> &VlmCapabilities;
    async fn analyze(&self, request: VlmRequest) -> PlatformResult<VlmResponse>;
    async fn analyze_batch(&self, request: VlmBatchRequest) -> PlatformResult<VlmBatchResult>;
    async fn is_ready(&self) -> bool;
    async fn warmup(&self) -> PlatformResult<()>;
    fn name(&self) -> &'static str;
}
```

### Phase 2: Apple DirectVlmBackend ✅

**Files modified:**
- `platform/apple/host/src/inference/direct_backend.rs`

**Changes:**
- Implements `VlmBackend` trait (in addition to legacy `InferenceBackend`)
- Converts `VlmImageSource` to `DynamicImage` for mistral.rs
- Supports JPEG, PNG, RGB, RGBA input formats

### Phase 3: Apple EventBusBackend ✅

**Files modified:**
- `platform/apple/host/src/inference/event_bus_backend.rs`

**Changes:**
- Implements `VlmBackend` trait
- Converts image sources to JPEG for event bus transmission

### Phase 4: Jetson TritonVlmBackend ✅

**Files created:**
```
platform/jetson/host/src/inference/
├── mod.rs                  # VlmInferenceService
├── triton_backend.rs       # TritonVlmBackend
├── triton_client.rs        # HTTP/gRPC client
└── frame_extractor.rs      # NVDEC-based extraction
```

**TritonVlmBackend features:**
- HTTP client to Triton Inference Server (port 8000)
- Image preprocessing (resize, NV12→RGB conversion)
- Batch processing with configurable concurrency
- Health checks and warmup

### Phase 5: Jetson Frame Extractor ✅

**Files created:**
- `platform/jetson/host/src/inference/frame_extractor.rs`

**GStreamer pipeline:**
```
filesrc ! qtdemux ! h264parse ! nvv4l2decoder ! nvvidconv !
video/x-raw,format=BGRx ! videoconvert ! jpegenc ! appsink
```

### Phase 6: Triton Deployment ✅

**Files created:**
```
platform/jetson/triton-models/
├── README.md               # Build instructions
└── qwen2_5_vl/
    ├── config.pbtxt        # Model configuration
    └── 1/                  # Model version directory

platform/jetson/docker-compose.triton.yml
```

## Data Flow Comparison

| Step | Apple (Direct) | Jetson (Triton) |
|------|----------------|-----------------|
| 1. Decode | VideoToolbox | NVDEC |
| 2. Extract | GStreamer → JPEG | GStreamer → NV12 |
| 3. Preprocess | CPU/Metal | CUDA kernels |
| 4. Inference | mistral.rs in-process | Triton gRPC |
| 5. Response | Direct return | gRPC response |

## Configuration

```toml
# yama.toml

[vlm]
backend = "direct"  # "direct", "event_bus", "triton"

[vlm.triton]
server_url = "localhost:8001"
model_name = "qwen2_5_vl"
use_system_shared_memory = true
```

## Next Steps

### Jetson (Thor Machine)
1. Build TensorRT-LLM engine for Qwen2.5-VL
2. Deploy Triton with docker-compose
3. Test TritonVlmBackend end-to-end
4. Optimize for memory constraints

### Frontend (Mac Machine)
1. Build UI with mock backends
2. Implement SSE streaming
3. Design inference job dashboard
