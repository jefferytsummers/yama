# Phase 7: Tool System

**Duration:** 8 days
**Goal:** Expose video analysis capabilities as agent tools for the Content Analyst workflow.

---

## Overview

This phase creates a tool-based interface for AI agents to interact with video analysis. Tools provide structured access to search, frame analysis, clip extraction, and reporting. Unlike the archived security monitoring roadmap (real-time tools), these tools are designed for batch operations on video archives.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  USER PROMPT                                                            │
│  "Find all mentions of 'budget cuts' and extract clips"                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  AGENT REASONING (VLM)                                                  │
│                                                                         │
│  1. Understand intent: search + extract                                 │
│  2. Select tools: search_videos → extract_clips                        │
│  3. Execute tools sequentially                                          │
│  4. Synthesize response                                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           ▼                        ▼                        ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ search_videos    │    │ extract_clips    │    │ generate_summary │
│                  │    │                  │    │                  │
│ FTS5 + semantic  │    │ FFmpeg batch     │    │ VLM synthesis    │
│ Returns: results │    │ Returns: paths   │    │ Returns: markdown│
└──────────────────┘    └──────────────────┘    └──────────────────┘
```

---

## Tool Definitions

| Tool | Description | Latency | Output |
|------|-------------|---------|--------|
| `search_videos` | Search across indexed videos | <3s | SearchResult[] |
| `get_frame` | Get frame at specific timestamp | <100ms | Frame (JPEG) |
| `analyze_frame` | VLM analysis of a frame | 1-5s | Analysis text |
| `count_occurrences` | Count objects/events | <3s | Counts + timestamps |
| `extract_clips` | Export matching segments | 10s-5min | File paths |
| `generate_summary` | Create written summary | 5-30s | Markdown |
| `get_video_info` | Get video metadata | <100ms | VideoMetadata |
| `list_indexed_videos` | List all indexed videos | <100ms | VideoRecord[] |

---

## Milestone 7.1: Search Tools

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 7.1.1 | `search_videos` tool | Multi-modal search | Returns ranked results |
| 7.1.2 | `count_occurrences` tool | Count detections/mentions | Accurate counts |
| 7.1.3 | `list_indexed_videos` tool | Discovery tool | Lists all videos |

### Code: Search Tools

```rust
// platform/apple/host/src/tools/search.rs

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Search across all indexed videos using natural language.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchVideos {
    /// Search query (natural language)
    pub query: String,

    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: u32,

    /// Filter to specific video IDs
    #[serde(default)]
    pub video_ids: Vec<i64>,

    /// Only search transcripts (faster)
    #[serde(default)]
    pub transcript_only: bool,
}

