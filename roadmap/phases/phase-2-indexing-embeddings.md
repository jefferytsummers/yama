# Phase 2: Indexing & Embeddings

**Duration:** 12 days
**Goal:** Build the indexing pipeline with CLIP visual embeddings, Whisper transcription, sqlite-vss vector search, and text embeddings.

---

## Overview

This phase implements the core indexing pipeline that processes video files into a searchable index. Key frames are embedded with CLIP, audio is transcribed with Whisper, and all data is stored in SQLite with vector similarity search via sqlite-vss. This enables the agent system (Phase 3) to perform semantic search across video archives.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  VIDEO FILE INPUT                                                       │
│  ~/Videos/meetings/*.mp4                                                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  INDEXING PIPELINE                                                      │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ 1. Scan Files    │→ │ 2. Extract Meta  │→ │ 3. Key Frames    │     │
│  │ (skip indexed)   │  │ (duration, fps)  │  │ (1 per 5 sec)    │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
│                                                    │                    │
│         ┌──────────────────────────────────────────┼──────────────┐    │
│         ▼                                          ▼              ▼    │
│  ┌──────────────────┐              ┌──────────────────┐  ┌──────────┐ │
│  │ 4a. Whisper      │              │ 4b. CLIP Embed   │  │ Thumbnail│ │
│  │ (transcribe)     │              │ (batch GPU)      │  │ (JPEG)   │ │
│  └──────────────────┘              └──────────────────┘  └──────────┘ │
│         │                                   │                  │       │
│         └───────────────────────────────────┴──────────────────┘       │
│                                             │                          │
│                                             ▼                          │
│         ┌─────────────────────────────────────────────────────────┐   │
│         │ 5. Store in SQLite + sqlite-vss                         │   │
│         │ • videos, keyframes, transcripts tables                 │   │
│         │ • vss_keyframes (vector similarity)                     │   │
│         │ • transcripts_fts (full-text search)                    │   │
│         └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone 2.1: Key Frame Extraction

**Duration:** 2 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 2.1.1 | Scene change detection | Histogram-based change detection | Reduces redundant frames by 30%+ |
| 2.1.2 | Uniform sampling fallback | Extract every N seconds | Consistent coverage for static videos |
| 2.1.3 | Thumbnail generation | 320px JPEG at quality 80 | Thumbnails under 50KB average |
| 2.1.4 | Batch extraction | Process multiple videos | Memory stays <2GB during extraction |

### Code: KeyFrameExtractor

```rust
// platform/apple/host/src/indexer/keyframe.rs

pub struct KeyFrameConfig {
    pub interval_seconds: f64,         // 5.0 default
    pub scene_change_threshold: f64,   // 0.3 default
    pub max_frames: u32,               // 500 default
    pub thumbnail_size: u32,           // 320 default
    pub jpeg_quality: u8,              // 80 default
}

pub struct ExtractedKeyFrame {
    pub timestamp_ms: u64,
    pub frame_number: u64,
    pub thumbnail_jpeg: Vec<u8>,
    pub frame_data: Frame,
}
```

### Verification

```bash
cargo run --example extract_keyframes -- ~/Videos/test.mp4
# AC-1: Extracts frames from 120s video
# AC-2: Scene detection reduces frame count from uniform sampling
# AC-3: Thumbnails average <50KB
```

---

## Milestone 2.2: CLIP Embedding Integration

**Duration:** 3 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 2.2.1 | CLIP model loading | Load ViT-B/32 via ONNX Runtime | Model loads in <5s |
| 2.2.2 | Image preprocessing | Resize 224x224, normalize | Correct tensor shape [1, 3, 224, 224] |
| 2.2.3 | Batch embedding | Process multiple images | GPU utilization >80% during batch |
| 2.2.4 | Text embedding | Embed search queries | Text→Image similarity works |

### Model Options

| Model | Dimension | Memory | Speed | Quality |
|-------|-----------|--------|-------|---------|
| ViT-B/32 | 512 | 350MB | Fast | Good |
| ViT-L/14 | 768 | 890MB | Slow | Better |

**Default:** ViT-B/32 for balance of speed and quality.

### Code: ClipModel

```rust
// platform/apple/host/src/indexer/clip.rs

pub struct ClipModel {
    session: ort::Session,
    dimension: usize,
}

impl ClipModel {
    pub fn load(model_path: &Path) -> Result<Self>;
    pub fn embed_image(&self, image: &ImageData) -> Result<Vec<f32>>;
    pub fn embed_images_batch(&self, images: &[ImageData]) -> Result<Vec<Vec<f32>>>;
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>>;
}

// CLIP normalization constants
const CLIP_MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
const CLIP_STD: [f32; 3] = [0.26862954, 0.26130258, 0.27577711];
```

### Verification

```bash
cargo run --example clip_similarity -- "a person presenting slides"
# AC-1: Returns top-5 similar frames from indexed videos
# AC-2: Similarity scores in range [0, 1]
# AC-3: Batch of 32 images embeds in <500ms (M1)
```

---

## Milestone 2.3: Whisper Transcription

**Duration:** 3 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 2.3.1 | Whisper model loading | Load via whisper.cpp | base.en loads in <3s |
| 2.3.2 | Audio preprocessing | 16kHz mono WAV | Correct format for Whisper |
| 2.3.3 | Segment extraction | Word-level timestamps | Segments align with speech |
| 2.3.4 | FTS5 integration | Store in searchable index | Full-text queries work |

### Model Options (from model-strategy.md)

| Model | Memory | Speed Factor | WER (English) |
|-------|--------|--------------|---------------|
| tiny.en | 200MB | 32x | 9.9% |
| base.en | 400MB | 16x | 6.7% |
| small.en | 1GB | 6x | 5.0% |

**Default:** base.en for balance of speed and accuracy.

### Code: WhisperModel

```rust
// platform/apple/host/src/indexer/whisper.rs

pub struct WhisperModel {
    ctx: whisper_rs::WhisperContext,
}

impl WhisperModel {
    pub fn load(model_path: &Path) -> Result<Self>;
    pub fn transcribe(&self, audio: &AudioData) -> Result<Vec<TranscriptSegment>>;
}

pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub confidence: f32,
}
```

### Verification

```bash
cargo run --example transcribe -- ~/Videos/meeting.mp4
# AC-1: Transcription completes at >10x realtime
# AC-2: Segments have word-level timestamps
# AC-3: FTS5 query "budget meeting" returns correct video
```

---

## Milestone 2.4: Vector Index (sqlite-vss)

**Duration:** 3 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 2.4.1 | Add sqlite-vss | Load extension at runtime | Extension loads on macOS |
| 2.4.2 | Create vss_keyframes table | Virtual table for embeddings | Table created with 512 dimensions |
| 2.4.3 | Insert embeddings | Populate during indexing | Embeddings indexed correctly |
| 2.4.4 | ANN query | Approximate nearest neighbor search | <100ms for 500K embeddings |

### Why sqlite-vss?

Naive linear scan over 500K embeddings takes seconds. sqlite-vss provides O(log n) approximate nearest neighbor search.

### Code: sqlite-vss Integration

```sql
-- Create virtual table
CREATE VIRTUAL TABLE IF NOT EXISTS vss_keyframes USING vss0(
    embedding(512)  -- CLIP ViT-B/32 dimension
);

-- Insert embedding
INSERT INTO vss_keyframes(rowid, embedding) VALUES (?1, ?2);

-- Query by similarity
SELECT k.*, vss_distance_l2(v.embedding, ?1) as distance
FROM vss_keyframes v
JOIN keyframes k ON k.id = v.rowid
ORDER BY distance
LIMIT 100;
```

### Fallback

If sqlite-vss is problematic, use `instant-distance` crate (HNSW in pure Rust):

```toml
[dependencies]
instant-distance = "0.6"
```

### Verification

```bash
cargo run --example vss_benchmark -- --count 500000
# AC-1: Index 500K embeddings in <60s
# AC-2: Query time <100ms (p99)
# AC-3: Recall@100 > 90%
```

---

## Milestone 2.5: Text Embeddings

**Duration:** 1 day

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 2.5.1 | Load text embedding model | all-MiniLM-L6-v2 | 90MB model loads |
| 2.5.2 | Embed transcripts | Generate embeddings for segments | Embeddings stored |
| 2.5.3 | Semantic transcript search | Find by meaning, not just keywords | Synonyms work |

### Model

**all-MiniLM-L6-v2** - 384 dimensions, 90MB, good for transcript search.

### Verification

```bash
cargo run --example text_search -- "financial projections"
# AC-1: Returns segments mentioning "budget", "forecast", etc.
# AC-2: Query time <50ms for 100K segments
```

---

## Dependencies

- Phase 1 (Core Infrastructure) - IndexProvider, VideoDecoder, ModelManager

## Blocks

- Phase 3 (Agent System) - needs search capabilities
- Phase 4 (Jetson Deployment) - needs indexing pipeline

---

## Checklist

### Milestone 2.1: Key Frame Extraction
- [ ] 2.1.1 Scene change detection
- [ ] 2.1.2 Uniform sampling fallback
- [ ] 2.1.3 Thumbnail generation
- [ ] 2.1.4 Batch extraction

### Milestone 2.2: CLIP Embedding
- [ ] 2.2.1 CLIP model loading
- [ ] 2.2.2 Image preprocessing
- [ ] 2.2.3 Batch embedding
- [ ] 2.2.4 Text embedding

### Milestone 2.3: Whisper Transcription
- [ ] 2.3.1 Whisper model loading
- [ ] 2.3.2 Audio preprocessing
- [ ] 2.3.3 Segment extraction
- [ ] 2.3.4 FTS5 integration

### Milestone 2.4: Vector Index (sqlite-vss)
- [ ] 2.4.1 Add sqlite-vss dependency
- [ ] 2.4.2 Create vss_keyframes virtual table
- [ ] 2.4.3 Insert embeddings on indexing
- [ ] 2.4.4 ANN query (<100ms for 500K embeddings)

### Milestone 2.5: Text Embeddings
- [ ] 2.5.1 Load text embedding model
- [ ] 2.5.2 Embed transcripts
- [ ] 2.5.3 Semantic transcript search
