# Yama Jetson AI Server: Implementation Summary

## Executive Summary

This roadmap defines the implementation plan for the Yama Jetson AI Server architecture, enabling offloading of AI operations from Mac/Web clients to a headless Jetson AI server. The architecture supports three deployment modes through a unified abstraction layer.

**Target Capabilities:**
- 4-8 concurrent 1080p/4K video streams
- Real-time object detection (YOLOv8, <10ms/frame)
- On-demand VLM queries (Qwen2.5-VL-7B, <5s)
- Multi-stage video characterization and model routing
- Low-latency video streaming (WebRTC <200ms)
- Tool-based agent integration

---

## Deployment Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **Mac Standalone** | All components on Mac | Development, single user |
| **Mac + Jetson** | Mac client, Jetson server | Production, distributed |
| **Jetson Standalone** | Headless Jetson | Edge deployment |

---

## Phase Summary

### Phase 1: Abstraction Layer Foundation
**Duration:** 11 days | **Tasks:** 19

Establishes platform-agnostic traits enabling all deployment modes.

| Milestone | Description |
|-----------|-------------|
| 1.1 | Core trait definitions (`AIServerProvider`, `VideoProvider`, etc.) |
| 1.2 | Request/response types (`InferRequest`, `Frame`, `Detection`) |
| 1.3 | Apple local implementations (wrap existing code) |
| 1.4 | Deployment configuration (TOML parsing, provider factory) |

**Deliverables:**
- `shared/platform-traits/src/providers.rs`
- `shared/platform-traits/src/types.rs`
- `platform/apple/host/src/providers/`
- `shared/ai-client/src/config.rs`

---

### Phase 2: Remote Event Bus & Authentication
**Duration:** 10 days | **Tasks:** 12

Enables authenticated remote connections between clients and Jetson.

| Milestone | Description |
|-----------|-------------|
| 2.1 | Event bus authentication (HMAC-SHA256 challenge-response) |
| 2.2 | Remote session protocol (session management, heartbeat) |
| 2.3 | Remote provider implementations (`RemoteAIServer`, reconnection) |

**Deliverables:**
- `shared/protocol/proto/remote.proto`
- `event_bus/auth.rs`
- `event_bus/session.rs`
- `shared/ai-client/src/remote.rs`

---

### Phase 3: DeepStream Video Pipeline
**Duration:** 10 days | **Tasks:** 13

Multi-stream video analytics with hardware-accelerated detection.

| Milestone | Description |
|-----------|-------------|
| 3.1 | DeepStream pipeline setup (multi-RTSP, YOLOv8, NvDCF tracker) |
| 3.2 | Metadata extraction (probes, protobuf, frame sampling) |
| 3.3 | DeepStream container (Dockerfile, docker-compose) |

**Deliverables:**
- `containers/deepstream/src/main.cpp`
- `containers/deepstream/Dockerfile`
- `platform/jetson/deepstream-config/`

---

### Phase 4: Triton Model Serving
**Duration:** 9 days | **Tasks:** 11

Multi-model inference server with dynamic loading.

| Milestone | Description |
|-----------|-------------|
| 4.1 | Model repository setup (YOLOv8 TensorRT, Qwen2.5-VL) |
| 4.2 | Triton Rust client (gRPC, YOLO inference, VLM inference) |
| 4.3 | Dynamic model loading (memory management, hot-swap) |

**Deliverables:**
- `containers/triton/model_repository/`
- `platform/jetson/triton-client/`
- `ai-server/src/triton/model_manager.rs`

---

### Phase 5: Multi-Stage VLM Orchestration
**Duration:** 26 days | **Tasks:** 32

Intelligent multi-pass video analysis with model routing.

| Milestone | Description |
|-----------|-------------|
| 5.1 | Video characterization VLM (scene type, environment, content hints) |
| 5.2 | Model router (strategy selection, GPU memory, dynamic loading) |
| 5.3 | Multi-model execution engine (parallel execution, result merging) |
| 5.4 | Detection router & aggregation (filtering, cross-stream) |
| 5.5 | VLM trigger system (event-based, periodic, rate limiting) |
| 5.6 | Context store (ring buffer, history queries) |
| 5.7 | Query handler (context building, prompt construction) |

**Deliverables:**
- `ai-server/src/orchestrator/characterization.rs`
- `ai-server/src/orchestrator/model_router.rs`
- `ai-server/src/orchestrator/execution_engine.rs`
- `ai-server/src/orchestrator/detection_router.rs`
- `ai-server/src/orchestrator/vlm_trigger.rs`
- `ai-server/src/orchestrator/context_store.rs`
- `ai-server/src/orchestrator/query_handler.rs`

---

### Phase 6: Video Streaming
**Duration:** 11 days | **Tasks:** 10

Low-latency video delivery to remote clients.

| Milestone | Description |
|-----------|-------------|
| 6.1 | WebRTC server (STUN/TURN, signaling, H.264 RTP) |
| 6.2 | HLS fallback (segment generation, playlist server) |
| 6.3 | Client video receiver (Mac WebRTC, egui texture, overlay) |

