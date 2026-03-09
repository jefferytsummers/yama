# Phase 6: Clip Extraction

**Duration:** 6 days
**Goal:** Frame-accurate clip extraction using FFmpeg with batch operations.

---

## Overview

This phase implements the clip extraction pipeline that allows Content Analysts to export matching segments from their video library. FFmpeg is used for frame-accurate seeking and high-quality segment extraction.

**Key Decision:** Use FFmpeg (not GStreamer) for extraction because:
1. Frame-accurate seeking with `-ss` before input
2. Better format compatibility
3. Simpler batch processing
4. Re-encoding control for consistent output

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  SEARCH RESULTS (from Phase 5)                                          │
│                                                                         │
│  • meeting_q4.mp4 @ 12:34.500                                          │
│  • meeting_q4.mp4 @ 15:22.100                                          │
│  • earnings_call.mp4 @ 45:12.000                                       │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  CLIP PLANNER                                                           │
│                                                                         │
│  1. Add padding (before/after)                                          │
│  2. Merge adjacent clips (if gap < threshold)                          │
│  3. Generate output filenames                                           │
│  4. Estimate total size                                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  FFMPEG BATCH EXECUTOR                                                  │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  ffmpeg -ss 12:29.500 -i meeting_q4.mp4                          │  │
│  │         -t 10 -c:v libx264 -crf 23 clip_001.mp4                  │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│  Progress: ████████████░░░░░░░░░░ 60%  (3/5 clips)                     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  OUTPUT                                                                 │
│                                                                         │
│  ~/exports/search-2024-03-08/                                          │
│  ├── meeting_q4_12-29-500.mp4                                          │
│  ├── meeting_q4_15-17-100.mp4                                          │
│  ├── earnings_call_45-07-000.mp4                                       │
│  └── manifest.json                                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone 6.1: Clip Planning

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 6.1.1 | Padding calculation | Add configurable before/after | Correct timestamps |
| 6.1.2 | Adjacent clip merging | Merge if gap < threshold | Merged correctly |
| 6.1.3 | Output naming | Sensible filename convention | No conflicts |
| 6.1.4 | Size estimation | Estimate output size | Within 20% accuracy |

### Code: Clip Planner

