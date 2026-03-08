# Phase 7: Tool System & Agent Integration

**Duration:** 8 days
**Goal:** Expose video analysis as agent tools.

---

## Overview

This phase creates a tool-based interface for agents to interact with video analysis. Tools provide structured access to video frames, detections, VLM analysis, and historical context. Agents use these tools through a standard tool execution protocol.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Agent Reasoning (LLM)                                                  │
│                                                                         │
│  1. Parse user intent                                                   │
│  2. Select appropriate tools                                            │
│  3. Execute tools                                                       │
│  4. Synthesize response                                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           ▼                        ▼                        ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ get_current_frame│    │ get_detections   │    │ analyze_frame    │
│                  │    │                  │    │                  │
│ Returns: JPEG    │    │ Returns:         │    │ Returns:         │
│ Latency: <50ms   │    │ Detection[]      │    │ VLM analysis     │
│                  │    │ Latency: <10ms   │    │ Latency: 1-5s    │
└──────────────────┘    └──────────────────┘    └──────────────────┘
           │                        │                        │
           └────────────────────────┼────────────────────────┘
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Video Analysis Backend (via Providers)                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Tool Definitions

| Tool | Input | Output | Latency |
|------|-------|--------|---------|
| `get_current_frame` | source_id | JPEG image | <50ms |
| `get_detections` | source_id, filters | Detection[] | <10ms (cached) |
| `analyze_frame` | source_id, prompt | VLM analysis | 1-5s |
| `search_history` | query, time_range | Event[] | <100ms |
| `track_object` | track_id | TrackedObject | <10ms |

---

## Milestone 7.1: Tool Definitions

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 7.1.1 | `get_current_frame` tool | JSON schema valid | Schema validates |
| 7.1.2 | `get_detections` tool | With filters | Filter support |
| 7.1.3 | `analyze_frame` tool | VLM trigger | Analysis returns |
| 7.1.4 | `search_history` tool | Context query | History returned |
| 7.1.5 | `track_object` tool | Start/stop tracking | Tracking works |

### Code: Tool Schemas

```rust
// shared/agent-sdk/src/tools/video.rs

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Get the current frame from a video source.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCurrentFrame {
    /// Video source identifier (e.g., "cam1", "parking-lot")
    pub source_id: String,

    /// Image format for output
    #[serde(default = "default_format")]
    pub format: ImageFormat,

    /// Maximum dimension (scales to fit)
    #[serde(default)]
    pub max_dimension: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    #[default]
    Jpeg,
    Png,
    Base64,
}

fn default_format() -> ImageFormat {
    ImageFormat::Jpeg
}

/// Get current detections from a video source.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetDetections {
    /// Video source identifier
    pub source_id: String,

    /// Filter by object classes (empty = all)
    #[serde(default)]
    pub class_filter: Vec<String>,

    /// Minimum confidence threshold (0.0 - 1.0)
    #[serde(default = "default_confidence")]
    pub min_confidence: f32,

    /// Include tracked objects
    #[serde(default = "default_true")]
    pub include_tracks: bool,
}

fn default_confidence() -> f32 {
    0.5
}

fn default_true() -> bool {
    true
}

/// Analyze a frame with VLM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeFrame {
    /// Video source identifier
    pub source_id: String,

    /// Analysis prompt/question
    pub prompt: String,

    /// Include recent detection context
    #[serde(default = "default_true")]
    pub include_context: bool,

    /// Context window in seconds
    #[serde(default = "default_context_window")]
    pub context_window_seconds: u32,
}

fn default_context_window() -> u32 {
    60
}

/// Search detection history.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchHistory {
    /// Video source identifier (empty = all sources)
    #[serde(default)]
    pub source_id: Option<String>,

    /// Time range in seconds (from now)
    pub time_range_seconds: u32,

    /// Filter by object classes
    #[serde(default)]
    pub class_filter: Vec<String>,

    /// Search for specific track IDs
    #[serde(default)]
    pub track_ids: Vec<u64>,

    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    100
}

/// Track a specific object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrackObject {
    /// Video source identifier
    pub source_id: String,

    /// Object track ID to monitor
    pub track_id: u64,

    /// Action to take
    pub action: TrackAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrackAction {
    /// Start monitoring this track
    Start,
    /// Stop monitoring
    Stop,
    /// Get current position
    GetPosition,
}

// Tool outputs

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FrameOutput {
    pub source_id: String,
    pub frame_number: u64,
    pub timestamp: String,
    pub width: u32,
    pub height: u32,
    pub data: String,  // Base64 or URL
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectionOutput {
    pub class_name: String,
    pub class_id: u32,
    pub confidence: f32,
    pub bbox: BoundingBoxOutput,
    pub track_id: Option<u64>,
    pub track_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BoundingBoxOutput {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalysisOutput {
    pub analysis: String,
    pub frame_reference: FrameReference,
    pub context_used: Option<ContextSummary>,
    pub inference_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FrameReference {
    pub source_id: String,
    pub frame_number: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContextSummary {
    pub detection_count: u32,
    pub description_count: u32,
}
```