**Deliverables:**
- `ai-server/src/streaming/webrtc.rs`
- `ai-server/src/streaming/hls.rs`
- `platform/apple/host/src/remote/video_receiver.rs`

---

### Phase 7: Tool System & Agent Integration
**Duration:** 8 days | **Tasks:** 13

Expose video analysis as agent tools.

| Milestone | Description |
|-----------|-------------|
| 7.1 | Tool definitions (`get_current_frame`, `get_detections`, etc.) |
| 7.2 | Tool handlers (execution router, provider integration) |
| 7.3 | Agent integration (tool registry, reasoning loop) |

**Deliverables:**
- `shared/agent-sdk/src/tools/video.rs`
- `shared/agent-sdk/src/tools/handlers.rs`
- `containers/agent/src/tools.rs`

---

### Phase 8: End-to-End Integration
**Duration:** 7 days | **Tasks:** 12

All components working together.

| Milestone | Description |
|-----------|-------------|
| 8.1 | Mac Standalone E2E (local video, VLM, chat) |
| 8.2 | Mac + Jetson E2E (remote connection, WebRTC, queries) |
| 8.3 | Jetson Standalone E2E (headless, web UI, HLS) |

**Deliverables:**
- `scripts/test-standalone.sh`
- `scripts/test-mac-jetson.sh`
- `scripts/test-jetson-standalone.sh`
- `docker-compose.yml` (all modes)

---

## Total Effort

| Metric | Value |
|--------|-------|
| **Phases** | 8 |
| **Milestones** | 29 |
| **Tasks** | 122 |
| **Estimated Duration** | 92 days (~19 weeks) |

---

## Critical Path

```
Phase 1 ──▶ Phase 2 ──▶ Phase 8
    │           │
    ▼           ▼
Phase 3 ──▶ Phase 5
    │           │
    ▼           ▼
Phase 4 ───────┘
                │
                ▼
          Phase 6
                │
                ▼
          Phase 7
```

**Parallelization Opportunities:**
- Phases 3 & 4 can run in parallel after Phase 1
- Phase 6 can start once Phase 3 video output is ready
- Phase 7 can start once Phase 5 orchestration is functional

---

## Technology Stack

| Layer | Technology |
|-------|------------|
| Video Pipeline | DeepStream 7.x, NVDEC, NvInfer |
| Detection | YOLOv8n TensorRT, NvDCF tracker |
| VLM Primary | Qwen2.5-VL-7B |
| VLM Fast | VILA-2.7B |
| Model Serving | Triton Inference Server |
| Streaming | WebRTC, HLS |
| IPC | Protocol Buffers, WebSocket |
| Language | Rust (host), C++ (DeepStream) |

---

## Memory Budget (Jetson AGX Orin 64GB)

| Component | Allocation |
|-----------|------------|
| DeepStream pipeline | 4GB |
| YOLOv8n TensorRT | 500MB |
| Qwen2.5-VL-7B (Q4) | 8GB |
| VILA-2.7B (Q4) | 3GB |
| Triton overhead | 2GB |
| Video buffers | 2GB |
| Context store | 1GB |
| System reserve | 4GB |
| **Available** | ~38GB |

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Video decode | <5ms/frame |
| Object detection | <10ms/frame |
| VLM inference (7B) | <5s |
| VLM inference (3B) | <2s |
| WebRTC latency | <200ms |
| HLS latency | 8-12s |
| Concurrent streams | 4-8 |

---

## Master Checklist

### Phase 1: Abstraction Layer Foundation
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

### Phase 2: Remote Event Bus & Authentication
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

### Phase 3: DeepStream Video Pipeline
- [ ] 3.1.1 Create DeepStream app skeleton
- [ ] 3.1.2 Configure multi-source RTSP
- [ ] 3.1.3 Add YOLOv8n primary inference
- [ ] 3.1.4 Configure NvDCF tracker
- [ ] 3.1.5 Add H.264 encode output
- [ ] 3.2.1 Add probe to extract NvDsMeta
- [ ] 3.2.2 Convert to Protobuf
- [ ] 3.2.3 Publish to event bus
- [ ] 3.2.4 Add frame extraction probe
- [ ] 3.3.1 Create Dockerfile
- [ ] 3.3.2 Add config volume mount
- [ ] 3.3.3 Add event bus connection
- [ ] 3.3.4 Create docker-compose entry

### Phase 4: Triton Model Serving
- [ ] 4.1.1 Convert YOLOv8n to TensorRT
- [ ] 4.1.2 Create YOLOv8 Triton config
- [ ] 4.1.3 Setup Qwen2.5-VL model
- [ ] 4.1.4 Create VLM Triton config
- [ ] 4.2.1 Create gRPC client wrapper
- [ ] 4.2.2 Implement YOLOv8 inference
- [ ] 4.2.3 Implement VLM inference
- [ ] 4.2.4 Add model management calls
- [ ] 4.3.1 Implement model switch handler
- [ ] 4.3.2 Add model state tracking
- [ ] 4.3.3 Memory-aware loading
- [ ] 4.3.4 Add model switch API

