# Implementation Checklist

## Agent-First Video Analysis Roadmap

**Total Duration:** 56 days
**Target Persona:** Content Analyst (video researchers, footage reviewers)
**Approach:** Conversational AI with agent presets

---

## Phase Overview

| Phase | Name | Days | Status |
|-------|------|------|--------|
| 1 | Core Infrastructure | 10 | ⬜ Not Started |
| 2 | Indexing & Embeddings | 12 | ⬜ Not Started |
| 3 | Agent System | 14 | ⬜ Not Started |
| 4 | Jetson Thor Deployment | 20 | ⬜ Not Started |

---

## Phase 1: Core Infrastructure (10 days)

### Milestone 1.1: SQLite Schema & IndexProvider
- [ ] 1.1.1 Design schema (videos, keyframes, transcripts, detections)
- [ ] 1.1.2 Create FTS5 tables with triggers
- [ ] 1.1.3 Implement IndexProvider CRUD operations
- [ ] 1.1.4 Add schema migrations

### Milestone 1.2: BatchProcessor
- [ ] 1.2.1 Define BatchProcessor trait
- [ ] 1.2.2 Implement LocalBatchProcessor (Tokio)
- [ ] 1.2.3 Progress channel for UI updates
- [ ] 1.2.4 Cancellation support

### Milestone 1.3: GStreamer VideoDecoder
- [ ] 1.3.1 Implement VideoDecoder trait (VideoToolbox)
- [ ] 1.3.2 Frame extraction at timestamp
- [ ] 1.3.3 Audio extraction for Whisper
- [ ] 1.3.4 Metadata extraction

### Milestone 1.4: ModelManager
- [ ] 1.4.1 Model registry (list available)
- [ ] 1.4.2 Load/unload lifecycle (idle timeout)
- [ ] 1.4.3 Model download (HuggingFace)
- [ ] 1.4.4 Capability detection (GPU, memory)

---

## Phase 2: Indexing & Embeddings (12 days)

### Milestone 2.1: Key Frame Extraction
- [ ] 2.1.1 Scene change detection (histogram)
- [ ] 2.1.2 Uniform sampling fallback
- [ ] 2.1.3 Thumbnail generation (320px JPEG)
- [ ] 2.1.4 Batch extraction (memory <2GB)

### Milestone 2.2: CLIP Embedding
- [ ] 2.2.1 CLIP model loading (ViT-B/32 via ONNX)
- [ ] 2.2.2 Image preprocessing (224x224, normalize)
- [ ] 2.2.3 Batch embedding (GPU utilization >80%)
- [ ] 2.2.4 Text embedding for queries

### Milestone 2.3: Whisper Transcription
- [ ] 2.3.1 Whisper model loading (base.en)
- [ ] 2.3.2 Audio preprocessing (16kHz mono)
- [ ] 2.3.3 Segment extraction (word-level timestamps)
- [ ] 2.3.4 FTS5 integration

### Milestone 2.4: Vector Index (sqlite-vss)
- [ ] 2.4.1 Add sqlite-vss dependency
- [ ] 2.4.2 Create vss_keyframes virtual table (512 dim)
- [ ] 2.4.3 Insert embeddings on indexing
- [ ] 2.4.4 ANN query (<100ms for 500K embeddings)

### Milestone 2.5: Text Embeddings
- [ ] 2.5.1 Load text embedding model (MiniLM-L6)
- [ ] 2.5.2 Embed transcript segments
- [ ] 2.5.3 Semantic transcript search

---

## Phase 3: Agent System (14 days)

### Milestone 3.1: Agent Presets
- [ ] 3.1.1 Define preset schema (JSON)
- [ ] 3.1.2 Implement preset loader
- [ ] 3.1.3 Model bundle validation
- [ ] 3.1.4 Default presets (4 presets)

### Milestone 3.2: Tool Definitions
- [ ] 3.2.1 search_videos (<3s response)
- [ ] 3.2.2 get_frame (<100ms response)
- [ ] 3.2.3 analyze_frame (VLM, <5s)
- [ ] 3.2.4 extract_clips (FFmpeg batch)
- [ ] 3.2.5 generate_summary (markdown)
- [ ] 3.2.6 count_occurrences
- [ ] 3.2.7 get_video_info
- [ ] 3.2.8 list_indexed_videos