### Verification

```bash
cargo test -p yama-tools -- schema_validation
```

### Deliverable

`shared/agent-sdk/src/tools/video.rs`

---

## Milestone 7.2: Tool Handlers

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 7.2.1 | Tool execution router | Dispatch by name | Correct dispatch |
| 7.2.2 | Connect to providers | Use trait implementations | Providers called |
| 7.2.3 | Async execution | Tools can take seconds | Async works |
| 7.2.4 | Error handling | Graceful failures | Errors reported |

### Code: Tool Executor

```rust
// shared/agent-sdk/src/tools/handlers.rs

use crate::tools::video::*;
use yama_platform_traits::{
    AIServerProvider, VideoProvider, DetectionProvider, ContextStore,
};

pub struct ToolExecutor {
    video_provider: Arc<dyn VideoProvider>,
    detection_provider: Arc<dyn DetectionProvider>,
    ai_server: Arc<dyn AIServerProvider>,
    context_store: Arc<dyn ContextStore>,
}

impl ToolExecutor {
    pub async fn execute(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match tool_name {
            "get_current_frame" => {
                let params: GetCurrentFrame = serde_json::from_value(parameters)?;
                let result = self.get_current_frame(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "get_detections" => {
                let params: GetDetections = serde_json::from_value(parameters)?;
                let result = self.get_detections(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "analyze_frame" => {
                let params: AnalyzeFrame = serde_json::from_value(parameters)?;
                let result = self.analyze_frame(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "search_history" => {
                let params: SearchHistory = serde_json::from_value(parameters)?;
                let result = self.search_history(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            "track_object" => {
                let params: TrackObject = serde_json::from_value(parameters)?;
                let result = self.track_object(params).await?;
                Ok(serde_json::to_value(result)?)
            }
            _ => Err(anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    async fn get_current_frame(&self, params: GetCurrentFrame) -> Result<FrameOutput> {
        let frame = self.video_provider
            .get_frame(&params.source_id)
            .await?;

        let data = match params.format {
            ImageFormat::Jpeg => {
                let jpeg = frame.to_jpeg(params.max_dimension)?;
                base64::encode(&jpeg)
            }
            ImageFormat::Png => {
                let png = frame.to_png(params.max_dimension)?;
                base64::encode(&png)
            }
            ImageFormat::Base64 => {
                base64::encode(&frame.data)
            }
        };

        Ok(FrameOutput {
            source_id: frame.source_id,
            frame_number: frame.frame_number,
            timestamp: frame.timestamp.to_rfc3339(),
            width: frame.width,
            height: frame.height,
            data,
        })
    }

    async fn get_detections(&self, params: GetDetections) -> Result<Vec<DetectionOutput>> {
        let detections = self.detection_provider
            .get_detections(&params.source_id)
            .await?;

        let filtered: Vec<_> = detections.into_iter()
            .filter(|d| d.confidence >= params.min_confidence)
            .filter(|d| {
                params.class_filter.is_empty()
                    || params.class_filter.contains(&d.class_name)
            })
            .map(|d| DetectionOutput {
                class_name: d.class_name,
                class_id: d.class_id,
                confidence: d.confidence,
                bbox: BoundingBoxOutput {
                    x: d.bbox.x,
                    y: d.bbox.y,
                    width: d.bbox.width,
                    height: d.bbox.height,
                },
                track_id: if params.include_tracks { d.track_id } else { None },
                track_state: d.track_state.map(|s| format!("{:?}", s)),
            })
            .collect();

        Ok(filtered)
    }

    async fn analyze_frame(&self, params: AnalyzeFrame) -> Result<AnalysisOutput> {
        let start = Instant::now();

        let request = VideoQueryRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            query: params.prompt,
            source_ids: vec![params.source_id],
            context: QueryContext {
                include_recent_detections: params.include_context,
                include_recent_descriptions: params.include_context,
                context_window_seconds: params.context_window_seconds,
            },
        };

        let response = self.ai_server.query(request).await?;

        Ok(AnalysisOutput {
            analysis: response.analysis,
            frame_reference: FrameReference {
                source_id: response.frame_reference.source_id,
                frame_number: response.frame_reference.frame_number,
                timestamp: response.frame_reference.timestamp.to_rfc3339(),
            },
            context_used: response.context_used.map(|c| ContextSummary {
                detection_count: c.detection_count,
                description_count: c.description_count,
            }),
            inference_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn search_history(&self, params: SearchHistory) -> Result<Vec<DetectionOutput>> {
        let query = DetectionQuery {
            source_id: params.source_id,
            time_range: Duration::from_secs(params.time_range_seconds as u64),
            class_filter: if params.class_filter.is_empty() {
                None
            } else {
                Some(params.class_filter)
            },
            track_ids: if params.track_ids.is_empty() {
                None
            } else {
                Some(params.track_ids)
            },
            limit: params.limit,
        };

        let events = self.detection_provider.search(query).await?;

        let outputs: Vec<_> = events.into_iter()
            .flat_map(|e| e.detections)
            .map(|d| DetectionOutput {
                class_name: d.class_name,
                class_id: d.class_id,
                confidence: d.confidence,
                bbox: BoundingBoxOutput {
                    x: d.bbox.x,
                    y: d.bbox.y,
                    width: d.bbox.width,
                    height: d.bbox.height,
                },
                track_id: d.track_id,
                track_state: None,
            })
            .collect();

        Ok(outputs)
    }
}
```

