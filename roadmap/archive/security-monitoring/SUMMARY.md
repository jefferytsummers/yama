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
**Duration:** 12 days | **Tasks:** 21

Enables authenticated remote connections between clients and Jetson.

| Milestone | Description |
|-----------|-------------|
| 2.1 | Event bus authentication (HMAC-SHA256 challenge-response) |
| 2.2 | Remote session protocol (session management, heartbeat) |
| 2.3 | Remote provider implementations (`RemoteAIServer`, reconnection) |
| 2.4 | Architecture hardening (graceful shutdown, backpressure, DashMap) |
| 2.5 | UX improvements (connection indicators, auth feedback) |

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
**Duration:** 14 days | **Tasks:** 19

Multi-model inference server with dynamic loading and tool-reasoning models.

| Milestone | Description |
|-----------|-------------|
| 4.1 | Model repository setup (YOLOv8 TensorRT, Qwen2.5-VL) |
| 4.2 | Triton Rust client (gRPC, YOLO inference, VLM inference) |
| 4.3 | Dynamic model loading (memory management, hot-swap) |
| 4.4 | Tool-reasoning model integration (Llama-3.1-8B, BFCL evaluation) |
| 4.5 | Jetson Thor optimization (FP4, MIG, 70B model support) |

**Deliverables:**
- `containers/triton/model_repository/`
- `platform/jetson/triton-client/`
- `ai-server/src/triton/model_manager.rs`
- Tool-calling model configuration

---

### Phase 5: Multi-Stage VLM Orchestration
**Duration:** 36 days | **Tasks:** 51

Intelligent multi-pass video analysis with model routing and agentic architecture.

| Milestone | Description |
|-----------|-------------|
| 5.1 | Video characterization VLM (scene type, environment, content hints) |
| 5.2 | Model router (strategy selection, GPU memory, dynamic loading) |
| 5.3 | Multi-model execution engine (parallel execution, result merging) |
| 5.4 | Detection router & aggregation (filtering, cross-stream) |
| 5.5 | VLM trigger system (event-based, periodic, rate limiting) |
| 5.6 | Context store (ring buffer, history queries) |
| 5.7 | Query handler (context building, prompt construction) |
| 5.8 | Architecture hardening (orchestrator split, dashmap, safe hot-swap) |
| 5.9 | Agentic architecture (ReAct loop, parallel tools, error recovery) |
| 5.10 | UX improvements (streaming responses, progress indicators) |
| 5.11 | Agent design improvements (tool discovery, audit logging, access control) |
| 5.12 | Safety & coordination (content moderation, Mac↔Jetson handoff) |

**Deliverables:**
- `ai-server/src/orchestrator/characterization.rs`
- `ai-server/src/orchestrator/model_router.rs`
- `ai-server/src/orchestrator/execution_engine.rs`
- `ai-server/src/orchestrator/detection_router.rs`
- `ai-server/src/orchestrator/vlm_trigger.rs`
- `ai-server/src/orchestrator/context_store.rs`
- `ai-server/src/orchestrator/query_handler.rs`
- `ai-server/src/orchestrator/react_loop.rs`
- `ai-server/src/orchestrator/tool_registry.rs`
- `ai-server/src/orchestrator/audit.rs`

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
**Duration:** 16 days | **Tasks:** 30

Expose video analysis as agent tools with LLM-optimized schemas and fine-tuning.

| Milestone | Description |
|-----------|-------------|
| 7.1 | Tool definitions (`get_current_frame`, `get_detections`, etc.) |
| 7.2 | Tool handlers (execution router, provider integration) |
| 7.3 | Agent integration (tool registry, reasoning loop) |
| 7.4 | Discovery and control tools (list_sources, access control, audit, ProxyTool) |
| 7.5 | Tool schema for LLM (BFCL-compatible JSON schema, streaming) |
| 7.6 | Fine-tuning pipeline (training data, LoRA, evaluation) |
| 7.7 | UX improvements (execution indicators, friendly messages, history panel) |

