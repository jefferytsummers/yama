# Yama Jetson AI Server Architecture Roadmap

## Executive Summary

This roadmap defines the architecture and implementation plan for offloading AI operations from Mac/Web clients to a headless Jetson AI server. The Jetson runs continuous video analytics (DeepStream) and VLM inference (Triton), streaming results to thin clients for display and interaction.

**Target:** 4-8 concurrent 1080p/4K streams with real-time object detection + on-demand VLM queries.

---

## Deployment Modes

The architecture supports three deployment topologies through a unified abstraction layer:

| Mode | Client | AI Server | Video Source | Use Case |
|------|--------|-----------|--------------|----------|
| **Mac Standalone** | Mac (egui) | Mac (local) | Mac decode | Development, single user |
| **Mac + Jetson** | Mac (egui) | Jetson (remote) | Jetson decode | Production, distributed |
| **Jetson Standalone** | Web / headless | Jetson (local) | Jetson decode | Edge deployment |

---

## Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Video Pipeline | DeepStream 7.x | Native multi-stream batching, NVDEC, NvInfer |
| Detection | YOLOv8n TensorRT | Best speed/accuracy for real-time |
| VLM (Primary) | Qwen2.5-VL-7B | General analysis, queries |
| VLM (Fast) | VILA-2.7B | Real-time scene description |
| Model Serving | Triton Inference Server | Multi-model, dynamic batching |
| IPC | Protocol Buffers + WebSocket | Existing event bus, extended for remote |
| Video Streaming | WebRTC + HLS fallback | Low latency + compatibility |

---

## Milestone Overview

### Phase 1: Abstraction Layer Foundation (11 days)
Establish platform-agnostic traits enabling all deployment modes.

| Milestone | Description | Days |
|-----------|-------------|------|
| 1.1 | Core Trait Definitions | 3 |
| 1.2 | Request/Response Types | 2 |
| 1.3 | Apple Local Implementations | 4 |
| 1.4 | Deployment Configuration | 2 |

### Phase 2: Remote Event Bus & Authentication (10 days)
Enable authenticated remote connections between clients and Jetson.

| Milestone | Description | Days |
|-----------|-------------|------|
| 2.1 | Event Bus Authentication | 3 |
| 2.2 | Remote Session Protocol | 3 |
| 2.3 | Remote Provider Implementations | 4 |

### Phase 3: DeepStream Video Pipeline (10 days)
Multi-stream video analytics with hardware-accelerated detection.

| Milestone | Description | Days |
|-----------|-------------|------|
| 3.1 | DeepStream Pipeline Setup | 5 |
| 3.2 | Metadata Extraction | 3 |
| 3.3 | DeepStream Container | 2 |

### Phase 4: Triton Model Serving (9 days)
Multi-model inference server with dynamic loading.

| Milestone | Description | Days |
|-----------|-------------|------|
| 4.1 | Model Repository Setup | 3 |
| 4.2 | Triton Rust Client | 4 |
| 4.3 | Dynamic Model Loading | 2 |

### Phase 5: Multi-Stage VLM Orchestration (26 days)
Intelligent multi-pass video analysis with model routing.

| Milestone | Description | Days |
|-----------|-------------|------|
| 5.1 | Video Characterization VLM | 4 |
| 5.2 | Model Router | 5 |
| 5.3 | Multi-Model Execution Engine | 4 |
| 5.4 | Detection Router & Aggregation | 3 |
| 5.5 | VLM Trigger System | 4 |
| 5.6 | Context Store | 3 |
| 5.7 | Query Handler | 3 |

### Phase 6: Video Streaming (11 days)
Low-latency video delivery to remote clients.

| Milestone | Description | Days |
|-----------|-------------|------|
| 6.1 | WebRTC Server | 5 |
| 6.2 | HLS Fallback | 3 |
| 6.3 | Client Video Receiver | 3 |

### Phase 7: Tool System & Agent Integration (8 days)
Expose video analysis as agent tools.

| Milestone | Description | Days |
|-----------|-------------|------|
| 7.1 | Tool Definitions | 2 |
| 7.2 | Tool Handlers | 3 |
| 7.3 | Agent Integration | 3 |

