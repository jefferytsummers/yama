# Implementation Checklist

## Content Analyst Roadmap

**Total Duration:** 66-70 days
**Target Persona:** Content Analyst (video researchers, footage reviewers)

---

## Phase Overview

| Phase | Name | Days | Status |
|-------|------|------|--------|
| 0 | UX Discovery | 5-7 | ⬜ Not Started |
| 0.5 | GStreamer Spike | 2 | ⬜ Not Started |
| 1 | Foundation | 8 | ⬜ Not Started |
| 2 | Video Indexing | 12 | ⬜ Not Started |
| 3 | Transcription | 6 | ⬜ Not Started |
| 4 | Inference | 9 | ⬜ Not Started |
| 5 | Search & Retrieval | 10 | ⬜ Not Started |
| 6 | Clip Extraction | 6 | ⬜ Not Started |
| 7 | Tool System | 8 | ⬜ Not Started |
| 8 | Export & Reporting | 5 | ⬜ Not Started |

---

## Phase 0: UX Discovery (5-7 days)

### Milestone 0.1: Workflow Mapping
- [ ] 0.1.1 Import flow diagram
- [ ] 0.1.2 Indexing flow diagram
- [ ] 0.1.3 Search flow diagram
- [ ] 0.1.4 Review flow diagram
- [ ] 0.1.5 Extract flow diagram
- [ ] 0.1.6 Report flow diagram

### Milestone 0.2: Wireframe Exploration
- [ ] 0.2.1 Library view wireframe
- [ ] 0.2.2 Import overlay wireframe
- [ ] 0.2.3 Search results wireframe
- [ ] 0.2.4 Preview panel wireframe
- [ ] 0.2.5 Clip extraction wireframe
- [ ] 0.2.6 Model settings wireframe
- [ ] 0.2.7 First-run wireframe

### Milestone 0.3: Component Inventory
- [ ] 0.3.1 Audit existing components
- [ ] 0.3.2 Document search results component
- [ ] 0.3.3 Document multi-stage progress
- [ ] 0.3.4 Document model download UI
- [ ] 0.3.5 Document file drop zone

### Milestone 0.4: UX Validation
- [ ] 0.4.1 Walk through user stories
- [ ] 0.4.2 Validate import flow
- [ ] 0.4.3 Validate search flow
- [ ] 0.4.4 Validate export flow
- [ ] 0.4.5 Document decisions

### Phase 0.5: GStreamer Spike (parallel)
- [ ] 0.5.1 Benchmark single video decode
- [ ] 0.5.2 Benchmark batch decode (10 videos)
- [ ] 0.5.3 Profile with Instruments
- [ ] 0.5.4 Document go/no-go decision

---

## Phase 1: Foundation (8 days)

### Milestone 1.1: Core Traits
- [ ] 1.1.1 VideoDecoder trait
- [ ] 1.1.2 IndexProvider trait
- [ ] 1.1.3 BatchProcessor trait
- [ ] 1.1.4 EmbeddingModel trait
- [ ] 1.1.5 VlmProvider trait

### Milestone 1.2: GStreamer Decoder
- [ ] 1.2.1 VideoToolbox integration
- [ ] 1.2.2 Frame extraction pipeline
- [ ] 1.2.3 Seek to timestamp
- [ ] 1.2.4 Metadata extraction

### Milestone 1.3: SQLite Index
- [ ] 1.3.1 Database schema (videos, keyframes, detections, transcripts)
- [ ] 1.3.2 FTS5 triggers
- [ ] 1.3.3 Video CRUD operations
- [ ] 1.3.4 Keyframe storage
- [ ] 1.3.5 Detection storage
- [ ] 1.3.6 Embedding queries

### Milestone 1.4: Batch Processor
- [ ] 1.4.1 Directory watcher
- [ ] 1.4.2 Job queue
- [ ] 1.4.3 Progress tracking
- [ ] 1.4.4 Resume capability

---

## Phase 2: Video Indexing (12 days)

