# Phase 1: Core Infrastructure

**Duration:** 10 days
**Goal:** Establish foundational services for batch video processing, storage, and model management.

---

## Overview

This phase builds the core infrastructure that all subsequent phases depend on: SQLite storage with FTS5, batch video processing, GStreamer decode pipeline, and model lifecycle management. These components enable both the indexing pipeline (Phase 2) and the agent system (Phase 3).

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        APPLICATION LAYER                                │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  egui UI (drag-drop, progress, chat interface)                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        SERVICE LAYER                                    │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ BatchProcessor   │  │ ModelManager     │  │ VideoDecoder     │     │
│  │                  │  │                  │  │                  │     │
│  │ • process_batch()│  │ • load_model()   │  │ • decode_file()  │     │
│  │ • progress()     │  │ • unload_model() │  │ • extract_frames │     │
│  │ • cancel()       │  │ • get_model()    │  │ • get_metadata() │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐                            │
│  │ IndexProvider    │  │ ConfigManager    │                            │
│  │                  │  │                  │                            │
│  │ • store()        │  │ • load_config()  │                            │
│  │ • query()        │  │ • save_config()  │                            │
│  │ • search_fts()   │  │ • validate()     │                            │
│  └──────────────────┘  └──────────────────┘                            │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        STORAGE LAYER                                    │
│                                                                         │
│  SQLite + FTS5 (yama.db)    │    Model Cache (~/.cache/yama/models/)   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone 1.1: SQLite Schema & IndexProvider

**Duration:** 3 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 1.1.1 | Design schema | Videos, keyframes, transcripts, detections | Schema supports all record types |
| 1.1.2 | Create FTS5 tables | Full-text search on transcripts | `MATCH` queries return results in <100ms |
| 1.1.3 | Implement IndexProvider | CRUD operations | `cargo test index_provider` passes |
| 1.1.4 | Add migrations | Schema versioning | Migrations run idempotently |

### Code: Schema

```sql
-- platform/apple/host/src/providers/schema.sql

CREATE TABLE IF NOT EXISTS videos (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    fps REAL NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    file_hash TEXT NOT NULL,
    indexed_at TEXT NOT NULL,
    has_transcript INTEGER NOT NULL DEFAULT 0,
    keyframe_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS keyframes (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    timestamp_ms INTEGER NOT NULL,
    frame_number INTEGER NOT NULL,
    thumbnail_jpeg BLOB,
    embedding BLOB,
    UNIQUE(video_id, frame_number)
);

CREATE TABLE IF NOT EXISTS transcripts (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    confidence REAL NOT NULL,
    speaker TEXT
);

CREATE TABLE IF NOT EXISTS detections (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    keyframe_id INTEGER REFERENCES keyframes(id),
    timestamp_ms INTEGER NOT NULL,
    class_name TEXT NOT NULL,
    confidence REAL NOT NULL,
    bbox_x REAL NOT NULL,
    bbox_y REAL NOT NULL,
    bbox_width REAL NOT NULL,
    bbox_height REAL NOT NULL
);

-- Full-text search index
CREATE VIRTUAL TABLE IF NOT EXISTS transcripts_fts USING fts5(
    video_id,
    start_ms,
    text,
    content='transcripts',
    content_rowid='id'
);

-- Triggers for FTS sync
CREATE TRIGGER IF NOT EXISTS transcripts_ai AFTER INSERT ON transcripts BEGIN
    INSERT INTO transcripts_fts(rowid, video_id, start_ms, text)
    VALUES (new.id, new.video_id, new.start_ms, new.text);
END;

CREATE TRIGGER IF NOT EXISTS transcripts_ad AFTER DELETE ON transcripts BEGIN
    INSERT INTO transcripts_fts(transcripts_fts, rowid, video_id, start_ms, text)
    VALUES ('delete', old.id, old.video_id, old.start_ms, old.text);
END;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_keyframes_video ON keyframes(video_id);
CREATE INDEX IF NOT EXISTS idx_transcripts_video ON transcripts(video_id);
CREATE INDEX IF NOT EXISTS idx_detections_video ON detections(video_id);
CREATE INDEX IF NOT EXISTS idx_detections_class ON detections(class_name);
```

### Verification

```bash
cargo test -p yama-host-apple test_index_provider
# AC-1: All CRUD tests pass
# AC-2: FTS5 MATCH query returns in <100ms
```

---

## Milestone 1.2: BatchProcessor Trait & Implementation

**Duration:** 2 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 1.2.1 | Define BatchProcessor trait | Job queue abstraction | Compiles with async methods |
| 1.2.2 | Implement LocalBatchProcessor | Tokio-based parallelism | Processes 10 videos concurrently |
| 1.2.3 | Progress channel | Real-time progress events | UI receives progress updates |
| 1.2.4 | Cancellation support | Graceful job cancellation | Cancel completes in <5s |

### Code: BatchProcessor

