# Phase 3: Transcription

**Duration:** 6 days
**Goal:** Integrate Whisper for speech-to-text and enable full-text search of audio content.

---

## Overview

This phase adds automatic transcription of video audio using Whisper. Transcripts are stored in SQLite with FTS5 for fast full-text search, enabling queries like "find all mentions of 'quarterly revenue'".

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  VIDEO FILE                                                             │
│  ~/Videos/meeting.mp4                                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  AUDIO EXTRACTION (GStreamer)                                           │
│  video → demux → audioconvert → audioresample → WAV 16kHz mono         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  WHISPER INFERENCE                                                      │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ whisper.cpp      │  │ mlx-whisper      │  │ whisper-rs       │     │
│  │ (CPU fallback)   │  │ (Metal GPU)      │  │ (ort bindings)   │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
│                                                                         │
│  Output: Word-level timestamps + text                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  SQLite FTS5 INDEX                                                      │
│                                                                         │
│  transcripts_fts:                                                       │
│  • video_id                                                             │
│  • start_ms, end_ms                                                     │
│  • text (full-text indexed)                                             │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Model Selection

| Model | Parameters | VRAM | Speed Factor | WER (English) |
|-------|------------|------|--------------|---------------|
| tiny.en | 39M | 200MB | 32x | 9.9% |
| base.en | 74M | 400MB | 16x | 6.7% |
| small.en | 244M | 1GB | 6x | 5.0% |
| medium.en | 769M | 3GB | 2x | 4.3% |
| large-v3 | 1.5B | 6GB | 1x | 3.0% |

**Recommendation:**
- **Default:** `base.en` - good balance of speed and accuracy
- **Fast:** `tiny.en` - for quick indexing
- **Quality:** `small.en` - for important content

---

## Milestone 3.1: Audio Extraction

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 3.1.1 | GStreamer audio pipeline | Extract audio to WAV | Correct sample rate |
| 3.1.2 | Handle no-audio videos | Skip gracefully | No crash on silent videos |
| 3.1.3 | Chunked extraction | Process long videos in chunks | Memory bounded |

### Code: Audio Extractor

```rust
// platform/apple/host/src/transcription/audio.rs

pub struct AudioExtractor {
    config: AudioConfig,
}

pub struct AudioConfig {
    /// Sample rate for Whisper (16000 Hz required)
    pub sample_rate: u32,

    /// Mono required for Whisper
    pub channels: u32,

    /// Chunk duration for long videos (seconds)
    pub chunk_duration_seconds: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            chunk_duration_seconds: 300, // 5 minute chunks
        }
    }
}

impl AudioExtractor {
    pub async fn extract(&self, video_path: &Path) -> Result<AudioData> {
        let pipeline_str = format!(
            "filesrc location={path} ! \
             decodebin ! \
             audioconvert ! \
             audioresample ! \
             audio/x-raw,format=F32LE,rate=16000,channels=1 ! \
             appsink name=sink",
            path = video_path.display()
        );

        let pipeline = gst::parse::launch(&pipeline_str)?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow!("Failed to create pipeline"))?;

        let sink = pipeline.by_name("sink")
            .ok_or_else(|| anyhow!("No appsink"))?
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| anyhow!("Not an appsink"))?;

        let mut samples = Vec::new();

        sink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Error)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                    // Convert bytes to f32 samples
                    let chunk: Vec<f32> = map.chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                        .collect();

                    samples.extend(chunk);
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        pipeline.set_state(gst::State::Playing)?;

        // Wait for EOS or error
        let bus = pipeline.bus().ok_or_else(|| anyhow!("No bus"))?;
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            match msg.view() {
                gst::MessageView::Eos(..) => break,
                gst::MessageView::Error(err) => {
                    return Err(anyhow!("GStreamer error: {}", err.error()));
                }
                _ => {}
            }
        }

        pipeline.set_state(gst::State::Null)?;

        Ok(AudioData {
            samples,
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
        })
    }
}

pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u32,
}

impl AudioData {
    pub fn duration_seconds(&self) -> f64 {
        self.samples.len() as f64 / self.sample_rate as f64
    }
}
```

---

## Milestone 3.2: Whisper Integration

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 3.2.1 | whisper.cpp bindings | Load model, run inference | Returns segments |
| 3.2.2 | Word-level timestamps | Enable precise seeking | Timestamps accurate |
| 3.2.3 | Model selection | Allow tiny/base/small | Model loads correctly |

### Code: Whisper Model