### Phase 8: End-to-End Integration (7 days)
All components working together.

| Milestone | Description | Days |
|-----------|-------------|------|
| 8.1 | Mac Standalone E2E | 2 |
| 8.2 | Mac + Jetson E2E | 3 |
| 8.3 | Jetson Standalone E2E | 2 |

---

## Total Effort

| Metric | Value |
|--------|-------|
| Phases | 8 |
| Milestones | 29 |
| Tasks | 122 |
| Estimated Duration | 92 days (~19 weeks) |

---

## Critical Path

```
Phase 1 (Abstraction) ──▶ Phase 2 (Remote Bus) ──▶ Phase 8 (E2E)
         │                      │
         ▼                      ▼
    Phase 3 (DeepStream) ──▶ Phase 5 (Orchestration)
         │                      │
         ▼                      ▼
    Phase 4 (Triton) ───────────┘
                                │
                                ▼
                          Phase 6 (Streaming)
                                │
                                ▼
                          Phase 7 (Tools)
```

---

## Phase Documents

- [Phase 1: Abstraction Layer Foundation](./phase-1-abstraction-layer.md)
- [Phase 2: Remote Event Bus & Authentication](./phase-2-remote-event-bus.md)
- [Phase 3: DeepStream Video Pipeline](./phase-3-deepstream-pipeline.md)
- [Phase 4: Triton Model Serving](./phase-4-triton-serving.md)
- [Phase 5: Multi-Stage VLM Orchestration](./phase-5-vlm-orchestration.md)
- [Phase 6: Video Streaming](./phase-6-video-streaming.md)
- [Phase 7: Tool System & Agent Integration](./phase-7-tool-system.md)
- [Phase 8: End-to-End Integration](./phase-8-integration.md)
- [Summary & Checklist](./SUMMARY.md)

---

## Architecture Diagram

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
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │   │
│  │  │ Detection    │  │ VLM Trigger  │  │ Query        │          │   │
│  │  │ Router       │  │ (event-based)│  │ Handler      │          │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘          │   │
│  │                           │                                     │   │
│  │                           ▼                                     │   │
│  │  ┌──────────────────────────────────────────────────────────┐  │   │
│  │  │  Triton Inference Server                                  │  │   │
│  │  │  ├── YOLOv8n.engine (TensorRT)                           │  │   │
│  │  │  ├── Qwen2.5-VL-7B (TensorRT-LLM / mistral.rs)          │  │   │
│  │  │  └── CLIP embeddings (optional)                          │  │   │
│  │  └──────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                      │                                                  │
│                      ▼                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Remote Event Bus (extended)                                    │   │
│  │  ws://jetson:8765 (authenticated, remote-capable)               │   │
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

## File Structure Overview

```
yama/
├── shared/
│   ├── platform-traits/src/
│   │   ├── ai_server.rs          # AIServerProvider trait
│   │   ├── video.rs              # VideoProvider trait
│   │   ├── detection.rs          # DetectionProvider trait
│   │   ├── context.rs            # ContextStore trait
│   │   ├── streaming.rs          # StreamProvider trait
│   │   └── tools.rs              # ToolProvider trait
│   │
│   ├── ai-client/                # Unified client for all modes
│   │   └── src/
│   │       ├── local.rs          # Local implementations
│   │       ├── remote.rs         # Remote implementations
│   │       └── config.rs         # Deployment configuration
│   │
│   └── protocol/proto/
│       ├── vlm.proto             # Extended VLM messages
│       ├── remote.proto          # Remote session protocol
│       └── detection.proto       # Multi-stream detections
│
├── platform/
│   ├── apple/host/src/providers/ # Apple-specific providers
│   │
│   └── jetson/
│       ├── ai-server/            # Headless AI server
│       │   └── src/
│       │       ├── deepstream/   # DeepStream integration
│       │       ├── triton/       # Triton client
│       │       ├── orchestrator/ # AI orchestration
│       │       └── streaming/    # Video output
│       │
│       └── deepstream-config/    # DeepStream configs
│
└── containers/
    ├── triton/                   # Triton model repository
    └── deepstream/               # DeepStream app
```
