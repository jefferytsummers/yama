# Phase 1: Foundation

**Duration:** 8 days
**Goal:** Establish core traits for batch video processing and indexing.

---

## Overview

This phase creates the foundational abstractions for batch video analysis. Unlike real-time streaming, the Content Analyst workflow processes local files in batches, indexes results to SQLite, and serves queries from the index.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        APPLICATION LAYER                                │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  egui UI (drag-drop, progress, query results)                   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        ABSTRACTION LAYER (shared/platform-traits/)      │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ VideoDecoder     │  │ IndexProvider    │  │ BatchProcessor   │     │
│  │                  │  │                  │  │                  │     │
│  │ • decode_file()  │  │ • store()        │  │ • process_batch()│     │
│  │ • extract_frames │  │ • query()        │  │ • progress()     │     │
│  │ • get_metadata() │  │ • search_fts()   │  │ • cancel()       │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ EmbeddingModel   │  │ TranscriptModel  │  │ VlmProvider      │     │
│  │                  │  │                  │  │                  │     │
│  │ • embed_image()  │  │ • transcribe()   │  │ • analyze()      │     │
│  │ • embed_text()   │  │ • align()        │  │ • query()        │     │
│  │ • similarity()   │  │                  │  │                  │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        IMPLEMENTATION (platform/apple/)                 │
│                                                                         │
│  • GStreamer + VideoToolbox decoder                                     │
│  • SQLite + FTS5 index                                                  │
│  • CLIP via CoreML or ONNX                                              │
│  • Whisper via whisper.cpp                                              │
│  • VLM via llama.cpp (GGUF) or MLX                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone 1.1: Core Trait Definitions

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 1.1.1 | Define `VideoDecoder` trait | Local file decode | Compiles with `decode_file()`, `extract_keyframes()`, `get_metadata()` |
| 1.1.2 | Define `IndexProvider` trait | SQLite abstraction | Compiles with `store()`, `query()`, `search_fts()` |
| 1.1.3 | Define `BatchProcessor` trait | Job queue abstraction | Compiles with `process_batch()`, `progress()`, `cancel()` |
| 1.1.4 | Define `EmbeddingModel` trait | Vector embedding | Compiles with `embed_image()`, `embed_text()` |
| 1.1.5 | Define `VlmProvider` trait | Local VLM inference | Compiles with `analyze()`, `query()` |

### Code: VideoDecoder Trait

```rust
// shared/platform-traits/src/decoder.rs

/// Local video file decoder.
#[async_trait]
pub trait VideoDecoder: Send + Sync {
    /// Decode a video file and return metadata.
    async fn get_metadata(&self, path: &Path) -> Result<VideoMetadata>;

    /// Extract key frames at specified interval.
    async fn extract_keyframes(
        &self,
        path: &Path,
        interval_seconds: f64,
        max_frames: Option<u32>,
    ) -> Result<Vec<KeyFrame>>;

    /// Decode a specific timestamp to frame.
    async fn decode_frame(&self, path: &Path, timestamp_ms: u64) -> Result<Frame>;

    /// Extract audio track for transcription.
    async fn extract_audio(&self, path: &Path) -> Result<AudioData>;
}

/// Video file metadata.
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

/// Extracted key frame.
pub struct KeyFrame {
    pub timestamp_ms: u64,
    pub frame_number: u64,
    pub data: FrameData,
}
```

### Code: IndexProvider Trait

```rust
// shared/platform-traits/src/index.rs

/// Index storage provider (SQLite backend).
#[async_trait]
pub trait IndexProvider: Send + Sync {
    /// Store video metadata.
    async fn store_video(&self, video: &VideoRecord) -> Result<i64>;

    /// Store key frame with embedding.
    async fn store_keyframe(&self, keyframe: &KeyFrameRecord) -> Result<i64>;

    /// Store detection result.
    async fn store_detection(&self, detection: &DetectionRecord) -> Result<i64>;

    /// Store transcript segment.
    async fn store_transcript(&self, segment: &TranscriptRecord) -> Result<i64>;

    /// Full-text search across transcripts.
    async fn search_fts(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>>;

    /// Semantic search by embedding similarity.
    async fn search_semantic(
        &self,
        embedding: &[f32],
        limit: u32,
        threshold: f32,
    ) -> Result<Vec<SearchResult>>;

    /// Query videos by filter criteria.
    async fn query_videos(&self, filter: &VideoFilter) -> Result<Vec<VideoRecord>>;

    /// Check if video is already indexed.
    async fn is_indexed(&self, path: &Path) -> Result<bool>;

    /// Get index statistics.
    async fn stats(&self) -> Result<IndexStats>;
}

pub struct IndexStats {
    pub video_count: u64,
    pub keyframe_count: u64,
    pub detection_count: u64,
    pub transcript_segment_count: u64,
    pub index_size_bytes: u64,
}
```

