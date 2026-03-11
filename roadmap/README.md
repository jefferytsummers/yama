# Yama Development Roadmap

## Current Status: Parallel Development

Following completion of the unified VLM abstraction, development proceeds on two parallel tracks:

| Track | Machine | Focus |
|-------|---------|-------|
| **Jetson Backend** | Thor (Jetson Orin) | Triton integration, TensorRT-LLM, NVDEC pipelines |
| **Frontend UX** | Mac (Development) | SvelteKit UI, mock backends, user experience |

## Quick Links

| Document | Description |
|----------|-------------|
| [Unified VLM Plan](./unified-vlm-abstraction.md) | Architecture for cross-platform VLM inference |
| [Archive: Agent-First v1](./archive/agent-first-v1/) | Previous roadmap (Phases 1-4) |

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    VlmBackend Trait (Unified)                     │
└──────────────────────────────────────────────────────────────────┘
                              │
      ┌───────────────────────┼───────────────────────┐
      ▼                       ▼                       ▼
┌──────────────┐       ┌──────────────┐       ┌──────────────────┐
│DirectBackend │       │EventBusBackend│      │TritonVlmBackend │
│(Apple Metal) │       │(Container IPC)│      │(Jetson gRPC)    │
└──────────────┘       └──────────────┘       └──────────────────┘
```

## Development Tracks

### Track 1: Jetson Backend (Thor)

Complete the Triton Inference Server integration for Jetson Orin:

- [ ] Build TensorRT-LLM engine for Qwen2.5-VL
- [ ] Deploy Triton with docker-compose
- [ ] Test TritonVlmBackend end-to-end
- [ ] Implement NVDEC frame extraction
- [ ] Optimize for Orin memory constraints

**Files:**
- `platform/jetson/host/src/inference/`
- `platform/jetson/triton-models/`
- `platform/jetson/docker-compose.triton.yml`

### Track 2: Frontend UX (Mac)

Build the user interface with mock backends:

- [ ] Implement project management UI
- [ ] Build chat interface with SSE streaming
- [ ] Create video preview components
- [ ] Design inference job dashboard
- [ ] Add settings and model management

**Files:**
- `web/src/routes/`
- `web/src/lib/components/`
- `web/src/lib/stores/`

## Archived Roadmaps

| Archive | Description |
|---------|-------------|
| [Agent-First v1](./archive/agent-first-v1/) | Original 4-phase roadmap (completed) |
| [Content Analyst Traditional](./archive/content-analyst-traditional/) | Import→search→extract UX |
| [Security Monitoring](./archive/security-monitoring/) | Real-time Jetson surveillance |