```rust
// platform/apple/host/src/export/planner.rs

pub struct ClipPlanner {
    config: ClipConfig,
}

pub struct ClipConfig {
    /// Padding before match point (seconds)
    pub padding_before: f64,

    /// Padding after match point (seconds)
    pub padding_after: f64,

    /// Merge clips if gap is less than this (seconds)
    pub merge_threshold: f64,

    /// Output directory
    pub output_dir: PathBuf,

    /// Output format
    pub format: OutputFormat,
}

impl Default for ClipConfig {
    fn default() -> Self {
        Self {
            padding_before: 5.0,
            padding_after: 5.0,
            merge_threshold: 10.0,
            output_dir: PathBuf::from("~/exports"),
            format: OutputFormat::Mp4 { crf: 23 },
        }
    }
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Mp4 { crf: u8 },
    Copy, // Stream copy (fastest, no re-encode)
    Gif { fps: u8, width: u32 },
}

#[derive(Debug, Clone)]
pub struct PlannedClip {
    pub source_path: PathBuf,
    pub start_ms: u64,
    pub end_ms: u64,
    pub output_path: PathBuf,
    pub original_matches: Vec<u64>, // Original match timestamps
}

impl ClipPlanner {
    pub fn plan(&self, results: &[RankedResult]) -> Result<ClipPlan> {
        // Group by video
        let mut by_video: HashMap<PathBuf, Vec<&RankedResult>> = HashMap::new();
        for result in results {
            by_video.entry(result.video_path.clone())
                .or_default()
                .push(result);
        }

        let mut clips = Vec::new();

        for (video_path, matches) in by_video {
            // Sort by timestamp
            let mut sorted: Vec<_> = matches.iter()
                .map(|m| m.timestamp_ms)
                .collect();
            sorted.sort();

            // Apply padding and merge
            let merged = self.merge_clips(&sorted);

            // Create planned clips
            for (i, (start_ms, end_ms, originals)) in merged.into_iter().enumerate() {
                let output_name = self.generate_filename(&video_path, start_ms, i);
                let output_path = self.config.output_dir.join(&output_name);

                clips.push(PlannedClip {
                    source_path: video_path.clone(),
                    start_ms,
                    end_ms,
                    output_path,
                    original_matches: originals,
                });
            }
        }

        // Estimate size
        let estimated_bytes = self.estimate_size(&clips);

        Ok(ClipPlan {
            clips,
            total_duration_ms: clips.iter().map(|c| c.end_ms - c.start_ms).sum(),
            estimated_bytes,
            config: self.config.clone(),
        })
    }

    fn merge_clips(&self, timestamps: &[u64]) -> Vec<(u64, u64, Vec<u64>)> {
        if timestamps.is_empty() {
            return Vec::new();
        }

        let padding_before_ms = (self.config.padding_before * 1000.0) as u64;
        let padding_after_ms = (self.config.padding_after * 1000.0) as u64;
        let merge_threshold_ms = (self.config.merge_threshold * 1000.0) as u64;

        let mut result = Vec::new();
        let mut current_start = timestamps[0].saturating_sub(padding_before_ms);
        let mut current_end = timestamps[0] + padding_after_ms;
        let mut current_originals = vec![timestamps[0]];

        for &ts in &timestamps[1..] {
            let ts_start = ts.saturating_sub(padding_before_ms);
            let ts_end = ts + padding_after_ms;

            // Check if should merge
            if ts_start <= current_end + merge_threshold_ms {
                // Extend current clip
                current_end = current_end.max(ts_end);
                current_originals.push(ts);
            } else {
                // Save current and start new
                result.push((current_start, current_end, current_originals));
                current_start = ts_start;
                current_end = ts_end;
                current_originals = vec![ts];
            }
        }

        // Don't forget the last clip
        result.push((current_start, current_end, current_originals));

        result
    }

    fn generate_filename(&self, source: &Path, start_ms: u64, index: usize) -> String {
        let stem = source.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "clip".to_string());

        let time = format_timestamp_filename(start_ms);

        format!("{}_{}.mp4", stem, time)
    }

    fn estimate_size(&self, clips: &[PlannedClip]) -> u64 {
        // Rough estimate: 1 MB per second at CRF 23
        let total_seconds: f64 = clips.iter()
            .map(|c| (c.end_ms - c.start_ms) as f64 / 1000.0)
            .sum();

        (total_seconds * 1_000_000.0) as u64
    }
}

fn format_timestamp_filename(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;

    if hours > 0 {
        format!("{:02}-{:02}-{:02}-{:03}", hours, minutes, seconds, millis)
    } else {
        format!("{:02}-{:02}-{:03}", minutes, seconds, millis)
    }
}

#[derive(Debug)]
pub struct ClipPlan {
    pub clips: Vec<PlannedClip>,
    pub total_duration_ms: u64,
    pub estimated_bytes: u64,
    pub config: ClipConfig,
}
```

---

## Milestone 6.2: FFmpeg Integration

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 6.2.1 | FFmpeg wrapper | Spawn and manage process | Process completes |
| 6.2.2 | Frame-accurate seeking | Use -ss before input | Accurate to 100ms |
| 6.2.3 | Progress parsing | Parse FFmpeg stderr | Progress updates |
| 6.2.4 | Error handling | Detect and report failures | Clear error messages |

### Code: FFmpeg Executor