fn default_limit() -> u32 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchVideosOutput {
    pub results: Vec<SearchResultOutput>,
    pub total_matches: u32,
    pub query_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResultOutput {
    pub video_id: i64,
    pub video_path: String,
    pub filename: String,
    pub timestamp_ms: u64,
    pub timestamp_formatted: String,
    pub score: f32,
    pub match_type: String,  // "transcript", "visual", "detection"
    pub snippet: String,
    pub thumbnail_base64: Option<String>,
}

/// Count occurrences of objects or phrases across videos.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CountOccurrences {
    /// What to count (object class or phrase)
    pub target: String,

    /// Search type
    pub count_type: CountType,

    /// Filter to specific videos
    #[serde(default)]
    pub video_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CountType {
    /// Count transcript mentions
    Transcript,
    /// Count detected objects
    Detection,
    /// Count both
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CountOccurrencesOutput {
    pub target: String,
    pub total_count: u32,
    pub by_video: Vec<VideoCount>,
    pub timestamps: Vec<OccurrenceTimestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VideoCount {
    pub video_id: i64,
    pub filename: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OccurrenceTimestamp {
    pub video_id: i64,
    pub timestamp_ms: u64,
    pub timestamp_formatted: String,
    pub context: String,
}

/// List all indexed videos.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListIndexedVideos {
    /// Filter by filename pattern (glob)
    #[serde(default)]
    pub filename_pattern: Option<String>,

    /// Sort order
    #[serde(default)]
    pub sort_by: SortBy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    IndexedAt,
    Filename,
    Duration,
    FileSize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListIndexedVideosOutput {
    pub videos: Vec<VideoInfoOutput>,
    pub total_count: u32,
    pub total_duration_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VideoInfoOutput {
    pub id: i64,
    pub filename: String,
    pub path: String,
    pub duration_formatted: String,
    pub duration_seconds: u64,
    pub has_transcript: bool,
    pub keyframe_count: u32,
    pub indexed_at: String,
}
```

---

## Milestone 7.2: Analysis Tools

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 7.2.1 | `get_frame` tool | Get specific frame | Returns JPEG |
| 7.2.2 | `analyze_frame` tool | VLM analysis | Returns text |
| 7.2.3 | `get_video_info` tool | Metadata access | Returns metadata |

### Code: Analysis Tools

```rust
// platform/apple/host/src/tools/analysis.rs

/// Get a frame at a specific timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetFrame {
    /// Video ID or path
    pub video: VideoRef,

    /// Timestamp in milliseconds
    pub timestamp_ms: u64,

    /// Max dimension (scales proportionally)
    #[serde(default = "default_max_dim")]
    pub max_dimension: u32,
}

fn default_max_dim() -> u32 {
    1280
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum VideoRef {
    Id(i64),
    Path(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetFrameOutput {
    pub video_id: i64,
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    /// Base64-encoded JPEG
    pub data: String,
}

/// Analyze a video frame using VLM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeFrame {
    /// Video ID or path
    pub video: VideoRef,

    /// Timestamp in milliseconds
    pub timestamp_ms: u64,

    /// Analysis prompt
    pub prompt: String,

    /// Include surrounding context
    #[serde(default = "default_true")]
    pub include_context: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeFrameOutput {
    pub video_id: i64,
    pub timestamp_ms: u64,
    pub timestamp_formatted: String,
    pub analysis: String,
    pub inference_time_ms: u64,
    pub model: String,
}

/// Get detailed information about a video.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetVideoInfo {
    /// Video ID or path
    pub video: VideoRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetVideoInfoOutput {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub duration_ms: u64,
    pub duration_formatted: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub file_size_bytes: u64,
    pub file_size_formatted: String,
    pub has_transcript: bool,
    pub transcript_segments: u32,
    pub keyframe_count: u32,
    pub detection_count: u32,
    pub indexed_at: String,
}
```

---

## Milestone 7.3: Export Tools

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 7.3.1 | `extract_clips` tool | Batch export | Creates files |
| 7.3.2 | `generate_summary` tool | Written report | Returns markdown |

### Code: Export Tools

```rust
// platform/apple/host/src/tools/export.rs

/// Extract video clips based on search results or timestamps.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractClips {
    /// Source: either search results or explicit timestamps
    pub source: ClipSource,

    /// Output directory (default: ~/exports/yama/{timestamp})
    #[serde(default)]
    pub output_dir: Option<String>,

    /// Padding before match point (seconds)
    #[serde(default = "default_padding")]
    pub padding_before: f64,

    /// Padding after match point (seconds)
    #[serde(default = "default_padding")]
    pub padding_after: f64,

    /// Merge clips within this threshold (seconds)
    #[serde(default = "default_merge")]
    pub merge_threshold: f64,
}

fn default_padding() -> f64 {
    5.0
}

fn default_merge() -> f64 {
    10.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ClipSource {
    /// Use results from a previous search
    SearchResults { result_ids: Vec<String> },
    /// Explicit timestamps
    Timestamps { clips: Vec<ExplicitClip> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExplicitClip {
    pub video: VideoRef,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractClipsOutput {
    pub output_dir: String,
    pub clips: Vec<ExtractedClipInfo>,
    pub total_duration_seconds: f64,
    pub total_size_bytes: u64,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedClipInfo {
    pub path: String,
    pub source_video: String,
    pub start_formatted: String,
    pub end_formatted: String,
    pub duration_seconds: f64,
}

/// Generate a written summary of search results or videos.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateSummary {
    /// What to summarize
    pub source: SummarySource,

    /// Summary style
    #[serde(default)]
    pub style: SummaryStyle,

    /// Maximum length (in approximate words)
    #[serde(default = "default_max_length")]
    pub max_length: u32,
}

fn default_max_length() -> u32 {
    500
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SummarySource {
    /// Summarize search results
    SearchResults { result_ids: Vec<String> },
    /// Summarize specific videos
    Videos { video_ids: Vec<i64> },
    /// Summarize everything
    All,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SummaryStyle {
    #[default]
    Brief,
    Detailed,
    Bullet,
    Narrative,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateSummaryOutput {
    /// Markdown-formatted summary
    pub summary: String,

    /// Key findings with timestamps
    pub key_findings: Vec<KeyFinding>,

    /// Statistics
    pub stats: SummaryStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KeyFinding {
    pub finding: String,
    pub video_id: i64,
    pub timestamp_ms: u64,
    pub timestamp_formatted: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SummaryStats {
    pub videos_analyzed: u32,
    pub total_duration_hours: f64,
    pub transcript_segments: u32,
    pub visual_matches: u32,
}
```

---

## Milestone 7.4: Tool Executor

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 7.4.1 | Tool registry | Register all tools | Tools discoverable |
| 7.4.2 | Execution router | Dispatch by tool name | Correct routing |
| 7.4.3 | Error handling | Graceful failures | Clear messages |
| 7.4.4 | Result caching | Cache search results | Faster follow-up |

### Code: Tool Executor

```rust
// platform/apple/host/src/tools/executor.rs

pub struct ToolExecutor {
    search: Arc<SearchService>,
    index: Arc<dyn IndexProvider>,
    decoder: Arc<dyn VideoDecoder>,
    vlm: Arc<dyn VlmProvider>,
    export: Arc<ExportPipeline>,
    result_cache: Arc<RwLock<HashMap<String, Vec<RankedResult>>>>,
}

impl ToolExecutor {
    pub fn tool_definitions() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "search_videos".to_string(),
                description: "Search across all indexed videos using natural language. \
                    Finds matches in transcripts, visual content, and detected objects.".to_string(),
                parameters: schemars::schema_for!(SearchVideos),
            },
            ToolDefinition {
                name: "get_frame".to_string(),
                description: "Get a video frame at a specific timestamp. \
                    Returns a JPEG image that can be analyzed or displayed.".to_string(),
                parameters: schemars::schema_for!(GetFrame),
            },
            ToolDefinition {
                name: "analyze_frame".to_string(),
                description: "Analyze a video frame using a vision-language model. \
                    Use for detailed understanding of what's happening in a specific moment.".to_string(),
                parameters: schemars::schema_for!(AnalyzeFrame),
            },
            ToolDefinition {
                name: "count_occurrences".to_string(),
                description: "Count how many times something appears across videos. \
                    Can count transcript mentions or detected objects.".to_string(),
                parameters: schemars::schema_for!(CountOccurrences),
            },
            ToolDefinition {
                name: "extract_clips".to_string(),
                description: "Extract video clips matching search results or specific timestamps. \
                    Creates MP4 files with configurable padding.".to_string(),
                parameters: schemars::schema_for!(ExtractClips),
            },
            ToolDefinition {
                name: "generate_summary".to_string(),
                description: "Generate a written summary of search results or videos. \
                    Returns markdown with key findings and timestamps.".to_string(),
                parameters: schemars::schema_for!(GenerateSummary),
            },
            ToolDefinition {
                name: "get_video_info".to_string(),
                description: "Get detailed metadata about a specific video. \
                    Includes duration, resolution, transcript status, etc.".to_string(),
                parameters: schemars::schema_for!(GetVideoInfo),
            },
            ToolDefinition {
                name: "list_indexed_videos".to_string(),
                description: "List all videos that have been indexed. \
                    Shows basic info like filename, duration, and index status.".to_string(),
                parameters: schemars::schema_for!(ListIndexedVideos),
            },
        ]
    }

    pub async fn execute(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match tool_name {
            "search_videos" => {
                let params: SearchVideos = serde_json::from_value(parameters)?;
                let result = self.search_videos(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "get_frame" => {
                let params: GetFrame = serde_json::from_value(parameters)?;
                let result = self.get_frame(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "analyze_frame" => {
                let params: AnalyzeFrame = serde_json::from_value(parameters)?;
                let result = self.analyze_frame(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "count_occurrences" => {
                let params: CountOccurrences = serde_json::from_value(parameters)?;
                let result = self.count_occurrences(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "extract_clips" => {
                let params: ExtractClips = serde_json::from_value(parameters)?;
                let result = self.extract_clips(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "generate_summary" => {
                let params: GenerateSummary = serde_json::from_value(parameters)?;
                let result = self.generate_summary(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "get_video_info" => {
                let params: GetVideoInfo = serde_json::from_value(parameters)?;
                let result = self.get_video_info(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "list_indexed_videos" => {
                let params: ListIndexedVideos = serde_json::from_value(parameters)?;
                let result = self.list_indexed_videos(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            _ => Err(anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    // Implementation methods...
    async fn search_videos(&self, params: SearchVideos) -> Result<SearchVideosOutput> {
        let start = Instant::now();

        let results = self.search.search(&params.query, params.limit).await?;

        // Cache results for potential clip extraction
        let cache_key = uuid::Uuid::new_v4().to_string();
        {
            let mut cache = self.result_cache.write().await;
            cache.insert(cache_key.clone(), results.clone());
        }

        let output_results: Vec<_> = results.iter()
            .map(|r| SearchResultOutput {
                video_id: r.video_id,
                video_path: r.video_path.to_string_lossy().to_string(),
                filename: r.video_path.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                timestamp_ms: r.timestamp_ms,
                timestamp_formatted: format_timestamp(r.timestamp_ms),
                score: r.score,
                match_type: format!("{:?}", r.match_types.first().unwrap_or(&MatchType::Transcript)),
                snippet: r.snippet.clone(),
                thumbnail_base64: r.thumbnail.as_ref().map(base64::encode),
            })
            .collect();

        Ok(SearchVideosOutput {
            results: output_results,
            total_matches: results.len() as u32,
            query_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn format_timestamp(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}
```

---

## Dependencies

- Phase 5 (Search) - search_videos implementation
- Phase 6 (Clip Extraction) - extract_clips implementation
- Phase 4 (Inference) - analyze_frame implementation

## Blocks

- Phase 8 (Export) - tools enable report generation

---

## Checklist

### Milestone 7.1: Search Tools
- [ ] 7.1.1 `search_videos` tool
- [ ] 7.1.2 `count_occurrences` tool
- [ ] 7.1.3 `list_indexed_videos` tool

### Milestone 7.2: Analysis Tools
- [ ] 7.2.1 `get_frame` tool
- [ ] 7.2.2 `analyze_frame` tool
- [ ] 7.2.3 `get_video_info` tool

### Milestone 7.3: Export Tools
- [ ] 7.3.1 `extract_clips` tool
- [ ] 7.3.2 `generate_summary` tool

### Milestone 7.4: Tool Executor
- [ ] 7.4.1 Tool registry
- [ ] 7.4.2 Execution router
- [ ] 7.4.3 Error handling
- [ ] 7.4.4 Result caching
