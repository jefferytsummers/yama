# Archived: Jetson AI Server Security Monitoring Roadmap

**Archived:** 2026-03-08
**Reason:** Pivot to Content Analyst persona (batch video analysis)
**Successor:** `/roadmap/` (Content Analyst roadmap)

## Summary

This roadmap defined a real-time security monitoring architecture:
- 4-8 concurrent RTSP streams
- DeepStream video pipeline on Jetson
- Triton model serving
- Mac + Jetson distributed deployment
- Real-time detection and alerts

## Why Archived

The project pivoted to the **Content Analyst** persona:
- Batch video analysis (not real-time)
- Local files (not RTSP streams)
- Mac-first (not Jetson-first)
- Search/export workflows (not monitoring)

## Contents

| File | Description |
|------|-------------|
| README.md | Original executive summary and architecture |
| SUMMARY.md | Implementation checklist (192 tasks) |
| phase-1-abstraction-layer.md | Platform traits, local/remote providers |
| phase-2-remote-event-bus.md | WebSocket auth, remote sessions |
| phase-3-deepstream-pipeline.md | DeepStream, NVDEC, YOLOv8 |
| phase-4-triton-serving.md | Triton server, model management |
| phase-5-vlm-orchestration.md | Multi-stage VLM, model routing |
| phase-6-video-streaming.md | WebRTC, HLS fallback |
| phase-7-tool-system.md | Agent tools for video analysis |
| phase-8-integration.md | E2E integration, deployment modes |

## Reusable Patterns

The following patterns from this roadmap were adapted for the Content Analyst roadmap:

1. **Abstraction Layer** (Phase 1) → Phase 1 Foundation (simplified)
2. **Local Inference** (Phase 4) → Phase 4 Inference (Metal focus)
3. **Tool System** (Phase 7) → Phase 7 Tools (batch operations)
4. **Context Store** (Phase 5) → Phase 2 Video Indexing (SQLite)

## Technology Decisions Preserved

- GStreamer for video decoding (hardware acceleration)
- GGUF models for LLM inference
- Protocol Buffers for structured data
- egui for native UI