### Verification

```bash
cargo run --example tool_executor -- get_detections '{"source_id":"cam1"}'
# Returns JSON array
```

### Deliverable

`shared/agent-sdk/src/tools/handlers.rs`

---

## Milestone 7.3: Agent Integration

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 7.3.1 | Register tools with agent | Tools in capability list | Tools listed |
| 7.3.2 | Tool call parsing | Agent selects tools | Calls parsed |
| 7.3.3 | Execute and return | Output feeds to agent | Results returned |
| 7.3.4 | Streaming results | For long VLM calls | Streaming works |

### Code: Agent Tool Registry

```rust
// containers/agent/src/tools.rs

use yama_agent_sdk::tools::{ToolDefinition, ToolExecutor};

pub struct VideoToolRegistry {
    executor: Arc<ToolExecutor>,
}

impl VideoToolRegistry {
    pub fn tool_definitions() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "get_current_frame".to_string(),
                description: "Get the current video frame from a source. Returns a JPEG image.".to_string(),
                parameters: schemars::schema_for!(GetCurrentFrame),
            },
            ToolDefinition {
                name: "get_detections".to_string(),
                description: "Get current object detections from a video source. Returns bounding boxes, classes, and confidence scores.".to_string(),
                parameters: schemars::schema_for!(GetDetections),
            },
            ToolDefinition {
                name: "analyze_frame".to_string(),
                description: "Analyze the current video frame using a vision-language model. Use for detailed scene understanding.".to_string(),
                parameters: schemars::schema_for!(AnalyzeFrame),
            },
            ToolDefinition {
                name: "search_history".to_string(),
                description: "Search detection history for past events. Useful for temporal queries.".to_string(),
                parameters: schemars::schema_for!(SearchHistory),
            },
            ToolDefinition {
                name: "track_object".to_string(),
                description: "Monitor a specific tracked object. Get position updates or start/stop tracking.".to_string(),
                parameters: schemars::schema_for!(TrackObject),
            },
        ]
    }
}

// Agent reasoning loop integration
pub async fn handle_tool_call(
    executor: &ToolExecutor,
    tool_call: &ToolCall,
) -> Result<ToolResult> {
    let start = Instant::now();

    let result = executor
        .execute(&tool_call.name, tool_call.parameters.clone())
        .await;

    match result {
        Ok(output) => Ok(ToolResult {
            tool_call_id: tool_call.id.clone(),
            output: serde_json::to_string_pretty(&output)?,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        }),
        Err(e) => Ok(ToolResult {
            tool_call_id: tool_call.id.clone(),
            output: String::new(),
            error: Some(e.to_string()),
            duration_ms: start.elapsed().as_millis() as u64,
        }),
    }
}
```

