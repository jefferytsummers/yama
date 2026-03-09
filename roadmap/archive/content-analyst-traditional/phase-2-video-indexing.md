# Phase 2: Video Indexing

**Duration:** 12 days
**Goal:** Build the video indexing pipeline with CLIP embeddings, key frame extraction, and sqlite-vss vector index.

---

## Overview

This phase implements the core indexing pipeline that processes video files into a searchable index. Key frames are extracted, embedded with CLIP, and stored in SQLite alongside video metadata. This enables both visual semantic search and fast metadata queries.

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
│  BATCH INDEXER                                                          │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ 1. Scan Files    │→ │ 2. Extract Meta  │→ │ 3. Key Frames    │     │
│  │ (skip indexed)   │  │ (duration, fps)  │  │ (1 per 5 sec)    │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
│                                                    │                    │
│                                    ┌───────────────┴───────────────┐   │
│                                    ▼                               ▼   │
│                        ┌──────────────────┐            ┌──────────────┐│
│                        │ 4. CLIP Embed    │            │ 5. Thumbnail ││
│                        │ (batch GPU)      │            │ (JPEG 320px) ││
│                        └──────────────────┘            └──────────────┘│
│                                    │                         │         │
│                                    └───────────┬─────────────┘         │
│                                                ▼                        │
│                        ┌─────────────────────────────────────────────┐ │
│                        │ 6. Store in SQLite                          │ │
│                        │ • videos table                              │ │
│                        │ • keyframes table (with embedding blob)     │ │
│                        │ • FTS5 triggers                             │ │
│                        └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone 2.1: Key Frame Extraction

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 2.1.1 | Scene change detection | Detect significant visual changes | Reduces redundant frames |
| 2.1.2 | Uniform sampling fallback | Extract every N seconds | Consistent coverage |
| 2.1.3 | Thumbnail generation | 320px JPEG at quality 80 | Thumbnails under 50KB |
| 2.1.4 | Batch extraction | Process multiple videos | Queue works correctly |

### Code: Key Frame Extractor

```rust
// platform/apple/host/src/indexer/keyframe.rs

pub struct KeyFrameExtractor {
    decoder: Arc<dyn VideoDecoder>,
    config: KeyFrameConfig,
}

pub struct KeyFrameConfig {
    /// Extract frame every N seconds (uniform sampling)
    pub interval_seconds: f64,

    /// Minimum scene change threshold (0.0-1.0)
    pub scene_change_threshold: f64,

    /// Maximum frames per video
    pub max_frames: u32,

    /// Thumbnail max dimension
    pub thumbnail_size: u32,

    /// JPEG quality (1-100)
    pub jpeg_quality: u8,
}

impl Default for KeyFrameConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 5.0,
            scene_change_threshold: 0.3,
            max_frames: 500,
            thumbnail_size: 320,
            jpeg_quality: 80,
        }
    }
}

impl KeyFrameExtractor {
    pub async fn extract(
        &self,
        path: &Path,
        on_progress: impl Fn(u32, u32),
    ) -> Result<Vec<ExtractedKeyFrame>> {
        let metadata = self.decoder.get_metadata(path).await?;
        let duration_sec = metadata.duration_ms as f64 / 1000.0;

        // Calculate expected frame count
        let expected_frames = (duration_sec / self.config.interval_seconds).ceil() as u32;
        let expected_frames = expected_frames.min(self.config.max_frames);

        let mut keyframes = Vec::new();
        let mut last_histogram: Option<Histogram> = None;

        for i in 0..expected_frames {
            let timestamp_ms = (i as f64 * self.config.interval_seconds * 1000.0) as u64;
            let frame = self.decoder.decode_frame(path, timestamp_ms).await?;

            // Scene change detection
            let histogram = compute_histogram(&frame);
            let is_scene_change = last_histogram
                .as_ref()
                .map(|prev| histogram_diff(prev, &histogram) > self.config.scene_change_threshold)
                .unwrap_or(true);

            if is_scene_change {
                let thumbnail = resize_and_encode_jpeg(
                    &frame,
                    self.config.thumbnail_size,
                    self.config.jpeg_quality,
                )?;

                keyframes.push(ExtractedKeyFrame {
                    timestamp_ms,
                    frame_number: (timestamp_ms as f64 * metadata.fps / 1000.0) as u64,
                    thumbnail_jpeg: thumbnail,
                    frame_data: frame,
                });
            }

            last_histogram = Some(histogram);
            on_progress(i + 1, expected_frames);
        }

        Ok(keyframes)
    }
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
# Output: Extracted 24 keyframes from 120s video
```

---

## Milestone 2.2: CLIP Embedding Integration