### Milestone 3.3: Tool Executor
- [ ] 3.3.1 Tool registry with schemas
- [ ] 3.3.2 Execution router (dispatch by name)
- [ ] 3.3.3 Error handling (clear messages)
- [ ] 3.3.4 Result caching (LRU)
- [ ] 3.3.5 Streaming output for long operations

### Milestone 3.4: Chat Interface
- [ ] 3.4.1 Chat history (persist per project)
- [ ] 3.4.2 Message streaming (incremental)
- [ ] 3.4.3 Tool call rendering (UI blocks)
- [ ] 3.4.4 Context management (prior turns)
- [ ] 3.4.5 egui chat widget (keyboard shortcuts)

### Milestone 3.5: Artifact Store
- [ ] 3.5.1 Define artifact types (clips, reports)
- [ ] 3.5.2 Storage backend (SQLite + filesystem)
- [ ] 3.5.3 Artifact viewer (preview in UI)
- [ ] 3.5.4 Export options (download)

---

## Phase 4: Jetson Thor Deployment (20 days)

### Milestone 4.1: Platform Abstraction
- [ ] 4.1.1 Remote provider traits
- [ ] 4.1.2 Deployment config (Mac vs Jetson)
- [ ] 4.1.3 Event bus authentication (HMAC)
- [ ] 4.1.4 Session management

### Milestone 4.2: DeepStream Pipeline
- [ ] 4.2.1 Multi-stream input (4-8 RTSP)
- [ ] 4.2.2 NvInfer integration (YOLOv8n)
- [ ] 4.2.3 NvDCF tracker
- [ ] 4.2.4 Metadata probe
- [ ] 4.2.5 DeepStream container (Docker)

### Milestone 4.3: Triton Inference Server
- [ ] 4.3.1 Model repository structure
- [ ] 4.3.2 YOLOv8 TensorRT (<10ms inference)
- [ ] 4.3.3 CLIP TensorRT (batch 32 <100ms)
- [ ] 4.3.4 Qwen2.5-VL TensorRT-LLM (30 t/s)
- [ ] 4.3.5 Triton Rust client (gRPC)

### Milestone 4.4: VLM Orchestration
- [ ] 4.4.1 Detection-triggered VLM
- [ ] 4.4.2 Query handler (natural language)
- [ ] 4.4.3 Context store (scene persistence)
- [ ] 4.4.4 Multi-camera aggregation

### Milestone 4.5: WebRTC Streaming
- [ ] 4.5.1 WebRTC server (GStreamer webrtcsink)
- [ ] 4.5.2 Detection overlay
- [ ] 4.5.3 HLS fallback (Safari compatibility)
- [ ] 4.5.4 Client receiver (web/egui)

---

## Dependencies

```
Phase 1 (Core Infrastructure)
         │
         ▼
Phase 2 (Indexing & Embeddings)
         │
         ▼
Phase 3 (Agent System)
         │
         ├───────────────────────┐
         ▼                       ▼
   Mac Release          Phase 4 (Jetson Thor)
```

---

## Success Criteria

| User Story | Phase | Validation |
|------------|-------|------------|
| Search videos by spoken words | 2, 3 | Transcript FTS returns results in <3s |
| Search by visual concepts | 2, 3 | CLIP ANN returns results in <100ms |
| Converse with agent | 3 | Chat interface works with presets |
| Extract specific clips | 3 | Tool creates valid MP4 files |
| Generate summary reports | 3 | Markdown with timestamps |
| Edge deployment | 4 | 4 streams at 30fps on Jetson Thor |

---

## Archived Roadmaps

Previous roadmap iterations:

```
roadmap/archive/content-analyst-traditional/
├── ARCHIVED.md           # Archive notice
├── phase-0-ux-discovery.md
├── phase-1-foundation.md
├── phase-2-video-indexing.md
├── phase-3-transcription.md
├── phase-4-inference.md
├── phase-5-search.md
├── phase-6-clip-extraction.md
├── phase-7-tools.md
└── phase-8-export.md

roadmap/archive/security-monitoring/
├── ARCHIVED.md           # Archive notice
├── README.md             # Jetson reference architecture
└── phase-*.md            # 8 original phases
```
