# Yama: Content Analyst Video Analysis Platform

## Executive Summary

Yama is a batch video analysis platform designed for the **Content Analyst** persona: researchers, journalists, and analysts who work with video archives to extract insights, find patterns, and produce deliverables.

**Target:** Process 100GB-10TB video libraries with local-first, privacy-preserving analysis.

---

## Primary Persona

| Attribute | Value |
|-----------|-------|
| **Name** | Content Analyst |
| **Role** | Researcher, journalist, analyst |
| **Environment** | Mac (primary), Linux workstation |
| **Mindset** | "I have too much footage and not enough time" |

See [personas/content-analyst.md](./personas/content-analyst.md) for full user stories.

---

## Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Platform | macOS (primary) | Target user environment |
| Video Decode | GStreamer + VideoToolbox | Hardware acceleration |
| Index Storage | SQLite + FTS5 + sqlite-vss | Fast local search with ANN |
| Embeddings | CLIP (ViT-B/32) | Semantic video search |
| Transcription | Whisper (whisper.cpp) | Speech-to-text |
| VLM Inference | GGUF + MLX | Local Metal inference |
| Clip Export | FFmpeg | Frame-accurate extraction |
| UI Framework | egui | Native cross-platform |

---

## Roadmap Structure

```
P0 UX Discovery (5-7d) ──┬── P0.5 GStreamer Spike (2d, parallel)
                         ↓
P1 Foundation (8d) → P2 Indexing + Vector (12d) → P3 Transcription (6d)
                                                        ↓
                   P4 Inference + Model Mgmt (9d) ──────┘
                         ↓
P5 Search + sqlite-vss (10d) → P6 Extraction (6d) → P7 Tools (8d) → P8 Export (5d)
```

---

## Phase Overview

| Phase | Name | Days | Focus |
|-------|------|------|-------|
| 0 | UX Discovery | 5-7 | Workflow mapping, wireframes, component inventory |
| 0.5 | GStreamer Spike | 2 | Validate CPU-buffer decode performance (parallel) |
| 1 | Foundation | 8 | Core traits, batch processing, indexing providers |
| 2 | Video Indexing | 12 | SQLite schema, CLIP embeddings, sqlite-vss vector index |
| 3 | Transcription | 6 | Whisper integration, FTS5 search |
| 4 | Inference | 9 | Local GGUF/MLX models, Metal acceleration, model management |
| 5 | Search & Retrieval | 10 | Semantic (ANN) + full-text search, query parsing |
| 6 | Clip Extraction | 6 | FFmpeg export, batch operations |
| 7 | Tool System | 8 | Content Analyst agent tools |
| 8 | Export & Reporting | 5 | Markdown, CSV, JSON reports |
| **Total** | | **66-70 days** | |

---

## Workflow

```
1. IMPORT
   └─ Drag folder of videos into Yama

2. INDEX (background)
   ├─ Transcription (Whisper)
   ├─ Key frame extraction
   ├─ CLIP embedding generation
   └─ SQLite metadata storage

3. QUERY
   └─ "Find all mentions of 'Q4 projections' with visible charts"

4. REVIEW
   ├─ Thumbnails + timestamps
   ├─ Confidence scores
   └─ Star/flag relevant results

5. EXTRACT
   └─ Export matching clips with padding

6. REPORT
   ├─ Generate summary markdown
   └─ Export CSV/JSON for further analysis
```

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Time to first result (50 videos) | < 5 minutes (initial index) |
| Query response time (indexed) | < 3 seconds |
| Clip extraction throughput | 10 clips/minute |
| False positive rate | < 20% for natural language queries |

---

## Memory Budget (Mac 16GB)

| Component | Allocation | Notes |
|-----------|------------|-------|
| Whisper (tiny.en) | 200MB | Fast transcription |
| Whisper (base.en) | 400MB | Better accuracy |
| CLIP (ViT-B/32) | 350MB | Semantic embeddings |
| VLM (7B Q4) | 4GB | Analysis queries |
| SQLite + index | 500MB | Metadata cache |
| Video decode | 500MB | GStreamer buffers |
| System reserve | 4GB | macOS, other apps |
| **Available headroom** | ~5GB | |

---

## Non-Goals (MVP)

Explicitly out of scope:
- Real-time monitoring / live streams
- Multi-user collaboration
- Cloud storage integration
- Mobile access
- Alert systems

---

## Phase Documents

- [Phase 0: UX Discovery](./phase-0-ux-discovery.md) *(includes GStreamer Spike)*
- [Phase 1: Foundation](./phase-1-foundation.md)
- [Phase 2: Video Indexing](./phase-2-video-indexing.md) *(+sqlite-vss)*
- [Phase 3: Transcription](./phase-3-transcription.md)
- [Phase 4: Inference](./phase-4-inference.md) *(+model management)*
- [Phase 5: Search & Retrieval](./phase-5-search.md) *(+ANN search)*
- [Phase 6: Clip Extraction](./phase-6-clip-extraction.md)
- [Phase 7: Tool System](./phase-7-tools.md)
- [Phase 8: Export & Reporting](./phase-8-export.md)

---

## Supporting Documents

- [Model Strategy](./model-strategy.md) - Model selection for Whisper, CLIP, VLM
- [Summary & Checklist](./SUMMARY.md) - Implementation tracking
- [Content Analyst Persona](./personas/content-analyst.md) - User stories

---

## Archived Roadmap

The previous security monitoring roadmap (Jetson + real-time streams) has been archived:

- [Archive: Security Monitoring](./archive/security-monitoring/)