### Milestone 2.1: Key Frame Extraction
- [ ] 2.1.1 Scene change detection
- [ ] 2.1.2 Frame quality scoring
- [ ] 2.1.3 Thumbnail generation
- [ ] 2.1.4 Frame deduplication

### Milestone 2.2: CLIP Embedding
- [ ] 2.2.1 Model loading (ViT-B/32)
- [ ] 2.2.2 Image preprocessing
- [ ] 2.2.3 Batch embedding
- [ ] 2.2.4 Embedding storage

### Milestone 2.3: Batch Indexer
- [ ] 2.3.1 Video discovery
- [ ] 2.3.2 Incremental indexing
- [ ] 2.3.3 Progress reporting
- [ ] 2.3.4 Error recovery

### Milestone 2.4: Vector Index (sqlite-vss)
- [ ] 2.4.1 Add sqlite-vss dependency
- [ ] 2.4.2 Create vss_keyframes virtual table
- [ ] 2.4.3 Insert embeddings on indexing
- [ ] 2.4.4 ANN query function (<100ms for 500K embeddings)

---

## Phase 3: Transcription (6 days)

### Milestone 3.1: Audio Extraction
- [ ] 3.1.1 GStreamer audio pipeline
- [ ] 3.1.2 Handle no-audio videos
- [ ] 3.1.3 Chunked extraction

### Milestone 3.2: Whisper Integration
- [ ] 3.2.1 whisper.cpp bindings
- [ ] 3.2.2 Word-level timestamps
- [ ] 3.2.3 Model selection

### Milestone 3.3: FTS5 Integration
- [ ] 3.3.1 Store transcripts
- [ ] 3.3.2 FTS5 search
- [ ] 3.3.3 Snippet generation
- [ ] 3.3.4 Incremental transcription

---

## Phase 4: Inference (9 days)

### Milestone 4.1: GGUF Model Loading
- [ ] 4.1.1 llama.cpp integration
- [ ] 4.1.2 Metal GPU backend
- [ ] 4.1.3 Model warmup
- [ ] 4.1.4 Memory management

### Milestone 4.2: Image Preprocessing
- [ ] 4.2.1 CLIP vision encoder
- [ ] 4.2.2 Image resizing
- [ ] 4.2.3 Normalization

### Milestone 4.3: Inference Pipeline
- [ ] 4.3.1 Batch inference
- [ ] 4.3.2 Context window
- [ ] 4.3.3 Streaming output
- [ ] 4.3.4 Memory budget

### Milestone 4.4: Model Management UI
- [ ] 4.4.1 Model registry configuration
- [ ] 4.4.2 Download manager with progress
- [ ] 4.4.3 Storage manager (show sizes, delete)
- [ ] 4.4.4 First-run wizard

---

## Phase 5: Search & Retrieval (10 days)

### Milestone 5.1: Query Parser
- [ ] 5.1.1 Visual concept detection
- [ ] 5.1.2 Temporal phrase extraction
- [ ] 5.1.3 Multi-modal routing

### Milestone 5.2: Semantic Search (sqlite-vss ANN)
- [ ] 5.2.1 CLIP text encoding
- [ ] 5.2.2 sqlite-vss ANN query
- [ ] 5.2.3 Top-K retrieval (<100ms for 500K vectors)
- [ ] 5.2.4 Result threshold filtering

### Milestone 5.3: Full-Text Search
- [ ] 5.3.1 FTS5 query syntax
- [ ] 5.3.2 Phrase search
- [ ] 5.3.3 Boolean operators

### Milestone 5.4: Result Ranking
- [ ] 5.4.1 Score fusion
- [ ] 5.4.2 Temporal clustering
- [ ] 5.4.3 Deduplication
- [ ] 5.4.4 Confidence filtering

---

## Phase 6: Clip Extraction (6 days)

### Milestone 6.1: FFmpeg Wrapper
- [ ] 6.1.1 FFmpeg subprocess
- [ ] 6.1.2 Frame-accurate seeking
- [ ] 6.1.3 Error handling