```rust
// platform/apple/host/src/export/ffmpeg.rs

use std::process::{Command, Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

pub struct FfmpegExecutor {
    ffmpeg_path: PathBuf,
}

impl FfmpegExecutor {
    pub fn new() -> Result<Self> {
        // Find ffmpeg
        let ffmpeg_path = which::which("ffmpeg")
            .map_err(|_| anyhow!("ffmpeg not found in PATH"))?;

        Ok(Self { ffmpeg_path })
    }

    pub async fn extract_clip(
        &self,
        clip: &PlannedClip,
        format: &OutputFormat,
        progress_tx: mpsc::Sender<FfmpegProgress>,
    ) -> Result<()> {
        // Calculate duration
        let duration_seconds = (clip.end_ms - clip.start_ms) as f64 / 1000.0;

        // Build FFmpeg command
        let mut cmd = TokioCommand::new(&self.ffmpeg_path);

        // Seek BEFORE input for frame-accurate seeking
        cmd.arg("-ss").arg(format!("{:.3}", clip.start_ms as f64 / 1000.0));

        // Input file
        cmd.arg("-i").arg(&clip.source_path);

        // Duration
        cmd.arg("-t").arg(format!("{:.3}", duration_seconds));

        // Output options based on format
        match format {
            OutputFormat::Mp4 { crf } => {
                cmd.args([
                    "-c:v", "libx264",
                    "-crf", &crf.to_string(),
                    "-preset", "medium",
                    "-c:a", "aac",
                    "-b:a", "128k",
                ]);
            }
            OutputFormat::Copy => {
                cmd.args(["-c", "copy"]);
            }
            OutputFormat::Gif { fps, width } => {
                cmd.args([
                    "-vf", &format!("fps={},scale={}:-1:flags=lanczos", fps, width),
                    "-loop", "0",
                ]);
            }
        }

        // Progress reporting
        cmd.arg("-progress").arg("pipe:1");

        // Output file
        cmd.arg("-y"); // Overwrite
        cmd.arg(&clip.output_path);

        // Set up pipes
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        // Parse progress from stdout
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        let progress_tx_clone = progress_tx.clone();
        let duration_ms = clip.end_ms - clip.start_ms;

        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if line.starts_with("out_time_ms=") {
                    if let Ok(ms) = line[12..].parse::<u64>() {
                        let progress = (ms as f64 / duration_ms as f64).min(1.0);
                        let _ = progress_tx_clone.send(FfmpegProgress::Progress(progress)).await;
                    }
                }
            }
        });

        // Wait for completion
        let status = child.wait().await?;

        if status.success() {
            progress_tx.send(FfmpegProgress::Complete).await?;
            Ok(())
        } else {
            // Read stderr for error message
            let stderr = child.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;
            let mut error_msg = String::new();
            BufReader::new(stderr).read_to_string(&mut error_msg).await?;

            progress_tx.send(FfmpegProgress::Error(error_msg.clone())).await?;
            Err(anyhow!("FFmpeg failed: {}", error_msg))
        }
    }

    pub fn probe_duration(&self, path: &Path) -> Result<u64> {
        let output = Command::new(&self.ffmpeg_path)
            .arg("-i")
            .arg(path)
            .arg("-f")
            .arg("null")
            .arg("-")
            .stderr(Stdio::piped())
            .output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse duration from "Duration: HH:MM:SS.ms"
        if let Some(caps) = regex::Regex::new(r"Duration: (\d+):(\d+):(\d+)\.(\d+)")
            .unwrap()
            .captures(&stderr)
        {
            let hours: u64 = caps[1].parse()?;
            let minutes: u64 = caps[2].parse()?;
            let seconds: u64 = caps[3].parse()?;
            let centis: u64 = caps[4].parse()?;

            let total_ms = hours * 3600000 + minutes * 60000 + seconds * 1000 + centis * 10;
            return Ok(total_ms);
        }

        Err(anyhow!("Could not parse duration"))
    }
}

#[derive(Debug, Clone)]
pub enum FfmpegProgress {
    Progress(f64), // 0.0 - 1.0
    Complete,
    Error(String),
}
```

---

## Milestone 6.3: Batch Export Pipeline

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 6.3.1 | Batch orchestration | Process multiple clips | All clips exported |
| 6.3.2 | Parallel execution | Configurable concurrency | Faster than sequential |
| 6.3.3 | Manifest generation | JSON summary of exports | Valid JSON |
| 6.3.4 | Cancellation | Cancel in-progress batch | Stops cleanly |

### Code: Export Pipeline