### Example Reasoning Sequence

```
┌─────────────────────────────────────────────────────────────────────────┐
│  User: "What vehicles have entered the parking lot in the last hour?"   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Agent Reasoning (LLM)                                                  │
│                                                                         │
│  1. Parse intent: temporal query about vehicles                         │
│  2. Select tools:                                                       │
│     - search_history(class="vehicle", time_range=3600s)                │
│     - get_detections(source_id="parking", class_filter=["car","truck"])│
│  3. Execute tools in parallel                                           │
│  4. Synthesize response                                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           ▼                        ▼                        ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ search_history   │    │ get_detections   │    │ (optional)       │
│                  │    │                  │    │ analyze_frame    │
│ → 15 vehicle     │    │ → 3 current      │    │                  │
│   entry events   │    │   vehicles       │    │                  │
└──────────────────┘    └──────────────────┘    └──────────────────┘
           │                        │                        │
           └────────────────────────┼────────────────────────┘
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Response Synthesis                                                     │
│                                                                         │
│  "In the last hour, 15 vehicles entered the parking lot:                │
│   - 8 sedans                                                            │
│   - 5 SUVs                                                              │
│   - 2 pickup trucks                                                     │
│   Currently, there are 3 vehicles in frame: [descriptions]"            │
└─────────────────────────────────────────────────────────────────────────┘
```

### Verification

```bash
cargo run -p yama-host-apple -- --chat
> What vehicles are in the lot?
# Agent uses tools to answer
```

### Deliverable

Agent integration in `containers/agent/`

---

## Dependencies

- Phase 1 (Abstraction Layer) - provider traits
- Phase 5 (VLM Orchestration) - backend services

## Blocks

- Phase 8 (E2E Integration) - agent chat requires tools

---

## Checklist

- [ ] 7.1.1 `get_current_frame` tool
- [ ] 7.1.2 `get_detections` tool
- [ ] 7.1.3 `analyze_frame` tool
- [ ] 7.1.4 `search_history` tool
- [ ] 7.1.5 `track_object` tool
- [ ] 7.2.1 Tool execution router
- [ ] 7.2.2 Connect to providers
- [ ] 7.2.3 Async execution
- [ ] 7.2.4 Error handling
- [ ] 7.3.1 Register tools with agent
- [ ] 7.3.2 Tool call parsing
- [ ] 7.3.3 Execute and return
- [ ] 7.3.4 Streaming results
