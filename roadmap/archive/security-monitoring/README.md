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

### Phase 0: Pre-Implementation Setup (6 days)
Critical fixes identified by agent team review. Must complete before Phase 1.

| Milestone | Description | Days | Status |
|-----------|-------------|------|--------|
| 0.1 | NVIDIA Configuration Fixes | 1 | Pending |
| 0.2 | Cargo Dependencies | 1 | ✅ Done |
| 0.3 | Design System Updates | 1 | ✅ Done |
| 0.4 | UX Foundations | 3 | Pending |

**Implemented Fixes:**
- ✅ Added WCAG AA accessibility info to design tokens (contrast ratios documented)
- ✅ Added Pewter color (#8E8E99) as WCAG-compliant tertiary text
- ✅ Added comprehensive UI state patterns (loading, empty, error, progress, reconnecting)
- ✅ Added missing Cargo workspace dependencies (dashmap, uuid, hmac, sha2, webrtc, etc.)
- ✅ Added `schema_version` field to Protocol Buffer Envelope message
- ✅ Replaced `RwLock<HashMap>` with `DashMap` in event bus (both platforms)

**Pending Fixes (Milestone 0.4 - UX Foundations):**
- [ ] 0.4.1 Document user personas (3 types)
- [ ] 0.4.2 Create user journey maps (3 journeys)
- [ ] 0.4.3 Define keyboard navigation patterns

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
| Phases | 8 + Phase 0 (Pre-work) |
| Milestones | 42 |
| Tasks | 186 |
| Estimated Duration | 112-120 days (~24 weeks) |

> **Note:** Revised estimate includes agent review additions: UX improvements (onboarding, progress indicators, accessibility), architecture hardening (graceful shutdown, backpressure, DashMap), and Privacy Agent (+10-13 days from previous estimate).

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

---

## UX Foundations (Phase 0.4)

### User Personas

| Persona | Description | Primary Goals | UX Priorities |
|---------|-------------|---------------|---------------|
| **Security Operator** | Monitors live feeds, responds to alerts | Real-time awareness, quick incident response | Speed, clarity, mobile-friendly |
| **Facility Manager** | Reviews summaries, generates reports | Historical analysis, compliance reporting | Simplicity, data visualization, export |
| **ML Engineer** | Debugs models, optimizes performance | Model metrics, pipeline visibility | Transparency, debugging tools, API access |

### User Journey Maps

**Journey 1: Security Operator Investigates Alert**
```
1. Alert received → "Person detected in restricted area"
2. Click alert → Jump to camera + timestamp
3. Ask: "What was this person doing?" → VLM analyzes
4. Response: "Person carrying box, walking toward exit"
5. Scrub timeline → Review previous 5 minutes
6. Export → Save incident as PDF report
```

**Journey 2: Facility Manager Reviews Daily Activity**
```
1. Open dashboard → See 24-hour summary
2. Filter by camera → Select "Loading Dock"
3. View metrics → Vehicle count, peak hours, dwell times
4. Ask: "Any unusual activity?" → VLM summary
5. Export → Generate daily report
```

**Journey 3: ML Engineer Debugs Low Detection Accuracy**
```
1. Open model metrics → See accuracy/latency trends
2. View tool call history → Trace failed detections
3. Query: "Why is confidence low on cam3?" → VLM analyzes
4. Adjust settings → Change confidence threshold
5. Validate → Compare before/after metrics
```

### Keyboard Navigation Patterns

| Action | Shortcut | Context |
|--------|----------|---------|
| Switch camera | `1-9` | Grid view |
| Next/Previous camera | `← →` | Single view |
| Play/Pause | `Space` | Video playback |
| Full screen | `F` | Any view |
| Send query | `Enter` | Chat input |
| Focus chat | `Cmd+K` / `Ctrl+K` | Global |
| Open settings | `Cmd+,` / `Ctrl+,` | Global |
| Escape modal | `Esc` | Modal dialogs |
| Toggle detection overlay | `D` | Video view |
| Toggle privacy blur | `P` | Video view |

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
- [Model Strategy](./model-strategy.md)
- [Summary & Checklist](./SUMMARY.md)

---

## NVIDIA Reference Architecture

The architecture aligns with NVIDIA's published patterns for agentic video workflows:

| Reference | Description | Relevance |
|-----------|-------------|-----------|
| [Agentic Video Workflow](https://developer.nvidia.com/blog/build-an-agentic-video-workflow-with-video-search-and-summarization/) | VLM + RAG + tool orchestration | Core architecture pattern |
| [Jetson Platform Services](https://docs.nvidia.com/jetson/jps/moj-overview.html) | Microservices stack for Jetson | API alignment |
| [Jetson AI Lab](https://www.jetson-ai-lab.com/) | Live VLM tutorials | Implementation guidance |

### Jetson Thor Capabilities

| Spec | Value | Impact |
|------|-------|--------|
| Memory | 128GB LPDDR5X | Can run 70B+ models |
| GPU | Blackwell 2560 cores | 7.5x AI compute over Orin |
| FP4 Support | Native | 2x model capacity vs FP8 |
| Video Decode | 10x 4Kp60 | Far exceeds 4-8 stream target |

---

## Reference Links

### NVIDIA Official
- [Jetson AI Lab](https://www.jetson-ai-lab.com/)
- [Live VLM WebUI Tutorial](https://www.jetson-ai-lab.com/tutorials/live-vlm-webui/)
- [Jetson Platform Services](https://docs.nvidia.com/jetson/jps/moj-overview.html)
- [JPS GitHub](https://github.com/NVIDIA-AI-IOT/jetson-platform-services)
- [Agentic Video Workflow Blueprint](https://developer.nvidia.com/blog/build-an-agentic-video-workflow-with-video-search-and-summarization/)
- [Jetson Thor Technical Blog](https://developer.nvidia.com/blog/introducing-nvidia-jetson-thor-the-ultimate-platform-for-physical-ai/)
- [DeepStream 7.0 Docs](https://docs.nvidia.com/metropolis/deepstream/7.0/text/DS_Overview.html)
- [TensorRT-LLM GitHub](https://github.com/NVIDIA/TensorRT-LLM)
- [NVIDIA NIM Microservices](https://www.nvidia.com/en-us/ai-data-science/products/nim-microservices/)
- [Metropolis for Developers](https://developer.nvidia.com/metropolis)

### Tool-Reasoning Models
- [Berkeley Function Calling Leaderboard (BFCL)](https://gorilla.cs.berkeley.edu/leaderboard.html)
- [Groq Llama-3-Tool-Use Models](https://groq.com/blog/introducing-llama-3-groq-tool-use-models)
- [NexusRaven-V2](https://github.com/nexusflowai/NexusRaven-V2)
- [Qwen2.5-VL Technical Report](https://arxiv.org/abs/2502.13923)
- [LLaVA-Mini](https://arxiv.org/html/2501.03895v1)
- [Fine-tuning for Function Calling (xLAM)](https://huggingface.co/learn/cookbook/en/function_calling_fine_tuning_llms_on_xlam)

### Sample Code & Repos
- [NVIDIA-AI-IOT/deepstream_reference_apps](https://github.com/NVIDIA-AI-IOT/deepstream_reference_apps)
- [NVIDIA-AI-IOT/mmj_genai](https://github.com/NVIDIA-AI-IOT/mmj_genai)
- [DeepStream-Yolo](https://github.com/marcoslucianops/DeepStream-Yolo)
- [ToolBench](https://github.com/OpenBMB/ToolBench)

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