**Deliverables:**
- `shared/agent-sdk/src/tools/video.rs`
- `shared/agent-sdk/src/tools/handlers.rs`
- `shared/agent-sdk/src/tools/schema.rs`
- `shared/agent-sdk/src/tools/discovery.rs`
- `shared/agent-sdk/src/tools/proxy.rs`
- `shared/agent-sdk/src/tools/errors.rs`
- `containers/agent/src/tools.rs`
- `scripts/finetune/` (training pipeline)

---

### Phase 8: End-to-End Integration
**Duration:** 15 days | **Tasks:** 31

All components working together with NVIDIA reference alignment, Thor optimization, and Privacy Agent.

| Milestone | Description |
|-----------|-------------|
| 8.1 | Mac Standalone E2E (local video, VLM, chat) |
| 8.2 | Mac + Jetson E2E (remote connection, WebRTC, queries) |
| 8.3 | Jetson Standalone E2E (headless, web UI, HLS) |
| 8.4 | UX improvements (onboarding, state indicators, accessibility audit) |
| 8.5 | NVIDIA reference alignment (JPS patterns, API conventions) |
| 8.6 | Jetson Thor optimization (FP4, MIG, 70B models, 10+ streams) |
| 8.7 | Privacy Agent (face detection, blur regions, PII detection, power budget) |

**Deliverables:**
- `scripts/test-standalone.sh`
- `scripts/test-mac-jetson.sh`
- `scripts/test-jetson-standalone.sh`
- `docker-compose.yml` (all modes)
- Thor-specific configuration files
- `ai-server/src/agents/privacy.rs`
- `shared/agent-sdk/src/tools/privacy.rs`

---

## Total Effort

| Metric | Value |
|--------|-------|
| **Phases** | 8 + Phase 0 (Pre-work) |
| **Milestones** | 44 |
| **Tasks** | 192 |
| **Estimated Duration** | 118-128 days (~26 weeks) |

> **Note:** Revised estimate includes agent review additions:
> - UX improvements, architecture hardening, Privacy Agent
> - Accessibility audit, streaming responses
> - **5.8.1 orchestrator split is BLOCKING** for Phase 6+
> - Safety & coordination milestone (content moderation, Mac↔Jetson)
>
> This adds approximately 16-21 days from original 107-day estimate.

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

| Component | Allocation | Notes |
|-----------|------------|-------|
| DeepStream pipeline | 4GB | |
| YOLOv8n TensorRT (INT8) | 250MB | Reduced with INT8 quantization |
| Qwen2.5-VL-7B (Q4 weights + FP16 vision) | 5.7GB | 4.2GB Q4 + 1.5GB FP16 vision encoder |
| VILA-2.7B (Q4 + FP16 vision) | 2.4GB | 1.6GB Q4 + 0.8GB FP16 vision encoder |
| Triton overhead | 2GB | |
| Video buffers | 2GB | |
| Context store | 1GB | |
| System reserve | 4GB | |
| **Available** | ~42GB | |

> **Note:** VLMs with vision encoders require FP16 for the vision component, increasing memory vs text-only LLMs.

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