```rust
// platform/apple/host/src/transcription/whisper.rs

use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

pub struct WhisperModel {
    ctx: WhisperContext,
    model_name: String,
}

pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub confidence: f32,
    pub words: Vec<WordTimestamp>,
}

pub struct WordTimestamp {
    pub word: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

impl WhisperModel {
    pub fn load(model_path: &Path) -> Result<Self> {
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or_else(|| anyhow!("Invalid path"))?,
            WhisperContextParameters::default(),
        )?;

        let model_name = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(Self { ctx, model_name })
    }

    pub fn transcribe(
        &self,
        audio: &AudioData,
        on_progress: impl Fn(f32),
    ) -> Result<Vec<TranscriptSegment>> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Enable word-level timestamps
        params.set_token_timestamps(true);
        params.set_max_len(0); // No max length per segment

        // English only for speed
        params.set_language(Some("en"));

        // Progress callback
        params.set_progress_callback_safe(move |progress| {
            on_progress(progress as f32 / 100.0);
        });

        // Create state and run inference
        let mut state = self.ctx.create_state()?;
        state.full(params, &audio.samples)?;

        // Extract segments
        let n_segments = state.full_n_segments()?;
        let mut segments = Vec::with_capacity(n_segments as usize);

        for i in 0..n_segments {
            let start_ms = (state.full_get_segment_t0(i)? * 10) as u64;
            let end_ms = (state.full_get_segment_t1(i)? * 10) as u64;
            let text = state.full_get_segment_text(i)?;

            // Get word timestamps
            let n_tokens = state.full_n_tokens(i)?;
            let mut words = Vec::new();

            for j in 0..n_tokens {
                let token_text = state.full_get_token_text(i, j)?;
                if !token_text.trim().is_empty() {
                    let token_data = state.full_get_token_data(i, j)?;
                    words.push(WordTimestamp {
                        word: token_text,
                        start_ms: (token_data.t0 * 10) as u64,
                        end_ms: (token_data.t1 * 10) as u64,
                    });
                }
            }

            segments.push(TranscriptSegment {
                start_ms,
                end_ms,
                text,
                confidence: 0.9, // Whisper doesn't expose per-segment confidence
                words,
            });
        }

        Ok(segments)
    }
}

#[async_trait]
impl TranscriptModel for WhisperModel {
    async fn transcribe(&self, audio: &AudioData) -> Result<Vec<TranscriptSegment>> {
        // Run in blocking thread to not block async runtime
        let samples = audio.samples.clone();
        let model = self.clone(); // Requires WhisperContext to be Clone

        tokio::task::spawn_blocking(move || {
            model.transcribe_sync(&AudioData {
                samples,
                sample_rate: 16000,
                channels: 1,
            }, |_| {})
        })
        .await?
    }
}
```

### Model Download

```bash
# Download models (run once)
mkdir -p ~/.cache/yama/models

# tiny.en (39MB)
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin \
  -o ~/.cache/yama/models/ggml-tiny.en.bin

# base.en (142MB)
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin \
  -o ~/.cache/yama/models/ggml-base.en.bin
```

---

## Milestone 3.3: FTS5 Integration

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 3.3.1 | Store transcripts | Insert segments to SQLite | Data persists |
| 3.3.2 | FTS5 search | Query by keywords | Results ranked correctly |
| 3.3.3 | Snippet generation | Show context around match | Snippets helpful |
| 3.3.4 | Incremental transcription | Skip already-transcribed | Re-run is fast |

### Code: Transcript Storage

```rust
// platform/apple/host/src/transcription/storage.rs

impl SqliteIndex {
    pub async fn store_transcript(&self, segment: &TranscriptRecord) -> Result<i64> {
        let conn = self.conn.write().await;

        conn.execute(
            "INSERT INTO transcripts (video_id, start_ms, end_ms, text, confidence, speaker)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                segment.video_id,
                segment.start_ms,
                segment.end_ms,
                segment.text,
                segment.confidence,
                segment.speaker,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub async fn search_transcripts(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<TranscriptSearchResult>> {
        let conn = self.conn.read().await;

        // Use FTS5 match with snippet
        let mut stmt = conn.prepare(
            "SELECT
                t.video_id,
                v.path,
                v.filename,
                t.start_ms,
                t.end_ms,
                snippet(transcripts_fts, 2, '<mark>', '</mark>', '...', 32) as snippet,
                bm25(transcripts_fts) as score
             FROM transcripts_fts
             JOIN transcripts t ON t.id = transcripts_fts.rowid
             JOIN videos v ON t.video_id = v.id
             WHERE transcripts_fts MATCH ?1
             ORDER BY score
             LIMIT ?2"
        )?;

        let results = stmt.query_map(params![query, limit], |row| {
            Ok(TranscriptSearchResult {
                video_id: row.get(0)?,
                video_path: PathBuf::from(row.get::<_, String>(1)?),
                filename: row.get(2)?,
                start_ms: row.get(3)?,
                end_ms: row.get(4)?,
                snippet: row.get(5)?,
                score: row.get(6)?,
            })
        })?;

        results.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub async fn has_transcript(&self, video_id: i64) -> Result<bool> {
        let conn = self.conn.read().await;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM transcripts WHERE video_id = ?1",
            params![video_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptSearchResult {
    pub video_id: i64,
    pub video_path: PathBuf,
    pub filename: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub snippet: String,
    pub score: f64,
}
```

### FTS5 Query Examples

```sql
-- Simple keyword search
SELECT * FROM transcripts_fts WHERE transcripts_fts MATCH 'revenue';

-- Phrase search
SELECT * FROM transcripts_fts WHERE transcripts_fts MATCH '"quarterly revenue"';

-- Boolean operators
SELECT * FROM transcripts_fts WHERE transcripts_fts MATCH 'revenue OR profit';

-- Prefix search
SELECT * FROM transcripts_fts WHERE transcripts_fts MATCH 'invest*';

-- Near search (words within 10 tokens)
SELECT * FROM transcripts_fts WHERE transcripts_fts MATCH 'NEAR(revenue growth, 10)';
```

---

## Dependencies

- Phase 1 (Foundation) - traits and types
- Phase 2 (Video Indexing) - video metadata

## Blocks

- Phase 5 (Search) - enables transcript search
- Phase 7 (Tools) - transcript queries

---

## Checklist

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