### Milestone 6.2: Clip Planning
- [ ] 6.2.1 Segment padding
- [ ] 6.2.2 Adjacent merging
- [ ] 6.2.3 Duration limits

### Milestone 6.3: Batch Export
- [ ] 6.3.1 Parallel extraction
- [ ] 6.3.2 Progress tracking
- [ ] 6.3.3 Manifest generation

---

## Phase 7: Tool System (8 days)

### Milestone 7.1: Tool Definitions
- [ ] 7.1.1 search_videos
- [ ] 7.1.2 get_frame
- [ ] 7.1.3 analyze_frame
- [ ] 7.1.4 count_occurrences
- [ ] 7.1.5 extract_clips
- [ ] 7.1.6 generate_summary
- [ ] 7.1.7 get_video_info
- [ ] 7.1.8 list_indexed_videos

### Milestone 7.2: Tool Executor
- [ ] 7.2.1 Schema validation
- [ ] 7.2.2 Execution dispatch
- [ ] 7.2.3 Result formatting
- [ ] 7.2.4 Error handling

### Milestone 7.3: Result Caching
- [ ] 7.3.1 LRU cache
- [ ] 7.3.2 Cache invalidation
- [ ] 7.3.3 Memory limits

---

## Phase 8: Export & Reporting (5 days)

### Milestone 8.1: Export Traits
- [ ] 8.1.1 ExportFormat enum
- [ ] 8.1.2 Exporter trait
- [ ] 8.1.3 ExportConfig

### Milestone 8.2: Markdown Report Generator
- [ ] 8.2.1 Template system
- [ ] 8.2.2 Timestamp formatting
- [ ] 8.2.3 Table generation
- [ ] 8.2.4 Video links

### Milestone 8.3: JSON/CSV Exporters
- [ ] 8.3.1 JSON serialization
- [ ] 8.3.2 CSV writer
- [ ] 8.3.3 Field mapping

### Milestone 8.4: Export Pipeline
- [ ] 8.4.1 Pipeline orchestration
- [ ] 8.4.2 Progress reporting
- [ ] 8.4.3 Batch export
- [ ] 8.4.4 Error recovery

---

## Dependencies

```
Phase 0 (UX Discovery) ──┬── Phase 0.5 (GStreamer Spike, parallel)
                         │
                         ▼
                    Phase 1
                   (Foundation)
                         │
    ┌────────────────────┼────────────────────┐
    ▼                    ▼                    ▼
Phase 2              Phase 3              Phase 4
(Indexing+vss)       (Transcription)      (Inference+Mgmt)
    │                    │                    │
    └────────────┬───────┴────────────────────┘
                 ▼
            Phase 5
         (Search+ANN)
                 │
    ┌────────────┴────────────┐
    ▼                         ▼
Phase 6                   Phase 7
(Extraction)              (Tools)
    │                         │
    └────────────┬────────────┘
                 ▼
            Phase 8
            (Export)
```

---

## Success Criteria

From the Content Analyst persona:

| User Story | Phase | Validation |
|------------|-------|------------|
| Search videos by spoken words | 3, 5 | Transcript FTS returns results |
| Search by visual concepts | 2, 5 | CLIP similarity returns results |
| Extract specific clips | 6 | FFmpeg exports correct segments |
| Count occurrences | 7 | Tool returns accurate counts |
| Generate summary reports | 7, 8 | Markdown report with timestamps |
| Catalog large libraries | 2 | Batch indexer handles 1000+ videos |

---

## Archived Roadmap

The original real-time security monitoring roadmap has been archived:

```
roadmap/archive/security-monitoring/
├── ARCHIVED.md      # Archival metadata
├── README.md        # Original overview
├── SUMMARY.md       # Original checklist
└── phase-*.md       # Original 8 phases
```

Patterns from the archived roadmap (event bus, trait abstractions) can be referenced for future real-time features.