### Phase 0: Pre-Implementation Setup (CRITICAL)
- [ ] 0.1.1 Fix Triton container to `py3-jetpack` (not `py3-igpu` or `jetpack-py3`)
- [ ] 0.1.2 Update VLM backend to TensorRT-LLM (not Python)
- [ ] 0.1.3 Verify YOLOv8 I/O names with `polygraphy inspect`
- [x] 0.2.1 Add missing Cargo workspace dependencies (webrtc, hmac, sha2, uuid, dashmap, base64, parking_lot, tokio-util)
- [x] 0.2.2 Add message schema versioning to Envelope proto (`schema_version` field added)
- [x] 0.3.1 Define loading/empty/error state patterns in design system (comprehensive `uiStates` object added)
- [x] 0.3.2 Fix accessibility contrast issue (added Pewter color, documented contrast ratios, marked Ash as decorative-only)
- [ ] 0.3.3 Specify animation parity between CSS and egui
- [x] 0.3.4 Replace `RwLock<HashMap>` with `dashmap` in event bus (both Apple and Jetson platforms)

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
- [ ] 1.1.6 Add `Service` trait for lifecycle management
- [ ] 1.1.7 Rename `AIServerProvider` → `VlmProvider`
- [ ] 1.1.8 Move `ContextStore` to separate crate

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
- [ ] 2.4.1 Add message schema versioning to Envelope
- [ ] 2.4.2 Implement backpressure/flow control
- [ ] 2.4.3 Add graceful shutdown with CancellationToken
- [ ] 2.4.4 Replace RwLock<HashMap> with DashMap
- [ ] 2.4.5 Handle broadcast receiver lag
- [ ] 2.5.1 Add connection state indicators (UX)
- [ ] 2.5.2 Show authentication failure reasons (UX)
- [ ] 2.5.3 Add reconnection progress UI (UX)
- [ ] 2.5.4 Add latency indicator (UX)

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
- [ ] 3.4.1 Use INT8 quantization for YOLOv8
- [ ] 3.4.2 Direct TensorRT export (skip ONNX)
- [ ] 3.4.3 Fix nvstreammux timing (33333μs)
- [ ] 3.4.4 Use shared memory for FrameSample

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
- [ ] 4.4.1 Add Llama-3.1-8B (Groq variant) to model repository
- [ ] 4.4.2 Configure tool-calling inference endpoint
- [ ] 4.4.3 Add BFCL-style evaluation for video tool schema
- [ ] 4.4.4 Document model selection rationale
- [ ] 4.5.1 Enable FP4 quantization path for Jetson Thor
- [ ] 4.5.2 Configure MIG for mixed workloads
- [ ] 4.5.3 Update memory budget for 128GB platform
- [ ] 4.5.4 Benchmark 70B model feasibility for complex queries

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
- [ ] 5.8.1 **[BLOCKING]** Refactor orchestrator into 4 components (Characterizer, Scheduler, Executor, Aggregator)
- [ ] 5.8.2 Replace `RwLock<HashMap>` with `dashmap`
- [ ] 5.8.3 Use `spawn_blocking` for JPEG encoding
- [ ] 5.8.4 Add model reference counting for safe hot-swap
- [ ] 5.9.1 Implement ReAct-style tool selection loop
- [ ] 5.9.2 Add tool-calling schema validation
- [ ] 5.9.3 Implement parallel tool execution
- [ ] 5.9.4 Add error recovery for failed tool calls
- [ ] 5.9.5 Benchmark against NVIDIA Agentic Video Workflow
- [ ] 5.10.1 Add VLM streaming responses (UX)
- [ ] 5.10.2 Show model selection rationale (UX)
- [ ] 5.10.3 Add inference progress indicator (UX)
- [ ] 5.10.4 Display estimated time remaining (UX)
- [ ] 5.11.1 Add `list_tools()` discovery endpoint
- [ ] 5.11.2 Add `get_model_capabilities()` introspection
- [ ] 5.11.3 Add tool execution audit logging
- [ ] 5.11.4 Add per-source access control
- [ ] 5.11.5 Add VLM content moderation (filter harmful output)
- [ ] 5.11.6 Document agent coordination pattern (Mac↔Jetson handoff)
- [ ] 5.12.1 Implement VLM content filter
- [ ] 5.12.2 Add PII redaction patterns
- [ ] 5.12.3 Define agent coordination protocol
- [ ] 5.12.4 Implement tool forwarding (Mac→Jetson)

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
- [ ] 7.4.1 Add `list_sources` discovery tool
- [ ] 7.4.2 Add `list_tracks` discovery tool
- [ ] 7.4.3 Add `get_status` discovery tool
- [ ] 7.4.4 Implement streaming tool results for VLM calls
- [ ] 7.4.5 Add tool-level access control
- [ ] 7.4.6 Add audit logging for tool invocations
- [ ] 7.4.7 Define video-specific error types
- [ ] 7.5.1 Define JSON schema for all video tools (BFCL-compatible)
- [ ] 7.5.2 Add tool descriptions optimized for LLM understanding
- [ ] 7.5.3 Implement streaming tool results for long operations
- [ ] 7.5.4 Add tool-level access control per source
- [ ] 7.6.1 Create video tool-use training dataset (500-1000 examples)
- [ ] 7.6.2 Set up LlamaFactory for LoRA fine-tuning
- [ ] 7.6.3 Fine-tune Llama-3.1-8B on custom video tool schema
- [ ] 7.6.4 Evaluate fine-tuned vs base model accuracy
- [ ] 7.4.8 Implement ProxyTool pattern for remote deployment
- [ ] 7.7.1 Add tool execution indicator (UX)
- [ ] 7.7.2 Show tool results summary in user-friendly format (UX)
- [ ] 7.7.3 Display tool errors as user-friendly messages (UX)
- [ ] 7.7.4 Add tool execution history panel (UX)

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
- [ ] 8.4.1 Add onboarding flows for each deployment mode (UX)
- [ ] 8.4.2 Add connection state indicators in UI (UX)
- [ ] 8.4.3 Add loading/empty/error states (UX)
- [ ] 8.4.4 Conduct WCAG AA accessibility audit (UX)
- [ ] 8.5.1 Review JPS VLM Service implementation patterns
- [ ] 8.5.2 Align REST API with NVIDIA conventions where sensible
- [ ] 8.5.3 Document architectural differences from JPS
- [ ] 8.5.4 Consider JPS compatibility layer for enterprise deployments
- [ ] 8.6.1 Enable FP4 quantization for supported models
- [ ] 8.6.2 Configure MIG for multi-tenant workloads
- [ ] 8.6.3 Update memory budget for 128GB platform
- [ ] 8.6.4 Benchmark 70B model feasibility for complex queries
- [ ] 8.6.5 Test 10-stream concurrent processing on Thor
- [ ] 8.7.1 Implement Privacy Agent service
- [ ] 8.7.2 Add `detect_faces` tool
- [ ] 8.7.3 Add `blur_regions` tool
- [ ] 8.7.4 Add `detect_pii` tool
- [ ] 8.7.5 Add `get_system_info` deployment awareness tool
- [ ] 8.7.6 Add power budget config for Thor