**Duration:** 4 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 2.2.1 | CLIP model loading | Load ViT-B/32 on startup | Model loads in <5s |
| 2.2.2 | Image preprocessing | Resize 224x224, normalize | Correct tensor shape |
| 2.2.3 | Batch embedding | Process multiple images | GPU utilization >80% |
| 2.2.4 | Text embedding | Embed search queries | Similarity works |

### Model Options

| Model | Dimension | Memory | Speed | Quality |
|-------|-----------|--------|-------|---------|
| ViT-B/32 | 512 | 350MB | Fast | Good |
| ViT-L/14 | 768 | 890MB | Slow | Better |
| ViT-B/16 | 512 | 350MB | Medium | Good |

**Recommendation:** Start with ViT-B/32 for speed, upgrade to ViT-L/14 if quality insufficient.

### Code: CLIP Embedding Model

```rust
// platform/apple/host/src/indexer/clip.rs

use ort::{Environment, Session, Value};

pub struct ClipModel {
    session: Session,
    dimension: usize,
}

impl ClipModel {
    pub fn load(model_path: &Path) -> Result<Self> {
        let environment = Environment::builder()
            .with_name("clip")
            .with_execution_providers([
                // Metal on Mac
                #[cfg(target_os = "macos")]
                ort::ExecutionProvider::CoreML(Default::default()),
            ])
            .build()?;

        let session = Session::builder()?
            .with_intra_threads(4)?
            .with_model_from_file(model_path)?;

        Ok(Self {
            session,
            dimension: 512, // ViT-B/32
        })
    }

    /// Embed a single image.
    pub fn embed_image(&self, image: &ImageData) -> Result<Vec<f32>> {
        let tensor = self.preprocess_image(image)?;
        let outputs = self.session.run(ort::inputs!["image" => tensor]?)?;
        let embedding = outputs["image_embedding"].extract_tensor::<f32>()?;
        Ok(embedding.view().iter().cloned().collect())
    }

    /// Batch embed multiple images.
    pub fn embed_images_batch(&self, images: &[ImageData]) -> Result<Vec<Vec<f32>>> {
        // Stack images into batch tensor
        let batch_tensor = self.preprocess_batch(images)?;
        let outputs = self.session.run(ort::inputs!["image" => batch_tensor]?)?;

        let embedding = outputs["image_embedding"].extract_tensor::<f32>()?;
        let view = embedding.view();
        let batch_size = images.len();

        (0..batch_size)
            .map(|i| {
                let start = i * self.dimension;
                let end = start + self.dimension;
                Ok(view.iter().skip(start).take(self.dimension).cloned().collect())
            })
            .collect()
    }

    /// Embed text query for semantic search.
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self.tokenize(text)?;
        let outputs = self.session.run(ort::inputs!["text" => tokens]?)?;
        let embedding = outputs["text_embedding"].extract_tensor::<f32>()?;
        Ok(embedding.view().iter().cloned().collect())
    }

    fn preprocess_image(&self, image: &ImageData) -> Result<Value> {
        // Resize to 224x224
        let resized = resize(image, 224, 224);

        // Normalize with CLIP mean/std
        let mean = [0.48145466, 0.4578275, 0.40821073];
        let std = [0.26862954, 0.26130258, 0.27577711];

        let normalized: Vec<f32> = resized
            .pixels()
            .flat_map(|p| {
                [
                    (p.r as f32 / 255.0 - mean[0]) / std[0],
                    (p.g as f32 / 255.0 - mean[1]) / std[1],
                    (p.b as f32 / 255.0 - mean[2]) / std[2],
                ]
            })
            .collect();

        Value::from_array(([1, 3, 224, 224], normalized.as_slice()))
    }
}

#[async_trait]
impl EmbeddingModel for ClipModel {
    async fn embed_image(&self, image: &ImageData) -> Result<Embedding> {
        let vector = self.embed_image(image)?;
        Ok(Embedding {
            vector,
            model: "clip-vit-b-32".to_string(),
        })
    }

    async fn embed_text(&self, text: &str) -> Result<Embedding> {
        let vector = self.embed_text(text)?;
        Ok(Embedding {
            vector,
            model: "clip-vit-b-32".to_string(),
        })
    }

    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        cosine_similarity(&a.vector, &b.vector)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}
```

### Verification

```bash
cargo run --example clip_embed -- ~/Videos/test.mp4
# Output: Generated 24 embeddings (512-dim each)
```

---

## Milestone 2.3: Batch Indexer

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 2.3.1 | File scanner | Recursively find video files | Finds all supported formats |
| 2.3.2 | Progress tracking | Real-time progress events | UI shows progress |
| 2.3.3 | Incremental indexing | Skip already-indexed | Re-run is fast |
| 2.3.4 | Error recovery | Continue on single file failure | Logs failures, continues |

### Code: Batch Indexer

