# Yama: Agent-First Video Analysis Platform

## Executive Summary

Yama is a conversational video analysis platform where users interact through natural language with AI agent presets. Instead of traditional "import → search → extract" workflows, the agent orchestrates tools autonomously to accomplish complex video analysis tasks.

**Target:** 100GB-10TB video libraries with local-first, privacy-preserving analysis on Mac, with optional edge deployment on Jetson Thor.

---

## Primary Persona

| Attribute | Value |
|-----------|-------|
| **Name** | Content Analyst |
| **Role** | Researcher, journalist, analyst |
| **Mindset** | "I have too much footage and not enough time" |

See [personas/content-analyst.md](./personas/content-analyst.md) for full user stories.

---

## Agent Presets

Users select a preset that bundles models and capabilities:

| Preset | Models | Capabilities |
|--------|--------|--------------|
| **Video Analyst** | Qwen2.5-VL, YOLOv8, RTMPose, CLIP | Find moments, detect objects, track motion |
| **Document Reporter** | SmolDockling, Whisper, Gemma-3, CLIP | OCR slides, transcribe, generate reports |
| **Quick Search** | CLIP, Whisper-tiny | Fast visual + transcript search |
| **Custom** | User-selected | Domain-specific workflows |

---

## Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Platform | macOS (primary), Jetson Thor (edge) | Target user environments |
| Video Decode | GStreamer + VideoToolbox/NVDEC | Hardware acceleration |
| Index Storage | SQLite + FTS5 + sqlite-vss | Fast local search with ANN |
| Embeddings | CLIP (ViT-B/32) | Semantic video search |
| Transcription | Whisper (whisper.cpp) | Speech-to-text |
| VLM Inference | GGUF + MLX (Mac) / TensorRT-LLM (Jetson) | Local inference |
| Edge Video | DeepStream 7.x | Multi-stream analytics |
| UI Framework | egui (native) / SvelteKit (web) | Cross-platform |

---

## Roadmap Structure

```
Phase 1: Core Infrastructure (10d)
         │
         ▼
Phase 2: Indexing & Embeddings (12d)
         │
         ▼
Phase 3: Agent System (14d)
         │
         ├───────────────────────┐
         ▼                       ▼
   Mac Release          Phase 4: Jetson Thor (20d)
```

---

## Phase Overview

| Phase | Name | Days | Focus |
|-------|------|------|-------|
| 1 | Core Infrastructure | 10 | SQLite, batch processor, GStreamer, model manager |
| 2 | Indexing & Embeddings | 12 | CLIP, Whisper, sqlite-vss, text embeddings |
| 3 | Agent System | 14 | Presets, tool executor, chat interface, artifacts |
| 4 | Jetson Thor Deployment | 20 | DeepStream, Triton, WebRTC, edge optimization |
| **Total** | | **56 days** | |

---

## Workflow

```
1. SELECT PRESET
   └─ Choose Video Analyst, Document Reporter, Quick Search, or Custom

2. ADD VIDEOS
   └─ Drag folders into project (auto-indexes in background)

3. CONVERSE
   └─ "Find all mentions of 'Q4 projections' with visible charts"

4. AGENT WORKS
   ├─ Searches transcripts + visual content
   ├─ Analyzes matching frames with VLM
   └─ Extracts clips automatically

5. REVIEW ARTIFACTS
   ├─ Watch extracted clips
   ├─ Read generated summaries
   └─ Export reports
```

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Time to first result (50 videos) | < 5 minutes (initial index) |
| Query response time (indexed) | < 3 seconds |
| Clip extraction throughput | 10 clips/minute |
| VLM analysis latency | < 5 seconds |

---

## Deployment Modes

| Mode | Client | AI Server | Use Case |
|------|--------|-----------|----------|
| Mac Standalone | Mac (egui) | Mac (local) | Development, single user |
| Mac + Jetson | Mac (egui) | Jetson (remote) | Production, distributed |
| Jetson Standalone | Web / headless | Jetson (local) | Edge deployment |

---

## Non-Goals (MVP)

Explicitly out of scope:
- Real-time live monitoring (post-event analysis only)
- Multi-user collaboration
- Cloud storage integration
- Mobile native apps

---

## Phase Documents

- [Phase 1: Core Infrastructure](./phases/phase-1-core-infrastructure.md)
- [Phase 2: Indexing & Embeddings](./phases/phase-2-indexing-embeddings.md)
- [Phase 3: Agent System](./phases/phase-3-agent-system.md)
- [Phase 4: Jetson Thor Deployment](./phases/phase-4-jetson-deployment.md)

---

## Supporting Documents

- [Model Strategy](./model-strategy.md) - Model selection for Whisper, CLIP, VLM
- [Summary & Checklist](./SUMMARY.md) - Implementation tracking
- [Content Analyst Persona](./personas/content-analyst.md) - User stories

---

## Archived Roadmaps

Previous roadmap iterations have been archived:

- [Archive: Content Analyst Traditional](./archive/content-analyst-traditional/) - Original import→search→extract UX
- [Archive: Security Monitoring](./archive/security-monitoring/) - Real-time Jetson surveillance (technical reference for Phase 4)