---

## Resources

### Model Strategy
See [Model Strategy](./model-strategy.md) for detailed model selection rationale, architecture options (dual-model vs single-model), and fine-tuning strategy.

### NVIDIA Documentation
- [DeepStream SDK](https://developer.nvidia.com/deepstream-sdk)
- [Triton Inference Server](https://docs.nvidia.com/deeplearning/triton-inference-server/)
- [Jetson Platform Services VLM](https://docs.nvidia.com/jetson/jps/inference-services/vlm.html)
- [Jetson AI Lab](https://www.jetson-ai-lab.com/)
- [Agentic Video Workflow Blueprint](https://developer.nvidia.com/blog/build-an-agentic-video-workflow-with-video-search-and-summarization/)
- [Jetson Thor Technical Blog](https://developer.nvidia.com/blog/introducing-nvidia-jetson-thor-the-ultimate-platform-for-physical-ai/)

### NGC Containers
- `nvcr.io/nvidia/deepstream-l4t:7.0-samples-jetpack` (NOT `multiarch` or `py3-igpu`)
- `nvcr.io/nvidia/tritonserver:24.05-py3-jetpack` (NOT `jetpack-py3`)

### Tool-Reasoning Models
- [Berkeley Function Calling Leaderboard (BFCL)](https://gorilla.cs.berkeley.edu/leaderboard.html)
- [Groq Llama-3-Tool-Use Models](https://groq.com/blog/introducing-llama-3-groq-tool-use-models)
- [NexusRaven-V2](https://github.com/nexusflowai/NexusRaven-V2)
- [Fine-tuning for Function Calling (xLAM)](https://huggingface.co/learn/cookbook/en/function_calling_fine_tuning_llms_on_xlam)

---

## Success Criteria

The implementation is complete when:

1. **Mac Standalone** works end-to-end with local VLM
2. **Mac + Jetson** connects and streams with <200ms latency
3. **Jetson Standalone** serves web UI with HLS playback
4. All performance targets are met
5. All 192 tasks are completed
6. Integration tests pass on all three modes
7. Accessibility audit passes WCAG AA standards
8. Privacy Agent tools functional for face/PII detection
9. VLM content moderation active for all responses
10. Agent coordination protocol verified (Mac↔Jetson)