```rust
// platform/apple/host/src/indexer/batch.rs

pub struct BatchIndexer {
    decoder: Arc<dyn VideoDecoder>,
    clip: Arc<ClipModel>,
    index: Arc<dyn IndexProvider>,
    config: BatchConfig,
}

impl BatchIndexer {
    pub async fn index_directory(
        &self,
        path: &Path,
        progress_tx: mpsc::Sender<IndexProgress>,
    ) -> Result<BatchResult> {
        // 1. Scan for video files
        let videos = self.scan_videos(path).await?;
        let total = videos.len();

        progress_tx.send(IndexProgress::Scanning { found: total }).await?;

        // 2. Filter out already indexed
        let mut to_process = Vec::new();
        for video in videos {
            if !self.index.is_indexed(&video).await? {
                to_process.push(video);
            }
        }

        let skipped = total - to_process.len();
        let mut processed = 0;
        let mut failed = Vec::new();

        // 3. Process each video
        for (i, video_path) in to_process.iter().enumerate() {
            progress_tx.send(IndexProgress::Processing {
                current: i + 1,
                total: to_process.len(),
                filename: video_path.file_name().unwrap().to_string_lossy().to_string(),
            }).await?;

            match self.index_video(video_path).await {
                Ok(()) => processed += 1,
                Err(e) => {
                    tracing::error!(path = %video_path.display(), error = %e, "Failed to index");
                    failed.push((video_path.clone(), e.to_string()));
                }
            }
        }

        progress_tx.send(IndexProgress::Complete).await?;

        Ok(BatchResult {
            processed,
            skipped: skipped as u32,
            failed,
            duration_seconds: 0, // TODO: track
        })
    }

    async fn index_video(&self, path: &Path) -> Result<()> {
        // 1. Get metadata
        let metadata = self.decoder.get_metadata(path).await?;

        // 2. Extract key frames
        let extractor = KeyFrameExtractor::new(
            self.decoder.clone(),
            KeyFrameConfig::default(),
        );
        let keyframes = extractor.extract(path, |_, _| {}).await?;

        // 3. Generate CLIP embeddings (batched)
        let frame_data: Vec<_> = keyframes.iter().map(|k| &k.frame_data).collect();
        let embeddings = self.clip.embed_images_batch(&frame_data)?;

        // 4. Store in index
        let video_record = VideoRecord {
            id: None,
            path: path.to_path_buf(),
            filename: path.file_name().unwrap().to_string_lossy().to_string(),
            duration_ms: metadata.duration_ms,
            width: metadata.width,
            height: metadata.height,
            fps: metadata.fps,
            file_size_bytes: metadata.file_size_bytes,
            file_hash: compute_file_hash(path)?,
            indexed_at: Utc::now(),
            has_transcript: false,
            keyframe_count: keyframes.len() as u32,
        };

        let video_id = self.index.store_video(&video_record).await?;

        // 5. Store keyframes with embeddings
        for (kf, embedding) in keyframes.iter().zip(embeddings.iter()) {
            let record = KeyFrameRecord {
                id: None,
                video_id,
                timestamp_ms: kf.timestamp_ms,
                frame_number: kf.frame_number,
                thumbnail_jpeg: kf.thumbnail_jpeg.clone(),
                embedding: Some(embedding.clone()),
            };
            self.index.store_keyframe(&record).await?;
        }

        Ok(())
    }

    async fn scan_videos(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let extensions = ["mp4", "mov", "avi", "mkv", "webm", "m4v"];
        let mut videos = Vec::new();

        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if extensions.contains(&ext.to_string_lossy().to_lowercase().as_str()) {
                        videos.push(entry.path().to_path_buf());
                    }
                }
            }
        }

        videos.sort();
        Ok(videos)
    }
}

#[derive(Debug, Clone)]
pub enum IndexProgress {
    Scanning { found: usize },
    Processing { current: usize, total: usize, filename: String },
    Complete,
}
```

### Verification

```bash
cargo run -p yama-host-apple -- --index ~/Videos/
# Output: Indexed 47 videos, skipped 12, failed 2
```

---

## Milestone 2.4: Vector Index (sqlite-vss)

**Duration:** 2-3 days

### Problem

The naive approach computes cosine similarity on ALL embeddings:
- 500 keyframes × 1000 videos = 500K vectors
- Linear scan per query = O(500K) = slow (seconds per query)

### Solution: sqlite-vss

sqlite-vss provides approximate nearest neighbor (ANN) search as a SQLite extension.

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 2.4.1 | Add sqlite-vss dependency | Load extension at runtime | Extension loads |
| 2.4.2 | Create vss_keyframes table | Virtual table for embeddings | Table created |
| 2.4.3 | Insert embeddings on indexing | Populate VSS index during batch | Embeddings stored |
| 2.4.4 | ANN query function | Search by vector similarity | <100ms for 500K embeddings |

### Code: sqlite-vss Schema