### Code: BatchProcessor Trait

```rust
// shared/platform-traits/src/batch.rs

/// Batch job processor with progress tracking.
#[async_trait]
pub trait BatchProcessor: Send + Sync {
    /// Process a batch of videos.
    async fn process_batch(
        &self,
        videos: Vec<PathBuf>,
        config: BatchConfig,
    ) -> Result<BatchHandle>;

    /// Get progress for a batch job.
    async fn progress(&self, handle: &BatchHandle) -> Result<BatchProgress>;

    /// Cancel a running batch job.
    async fn cancel(&self, handle: &BatchHandle) -> Result<()>;

    /// Wait for batch completion.
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

pub struct BatchResult {
    pub processed: u32,
    pub skipped: u32,
    pub failed: Vec<(PathBuf, String)>,
    pub duration_seconds: u64,
}
```

### Code: EmbeddingModel Trait

```rust
// shared/platform-traits/src/embedding.rs

/// Vector embedding model (CLIP, etc).
#[async_trait]
pub trait EmbeddingModel: Send + Sync {
    /// Embed an image to vector.
    async fn embed_image(&self, image: &ImageData) -> Result<Embedding>;

    /// Embed text to vector.
    async fn embed_text(&self, text: &str) -> Result<Embedding>;

    /// Batch embed images.
    async fn embed_images_batch(&self, images: &[ImageData]) -> Result<Vec<Embedding>>;

    /// Compute cosine similarity between embeddings.
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32;

    /// Get embedding dimension.
    fn dimension(&self) -> usize;
}

pub struct Embedding {
    pub vector: Vec<f32>,
    pub model: String,
}
```

### Verification

```bash
cargo build -p yama-platform-traits
cargo test -p yama-platform-traits
cargo clippy -p yama-platform-traits -- -D warnings
```

### Deliverable

`shared/platform-traits/src/`

---

## Milestone 1.2: Request/Response Types

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 1.2.1 | Define `VideoRecord` | Database video entry | Serializes to/from SQLite |
| 1.2.2 | Define `KeyFrameRecord` | Key frame with embedding | Includes timestamp, thumbnail |
| 1.2.3 | Define `DetectionRecord` | Object detection result | Includes bbox, class, confidence |
| 1.2.4 | Define `TranscriptRecord` | Transcript segment | Includes start/end time, text |
| 1.2.5 | Define `SearchResult` | Unified search result | Works for FTS and semantic |

### Code: Record Types

```rust
// shared/platform-traits/src/records.rs

/// Video file record in index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRecord {
    pub id: Option<i64>,
    pub path: PathBuf,
    pub filename: String,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub file_size_bytes: u64,
    pub file_hash: String,
    pub indexed_at: DateTime<Utc>,
    pub has_transcript: bool,
    pub keyframe_count: u32,
}

/// Key frame record with embedding.
#[derive(Debug, Clone)]
pub struct KeyFrameRecord {
    pub id: Option<i64>,
    pub video_id: i64,
    pub timestamp_ms: u64,
    pub frame_number: u64,
    pub thumbnail_jpeg: Vec<u8>,
    pub embedding: Option<Vec<f32>>,
}

/// Object detection record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRecord {
    pub id: Option<i64>,
    pub video_id: i64,
    pub keyframe_id: i64,
    pub timestamp_ms: u64,
    pub class_name: String,
    pub confidence: f32,
    pub bbox_x: f32,
    pub bbox_y: f32,
    pub bbox_width: f32,
    pub bbox_height: f32,
}

/// Transcript segment record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptRecord {
    pub id: Option<i64>,
    pub video_id: i64,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub confidence: f32,
    pub speaker: Option<String>,
}

/// Unified search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub video_id: i64,
    pub video_path: PathBuf,
    pub timestamp_ms: u64,
    pub result_type: SearchResultType,
    pub score: f32,
    pub snippet: String,
    pub thumbnail: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchResultType {
    Transcript,
    Visual,
    Detection,
}
```

### Deliverable

`shared/platform-traits/src/records.rs`

---

## Milestone 1.3: Apple Local Implementations

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 1.3.1 | Implement `GStreamerDecoder` | VideoToolbox decode | Extracts frames from test video |
| 1.3.2 | Implement `SqliteIndex` | FTS5 + vector search | CRUD operations work |
| 1.3.3 | Implement `LocalBatchProcessor` | Tokio job queue | Processes video batch |
| 1.3.4 | Stub `ClipEmbedding` | Placeholder for Phase 2 | Returns dummy embeddings |

### Code: SQLite Index Implementation