```rust
// shared/platform-traits/src/batch.rs

#[async_trait]
pub trait BatchProcessor: Send + Sync {
    async fn process_batch(
        &self,
        videos: Vec<PathBuf>,
        config: BatchConfig,
    ) -> Result<BatchHandle>;

    async fn progress(&self, handle: &BatchHandle) -> Result<BatchProgress>;
    async fn cancel(&self, handle: &BatchHandle) -> Result<()>;
    async fn wait(&self, handle: &BatchHandle) -> Result<BatchResult>;
}

pub struct BatchConfig {
    pub extract_keyframes: bool,
    pub transcribe: bool,
    pub generate_embeddings: bool,
    pub skip_indexed: bool,
    pub parallelism: u32,
}

pub struct BatchProgress {
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
    pub current_file: Option<String>,
    pub stage: BatchStage,
    pub eta_seconds: Option<u64>,
}

pub enum BatchStage {
    Scanning,
    Transcribing,
    ExtractingFrames,
    GeneratingEmbeddings,
    Indexing,
    Complete,
}
```

### Verification

```bash
cargo test -p yama-host-apple test_batch_processor
# AC-1: Batch of 10 test videos completes
# AC-2: Progress events received for each stage
# AC-3: Cancel stops processing within 5s
```

---

## Milestone 1.3: GStreamer VideoDecoder

**Duration:** 3 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 1.3.1 | Implement VideoDecoder trait | GStreamer + VideoToolbox | Hardware decode works |
| 1.3.2 | Frame extraction | Extract frames at timestamp | Frames extracted at correct time |
| 1.3.3 | Audio extraction | Extract audio for Whisper | WAV output for transcription |
| 1.3.4 | Metadata extraction | Duration, resolution, codec | All metadata fields populated |

### Code: VideoDecoder

```rust
// shared/platform-traits/src/decoder.rs

#[async_trait]
pub trait VideoDecoder: Send + Sync {
    async fn get_metadata(&self, path: &Path) -> Result<VideoMetadata>;

    async fn extract_keyframes(
        &self,
        path: &Path,
        interval_seconds: f64,
        max_frames: Option<u32>,
    ) -> Result<Vec<KeyFrame>>;

    async fn decode_frame(&self, path: &Path, timestamp_ms: u64) -> Result<Frame>;
    async fn extract_audio(&self, path: &Path) -> Result<AudioData>;
}

pub struct VideoMetadata {
    pub path: PathBuf,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub codec: String,
    pub file_size_bytes: u64,
    pub has_audio: bool,
}
```

### GStreamer Pipeline

```
appsrc name=src ! vtdec_hw ! videoconvert ! video/x-raw,format=NV12 ! appsink name=sink
```

### Verification

```bash
cargo run --example extract_frames -- ~/Videos/test.mp4
# AC-1: Extracts frames at 5s intervals
# AC-2: Frame timestamps match requested times (±100ms)
# AC-3: Audio extracted as 16kHz mono WAV
```

---

## Milestone 1.4: ModelManager

**Duration:** 2 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 1.4.1 | Model registry | Track available models | Lists models in ~/.cache/yama/models/ |
| 1.4.2 | Load/unload lifecycle | Memory management | Models unload after idle timeout |
| 1.4.3 | Model download | Fetch from HuggingFace | Progress shown during download |
| 1.4.4 | Capability detection | Check GPU, memory | Reports available backends |

### Code: ModelManager

```rust
// platform/apple/host/src/model_manager.rs

pub struct ModelManager {
    models_dir: PathBuf,
    loaded: HashMap<String, LoadedModel>,
    idle_timeout: Duration,
}

impl ModelManager {
    pub fn new(models_dir: PathBuf) -> Self;
    pub fn list_available(&self) -> Vec<ModelInfo>;
    pub fn is_loaded(&self, name: &str) -> bool;
    pub async fn load(&mut self, name: &str) -> Result<&LoadedModel>;
    pub async fn unload(&mut self, name: &str) -> Result<()>;
    pub fn memory_usage(&self) -> MemoryUsage;
    pub async fn download(&self, model_id: &str, progress: impl Fn(u64, u64)) -> Result<PathBuf>;
}

pub struct ModelInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub model_type: ModelType,
    pub loaded: bool,
}

pub enum ModelType {
    Vlm,
    Whisper,
    Clip,
    TextEmbedding,
    Detection,
}
```

### Verification

```bash
cargo run -p yama-host-apple -- --list-models
# AC-1: Lists all models in cache directory
# AC-2: Shows loaded status and memory usage
```

---

## Dependencies

This phase has no dependencies on other phases.

## Blocks

- Phase 2 (Indexing & Embeddings) - needs IndexProvider, VideoDecoder
- Phase 3 (Agent System) - needs ModelManager, BatchProcessor

---

## Checklist

### Milestone 1.1: SQLite Schema & IndexProvider
- [ ] 1.1.1 Design schema
- [ ] 1.1.2 Create FTS5 tables
- [ ] 1.1.3 Implement IndexProvider
- [ ] 1.1.4 Add migrations

### Milestone 1.2: BatchProcessor
- [ ] 1.2.1 Define BatchProcessor trait
- [ ] 1.2.2 Implement LocalBatchProcessor
- [ ] 1.2.3 Progress channel
- [ ] 1.2.4 Cancellation support

### Milestone 1.3: GStreamer VideoDecoder
- [ ] 1.3.1 Implement VideoDecoder trait
- [ ] 1.3.2 Frame extraction
- [ ] 1.3.3 Audio extraction
- [ ] 1.3.4 Metadata extraction

### Milestone 1.4: ModelManager
- [ ] 1.4.1 Model registry
- [ ] 1.4.2 Load/unload lifecycle
- [ ] 1.4.3 Model download
- [ ] 1.4.4 Capability detection