```sql
-- Create virtual table for vector similarity search
CREATE VIRTUAL TABLE IF NOT EXISTS vss_keyframes USING vss0(
    embedding(512)  -- CLIP ViT-B/32 dimension
);

-- Insert during indexing (after storing keyframe)
INSERT INTO vss_keyframes(rowid, embedding)
SELECT id, embedding FROM keyframes WHERE embedding IS NOT NULL;

-- ANN query (from Rust)
SELECT k.*, vss_distance_l2(v.embedding, ?1) as distance
FROM vss_keyframes v
JOIN keyframes k ON k.id = v.rowid
ORDER BY distance
LIMIT 100;
```

### Code: Rust Integration

```rust
// platform/apple/host/src/providers/sqlite_index.rs

impl SqliteIndex {
    /// Initialize sqlite-vss extension
    pub async fn init_vss(&self) -> Result<()> {
        let conn = self.conn.write().await;

        // Load the extension
        unsafe {
            conn.load_extension_enable()?;
            conn.load_extension("vss0", None)?;
            conn.load_extension_disable()?;
        }

        // Create virtual table
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS vss_keyframes USING vss0(
                embedding(512)
            );"
        )?;

        Ok(())
    }

    /// Insert embedding into VSS index
    pub async fn index_embedding(&self, keyframe_id: i64, embedding: &[f32]) -> Result<()> {
        let conn = self.conn.write().await;

        let embedding_blob = serialize_embedding(embedding);
        conn.execute(
            "INSERT INTO vss_keyframes(rowid, embedding) VALUES (?1, ?2)",
            params![keyframe_id, embedding_blob],
        )?;

        Ok(())
    }

    /// ANN search using sqlite-vss
    pub async fn search_vss(
        &self,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<VssSearchResult>> {
        let conn = self.conn.read().await;

        let query_blob = serialize_embedding(query_embedding);
        let mut stmt = conn.prepare(
            "SELECT k.id, k.video_id, v.path, k.timestamp_ms, k.thumbnail_jpeg,
                    vss_distance_l2(vss.embedding, ?1) as distance
             FROM vss_keyframes vss
             JOIN keyframes k ON k.id = vss.rowid
             JOIN videos v ON k.video_id = v.id
             ORDER BY distance
             LIMIT ?2"
        )?;

        let results = stmt.query_map(params![query_blob, limit], |row| {
            Ok(VssSearchResult {
                keyframe_id: row.get(0)?,
                video_id: row.get(1)?,
                video_path: PathBuf::from(row.get::<_, String>(2)?),
                timestamp_ms: row.get(3)?,
                thumbnail: row.get(4)?,
                distance: row.get(5)?,
            })
        })?;

        results.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}
```

### Dependencies

```toml
# Cargo.toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled", "load_extension"] }

# Build: download sqlite-vss for macOS
# curl -L https://github.com/asg017/sqlite-vss/releases/download/v0.1.2/sqlite-vss-v0.1.2-macos-aarch64.tar.gz | tar xz
# Place vss0.dylib in ~/.cache/yama/lib/
```

### Fallback

If sqlite-vss is problematic, use `instant-distance` crate (HNSW in pure Rust):

```toml
[dependencies]
instant-distance = "0.6"
```

### Verification

```bash
# Benchmark: index 500K embeddings
cargo run --example vss_benchmark -- --count 500000
# Output: Indexed 500K embeddings in 45s

# Benchmark: query time
cargo run --example vss_query_benchmark -- --queries 100
# Output: Average query time: 45ms (p99: 95ms)
```

---

## Dependencies

- Phase 1 (Foundation) - traits and types

## Blocks

- Phase 5 (Search) - needs embeddings and VSS for semantic search
- Phase 7 (Tools) - needs index for queries

---

## Checklist

### Milestone 2.1: Key Frame Extraction
- [ ] 2.1.1 Scene change detection
- [ ] 2.1.2 Uniform sampling fallback
- [ ] 2.1.3 Thumbnail generation
- [ ] 2.1.4 Batch extraction

### Milestone 2.2: CLIP Embedding Integration
- [ ] 2.2.1 CLIP model loading
- [ ] 2.2.2 Image preprocessing
- [ ] 2.2.3 Batch embedding
- [ ] 2.2.4 Text embedding

### Milestone 2.3: Batch Indexer
- [ ] 2.3.1 File scanner
- [ ] 2.3.2 Progress tracking
- [ ] 2.3.3 Incremental indexing
- [ ] 2.3.4 Error recovery

### Milestone 2.4: Vector Index (sqlite-vss)
- [ ] 2.4.1 Add sqlite-vss dependency
- [ ] 2.4.2 Create vss_keyframes virtual table
- [ ] 2.4.3 Insert embeddings on indexing
- [ ] 2.4.4 ANN query function (<100ms for 500K embeddings)