```rust
// platform/apple/host/src/providers/sqlite_index.rs

use rusqlite::{Connection, params};
use tokio::sync::RwLock;

pub struct SqliteIndex {
    conn: RwLock<Connection>,
}

impl SqliteIndex {
    pub async fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Create schema
        conn.execute_batch(include_str!("schema.sql"))?;

        Ok(Self {
            conn: RwLock::new(conn),
        })
    }
}

#[async_trait]
impl IndexProvider for SqliteIndex {
    async fn store_video(&self, video: &VideoRecord) -> Result<i64> {
        let conn = self.conn.write().await;
        conn.execute(
            "INSERT INTO videos (path, filename, duration_ms, width, height, fps, file_size_bytes, file_hash, indexed_at, has_transcript, keyframe_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                video.path.to_string_lossy(),
                video.filename,
                video.duration_ms,
                video.width,
                video.height,
                video.fps,
                video.file_size_bytes,
                video.file_hash,
                video.indexed_at.to_rfc3339(),
                video.has_transcript,
                video.keyframe_count,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn search_fts(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>> {
        let conn = self.conn.read().await;
        let mut stmt = conn.prepare(
            "SELECT t.video_id, v.path, t.start_ms, t.text, bm25(transcripts_fts) as score
             FROM transcripts_fts t
             JOIN videos v ON t.video_id = v.id
             WHERE transcripts_fts MATCH ?1
             ORDER BY score
             LIMIT ?2"
        )?;

        let results = stmt.query_map(params![query, limit], |row| {
            Ok(SearchResult {
                video_id: row.get(0)?,
                video_path: PathBuf::from(row.get::<_, String>(1)?),
                timestamp_ms: row.get(2)?,
                result_type: SearchResultType::Transcript,
                score: row.get(4)?,
                snippet: row.get(3)?,
                thumbnail: None,
            })
        })?;

        results.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    async fn is_indexed(&self, path: &Path) -> Result<bool> {
        let conn = self.conn.read().await;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM videos WHERE path = ?1",
            params![path.to_string_lossy()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    // ... other methods
}
```

### SQL Schema

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
    video_id INTEGER NOT NULL REFERENCES videos(id),
    timestamp_ms INTEGER NOT NULL,
    frame_number INTEGER NOT NULL,
    thumbnail_jpeg BLOB,
    embedding BLOB,
    UNIQUE(video_id, frame_number)
);

CREATE TABLE IF NOT EXISTS detections (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL REFERENCES videos(id),
    keyframe_id INTEGER REFERENCES keyframes(id),
    timestamp_ms INTEGER NOT NULL,
    class_name TEXT NOT NULL,
    confidence REAL NOT NULL,
    bbox_x REAL NOT NULL,
    bbox_y REAL NOT NULL,
    bbox_width REAL NOT NULL,
    bbox_height REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS transcripts (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL REFERENCES videos(id),
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    confidence REAL NOT NULL,
    speaker TEXT
);

-- Full-text search index
CREATE VIRTUAL TABLE IF NOT EXISTS transcripts_fts USING fts5(
    video_id,
    start_ms,
    text,
    content='transcripts',
    content_rowid='id'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS transcripts_ai AFTER INSERT ON transcripts BEGIN
    INSERT INTO transcripts_fts(rowid, video_id, start_ms, text)
    VALUES (new.id, new.video_id, new.start_ms, new.text);
END;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_keyframes_video ON keyframes(video_id);
CREATE INDEX IF NOT EXISTS idx_detections_video ON detections(video_id);
CREATE INDEX IF NOT EXISTS idx_detections_class ON detections(class_name);
CREATE INDEX IF NOT EXISTS idx_transcripts_video ON transcripts(video_id);
```

### Verification

```bash
cargo run -p yama-host-apple -- --index ~/Videos/test.mp4
# Creates index, extracts metadata
```

### Deliverable

`platform/apple/host/src/providers/`

---

## Dependencies

This phase has no dependencies on other phases.

## Blocks

- Phase 2 (Video Indexing) - needs index provider
- Phase 3 (Transcription) - needs decoder and index
- Phase 4 (Inference) - needs VLM provider stub

---

## Checklist

### Milestone 1.1: Core Trait Definitions
- [ ] 1.1.1 Define `VideoDecoder` trait
- [ ] 1.1.2 Define `IndexProvider` trait
- [ ] 1.1.3 Define `BatchProcessor` trait
- [ ] 1.1.4 Define `EmbeddingModel` trait
- [ ] 1.1.5 Define `VlmProvider` trait

### Milestone 1.2: Request/Response Types
- [ ] 1.2.1 Define `VideoRecord`
- [ ] 1.2.2 Define `KeyFrameRecord`
- [ ] 1.2.3 Define `DetectionRecord`
- [ ] 1.2.4 Define `TranscriptRecord`
- [ ] 1.2.5 Define `SearchResult`

### Milestone 1.3: Apple Local Implementations
- [ ] 1.3.1 Implement `GStreamerDecoder`
- [ ] 1.3.2 Implement `SqliteIndex`
- [ ] 1.3.3 Implement `LocalBatchProcessor`
- [ ] 1.3.4 Stub `ClipEmbedding`