### Phase 5: Multi-Stage VLM Orchestration
- [ ] 5.1.1 Define `VideoCharacteristics` proto
- [ ] 5.1.2 Create characterization prompt
- [ ] 5.1.3 Implement sample frame extraction
- [ ] 5.1.4 Parse VLM response to proto
- [ ] 5.1.5 Cache characteristics per source
- [ ] 5.2.1 Define model registry
- [ ] 5.2.2 Implement strategy selection
- [ ] 5.2.3 GPU memory management
- [ ] 5.2.4 Dynamic model loading
- [ ] 5.2.5 Model unloading for memory
- [ ] 5.2.6 Pipeline configuration builder
- [ ] 5.3.1 Parallel model execution
- [ ] 5.3.2 Result merging
- [ ] 5.3.3 Trigger evaluation
- [ ] 5.3.4 VLM scheduling
- [ ] 5.3.5 Specialized model invocation
- [ ] 5.4.1 Subscribe to DeepStream detections
- [ ] 5.4.2 Implement filtering logic
- [ ] 5.4.3 Publish filtered detections
- [ ] 5.4.4 Add detection aggregation
- [ ] 5.5.1 Event-based triggers
- [ ] 5.5.2 Periodic triggers
- [ ] 5.5.3 On-demand triggers
- [ ] 5.5.4 Rate limiting
- [ ] 5.5.5 Priority queue
- [ ] 5.6.1 Ring buffer storage
- [ ] 5.6.2 Store detection history
- [ ] 5.6.3 Store VLM descriptions
- [ ] 5.6.4 Semantic search (optional)
- [ ] 5.7.1 Parse `VideoQueryRequest`
- [ ] 5.7.2 Gather context
- [ ] 5.7.3 Build VLM prompt
- [ ] 5.7.4 Execute VLM inference
- [ ] 5.7.5 Format response

### Phase 6: Video Streaming
- [ ] 6.1.1 STUN/TURN setup
- [ ] 6.1.2 SDP offer/answer
- [ ] 6.1.3 Video track from DeepStream
- [ ] 6.1.4 Detection overlay (optional)
- [ ] 6.2.1 Segment generation
- [ ] 6.2.2 M3U8 playlist server
- [ ] 6.2.3 Segment cleanup
- [ ] 6.3.1 WebRTC client (Mac)
- [ ] 6.3.2 Decode to egui texture
- [ ] 6.3.3 HLS fallback
- [ ] 6.3.4 Detection overlay

### Phase 7: Tool System & Agent Integration
- [ ] 7.1.1 `get_current_frame` tool
- [ ] 7.1.2 `get_detections` tool
- [ ] 7.1.3 `analyze_frame` tool
- [ ] 7.1.4 `search_history` tool
- [ ] 7.1.5 `track_object` tool
- [ ] 7.2.1 Tool execution router
- [ ] 7.2.2 Connect to providers
- [ ] 7.2.3 Async execution
- [ ] 7.2.4 Error handling
- [ ] 7.3.1 Register tools with agent
- [ ] 7.3.2 Tool call parsing
- [ ] 7.3.3 Execute and return
- [ ] 7.3.4 Streaming results

### Phase 8: End-to-End Integration
- [ ] 8.1.1 Configure standalone
- [ ] 8.1.2 Test video playback
- [ ] 8.1.3 Test VLM inference
- [ ] 8.1.4 Test agent chat
- [ ] 8.2.1 Start Jetson server
- [ ] 8.2.2 Connect Mac client
- [ ] 8.2.3 Verify video stream (WebRTC < 200ms)
- [ ] 8.2.4 Verify detections
- [ ] 8.2.5 Verify VLM queries (< 5s)
- [ ] 8.3.1 Configure headless
- [ ] 8.3.2 Access via browser
- [ ] 8.3.3 Test RTSP input
- [ ] 8.3.4 Test HLS output

---

## Resources

### NVIDIA Documentation
- [DeepStream SDK](https://developer.nvidia.com/deepstream-sdk)
- [Triton Inference Server](https://docs.nvidia.com/deeplearning/triton-inference-server/)
- [Jetson Platform Services VLM](https://docs.nvidia.com/jetson/jps/inference-services/vlm.html)

### NGC Containers
- `nvcr.io/nvidia/deepstream-l4t:7.0-triton-multiarch`
- `nvcr.io/nvidia/tritonserver:24.05-py3-igpu`

---

## Success Criteria

The implementation is complete when:

1. **Mac Standalone** works end-to-end with local VLM
2. **Mac + Jetson** connects and streams with <200ms latency
3. **Jetson Standalone** serves web UI with HLS playback
4. All performance targets are met
5. All 122 tasks are completed
6. Integration tests pass on all three modes