```rust
// platform/apple/host/src/export/pipeline.rs

pub struct ExportPipeline {
    ffmpeg: FfmpegExecutor,
    config: ExportConfig,
}

pub struct ExportConfig {
    /// Maximum concurrent FFmpeg processes
    pub concurrency: u32,

    /// Continue on individual clip failure
    pub continue_on_error: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            concurrency: 2, // FFmpeg is CPU-intensive
            continue_on_error: true,
        }
    }
}

impl ExportPipeline {
    pub async fn export(
        &self,
        plan: &ClipPlan,
        progress_tx: mpsc::Sender<ExportProgress>,
        cancel_token: CancellationToken,
    ) -> Result<ExportResult> {
        // Create output directory
        std::fs::create_dir_all(&plan.config.output_dir)?;

        let total_clips = plan.clips.len();
        let mut completed = 0;
        let mut failed = Vec::new();

        // Semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency as usize));

        // Process clips with bounded concurrency
        let mut handles = Vec::new();

        for (i, clip) in plan.clips.iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await?;
            let ffmpeg = self.ffmpeg.clone();
            let clip = clip.clone();
            let format = plan.config.format.clone();
            let progress_tx = progress_tx.clone();
            let cancel = cancel_token.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit; // Release on drop

                // Check cancellation
                if cancel.is_cancelled() {
                    return Err(anyhow!("Cancelled"));
                }

                let (tx, mut rx) = mpsc::channel(32);

                // Spawn FFmpeg
                let ffmpeg_handle = tokio::spawn({
                    let clip = clip.clone();
                    let format = format.clone();
                    async move {
                        ffmpeg.extract_clip(&clip, &format, tx).await
                    }
                });

                // Forward progress
                while let Some(prog) = rx.recv().await {
                    match prog {
                        FfmpegProgress::Progress(p) => {
                            progress_tx.send(ExportProgress::ClipProgress {
                                clip_index: i,
                                progress: p,
                            }).await?;
                        }
                        FfmpegProgress::Complete => {
                            progress_tx.send(ExportProgress::ClipComplete {
                                clip_index: i,
                                path: clip.output_path.clone(),
                            }).await?;
                        }
                        FfmpegProgress::Error(e) => {
                            progress_tx.send(ExportProgress::ClipError {
                                clip_index: i,
                                error: e,
                            }).await?;
                        }
                    }
                }

                ffmpeg_handle.await?
            });

            handles.push(handle);
        }

        // Wait for all
        for handle in handles {
            match handle.await {
                Ok(Ok(())) => completed += 1,
                Ok(Err(e)) => {
                    if !self.config.continue_on_error {
                        return Err(e);
                    }
                    failed.push(e.to_string());
                }
                Err(e) => {
                    if !self.config.continue_on_error {
                        return Err(e.into());
                    }
                    failed.push(e.to_string());
                }
            }
        }

        // Generate manifest
        let manifest = self.generate_manifest(&plan, completed, &failed)?;
        let manifest_path = plan.config.output_dir.join("manifest.json");
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

        progress_tx.send(ExportProgress::Complete {
            total: total_clips,
            successful: completed,
            failed: failed.len(),
        }).await?;

        Ok(ExportResult {
            output_dir: plan.config.output_dir.clone(),
            manifest_path,
            successful: completed,
            failed: failed.len(),
        })
    }

    fn generate_manifest(
        &self,
        plan: &ClipPlan,
        successful: usize,
        failed: &[String],
    ) -> Result<ExportManifest> {
        let clips: Vec<_> = plan.clips.iter()
            .enumerate()
            .filter(|(i, _)| *i < successful)
            .map(|(_, clip)| ManifestClip {
                source: clip.source_path.clone(),
                output: clip.output_path.clone(),
                start_ms: clip.start_ms,
                end_ms: clip.end_ms,
                matches: clip.original_matches.clone(),
            })
            .collect();

        Ok(ExportManifest {
            version: 1,
            created_at: Utc::now(),
            query: String::new(), // TODO: pass from search
            clips,
            errors: failed.to_vec(),
        })
    }
}

#[derive(Debug, Clone)]
pub enum ExportProgress {
    ClipProgress { clip_index: usize, progress: f64 },
    ClipComplete { clip_index: usize, path: PathBuf },
    ClipError { clip_index: usize, error: String },
    Complete { total: usize, successful: usize, failed: usize },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportManifest {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub query: String,
    pub clips: Vec<ManifestClip>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestClip {
    pub source: PathBuf,
    pub output: PathBuf,
    pub start_ms: u64,
    pub end_ms: u64,
    pub matches: Vec<u64>,
}
```

---

## Dependencies

- Phase 5 (Search) - provides results to export

## Blocks

- Phase 7 (Tools) - extract_clips tool

---

## Checklist

### Milestone 6.1: Clip Planning
- [ ] 6.1.1 Padding calculation
- [ ] 6.1.2 Adjacent clip merging
- [ ] 6.1.3 Output naming
- [ ] 6.1.4 Size estimation

### Milestone 6.2: FFmpeg Integration
- [ ] 6.2.1 FFmpeg wrapper
- [ ] 6.2.2 Frame-accurate seeking
- [ ] 6.2.3 Progress parsing
- [ ] 6.2.4 Error handling

### Milestone 6.3: Batch Export Pipeline
- [ ] 6.3.1 Batch orchestration
- [ ] 6.3.2 Parallel execution
- [ ] 6.3.3 Manifest generation
- [ ] 6.3.4 Cancellation
