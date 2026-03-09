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
| `list_sources` | none | SourceInfo[] | <10ms |
| `list_tracks` | source_id | Track[] | <10ms |
| `get_status` | none | SystemStatus | <10ms |

> **Agent Review Addition:** Discovery tools (`list_sources`, `list_tracks`, `get_status`) enable agents to explore available resources without prior knowledge.

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

## Agent Review Additions

### Agent Design Review

The following improvements were identified by agent design review:

| ID | Task | Rationale | Days |
|----|------|-----------|------|
| 7.4.1 | Add `list_sources` tool | Discovery without prior knowledge | 0.5 |
| 7.4.2 | Add `list_tracks` tool | Discovery without prior knowledge | 0.5 |
| 7.4.3 | Add `get_status` tool | System health visibility | 0.5 |
| 7.4.4 | Implement streaming tool results | VLM calls block 1-5s otherwise | 1 |
| 7.4.5 | Add tool-level access control | Per-tool AND per-source permissions | 1 |
| 7.4.6 | Add audit logging | Track all tool invocations | 0.5 |
| 7.4.7 | Define video-specific error types | SourceNotFound, InferenceTimeout, RateLimited | 0.5 |
| 7.4.8 | Implement ProxyTool pattern | Remote deployment tool forwarding | 1 |

**Code: Discovery Tools**

```rust
// shared/agent-sdk/src/tools/discovery.rs

/// List all available video sources.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSources {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceInfo {
    pub source_id: String,
    pub name: String,
    pub url: Option<String>,
    pub status: SourceStatus,
    pub resolution: Option<Resolution>,
    pub fps: Option<u32>,
    pub capabilities: Vec<String>,  // ["detection", "vlm", "tracking"]
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SourceStatus {
    Active,
    Idle,
    Error,
    Disconnected,
}

/// List all active object tracks.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTracks {
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrackInfo {
    pub track_id: u64,
    pub class_name: String,
    pub age_frames: u64,
    pub last_seen_bbox: BoundingBoxOutput,
    pub velocity: Option<VelocityOutput>,
}

/// Get system status.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetStatus {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SystemStatusOutput {
    pub deployment_mode: String,  // "mac_standalone", "mac_jetson", "jetson_standalone"
    pub sources: Vec<SourceInfo>,
    pub models_loaded: Vec<ModelStatusOutput>,
    pub gpu_memory_used_mb: u64,
    pub gpu_memory_total_mb: u64,
    pub uptime_seconds: u64,
}
```

**Code: ProxyTool Pattern (Remote Deployment)**

```rust
// shared/agent-sdk/src/tools/proxy.rs

/// ProxyTool forwards tool calls to a remote AI server (Jetson).
/// Used in Mac+Jetson deployment mode.
pub struct ProxyTool {
    remote_client: Arc<RemoteEventBusClient>,
    local_tool: Arc<dyn Tool>,
    timeout: Duration,
}

impl ProxyTool {
    pub async fn execute(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Check if tool requires remote execution
        if self.requires_remote(tool_name) {
            // Forward to Jetson
            let envelope = envelope(
                "tool.execute.request",
                &self.session_id,
                ToolExecuteRequest {
                    tool_name: tool_name.to_string(),
                    parameters,
                },
            );
            self.remote_client.publish(envelope).await?;

            // Wait for response
            let response = tokio::time::timeout(
                self.timeout,
                self.await_response(tool_name)
            ).await??;

            Ok(response)
        } else {
            // Execute locally
            self.local_tool.execute(tool_name, parameters).await
        }
    }

    fn requires_remote(&self, tool_name: &str) -> bool {
        // Tools that need GPU/video access run on Jetson
        matches!(tool_name,
            "get_current_frame" |
            "get_detections" |
            "analyze_frame" |
            "search_history" |
            "track_object"
        )
    }
}
```

**Code: Video-Specific Error Types**

```rust
// shared/agent-sdk/src/tools/errors.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VideoToolError {
    #[error("Video source '{0}' not found")]
    SourceNotFound(String),

    #[error("Source '{0}' is not active (status: {1})")]
    SourceNotActive(String, String),

    #[error("Inference timed out after {0}ms")]
    InferenceTimeout(u64),

    #[error("Rate limited: max {0} requests per {1}s")]
    RateLimited(u32, u32),

    #[error("Track ID {0} not found on source '{1}'")]
    TrackNotFound(u64, String),

    #[error("Access denied to source '{0}'")]
    AccessDenied(String),

    #[error("Model '{0}' is not loaded")]
    ModelNotLoaded(String),

    #[error("GPU memory insufficient: need {0}MB, available {1}MB")]
    InsufficientMemory(u64, u64),
}

// Map to user-friendly messages
impl VideoToolError {
    pub fn user_message(&self) -> String {
        match self {
            Self::SourceNotFound(id) => format!(
                "Camera '{}' not found. Use list_sources to see available cameras.", id
            ),
            Self::InferenceTimeout(_) => {
                "Analysis is taking longer than expected. Try a simpler query.".to_string()
            }
            Self::RateLimited(max, secs) => format!(
                "Too many requests. Maximum {} per {} seconds.", max, secs
            ),
            _ => self.to_string(),
        }
    }
}
```

### UX Review

Tool execution should be visible to users, not invisible.

| ID | Task | Rationale | Days |
|----|------|-----------|------|
| 7.7.1 | Add tool execution indicator | Users see "Analyzing frame..." during tool calls | 0.5 |
| 7.7.2 | Show tool results summary | "Found 3 vehicles, 2 people" instead of raw JSON | 0.5 |
| 7.7.3 | Display tool errors as user-friendly messages | Not stack traces | 0.5 |
| 7.7.4 | Add tool execution history panel | Review past tool calls | 1 |

**UX: Tool Execution Indicators**

| Tool | Active State | Completion |
|------|--------------|------------|
| `get_current_frame` | "Capturing frame..." | "Frame captured" |
| `get_detections` | "Getting detections..." | "Found {n} objects" |
| `analyze_frame` | "Analyzing with VLM..." | "Analysis complete" |
| `search_history` | "Searching history..." | "Found {n} events" |
| `track_object` | "Tracking object..." | "Tracking active" |

---

---

## Milestone 7.5: Tool Schema for LLM Integration

**Duration:** 2 days

### Rationale

Tool-calling models like Llama-3.1-8B (Groq) require BFCL-compatible JSON schemas. Well-structured schemas improve tool selection accuracy from ~70% to >85%.

**Reference:** [Berkeley Function Calling Leaderboard](https://gorilla.cs.berkeley.edu/leaderboard.html)

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 7.5.1 | Define JSON schema for all video tools | BFCL-compatible format | Schema validates |
| 7.5.2 | Add tool descriptions optimized for LLM | Clear, concise descriptions | LLM understands |
| 7.5.3 | Implement streaming tool results | WebSocket streaming for VLM | Stream works |
| 7.5.4 | Add tool-level access control | Per-tool AND per-source permissions | Permissions enforced |

### Code: BFCL-Compatible Tool Schema

```json
{
  "name": "get_detections",
  "description": "Get current object detections from a video source. Returns bounding boxes, class names, confidence scores, and track IDs for all detected objects.",
  "parameters": {
    "type": "object",
    "properties": {
      "source_id": {
        "type": "string",
        "description": "Video source identifier (e.g., 'cam1', 'parking-lot')"
      },
      "class_filter": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Filter results to specific object classes (e.g., ['person', 'vehicle']). Empty array returns all."
      },
      "min_confidence": {
        "type": "number",
        "minimum": 0,
        "maximum": 1,
        "default": 0.5,
        "description": "Minimum confidence threshold (0.0-1.0)"
      }
    },
    "required": ["source_id"]
  }
}
```

### Tool Schema Registry

```rust
// shared/agent-sdk/src/tools/schema.rs

pub fn video_tool_schemas() -> Vec<ToolSchema> {
    vec![
        ToolSchema {
            name: "get_current_frame".to_string(),
            description: "Capture the current video frame from a source. Returns JPEG image data.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "source_id": { "type": "string", "description": "Video source identifier" },
                    "max_dimension": { "type": "integer", "description": "Max width/height in pixels" }
                },
                "required": ["source_id"]
            }),
        },
        ToolSchema {
            name: "analyze_frame".to_string(),
            description: "Analyze the current video frame using a vision-language model. Use for detailed scene understanding, reading text, identifying specific objects, or answering questions about visual content.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "source_id": { "type": "string", "description": "Video source identifier" },
                    "prompt": { "type": "string", "description": "Analysis question or instruction" },
                    "include_context": { "type": "boolean", "default": true, "description": "Include recent detection history" }
                },
                "required": ["source_id", "prompt"]
            }),
        },
        ToolSchema {
            name: "search_history".to_string(),
            description: "Search detection history for past events. Use for temporal queries like 'what vehicles entered in the last hour' or 'when did the person leave'.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "source_id": { "type": "string", "description": "Video source (optional, empty = all)" },
                    "time_range_seconds": { "type": "integer", "description": "How far back to search in seconds" },
                    "class_filter": { "type": "array", "items": { "type": "string" } },
                    "limit": { "type": "integer", "default": 100 }
                },
                "required": ["time_range_seconds"]
            }),
        },
        // ... additional tools
    ]
}
```

---

## Milestone 7.6: Fine-Tuning Pipeline

**Duration:** 3 days

### Rationale

Custom fine-tuning on video tool-use data can improve tool selection accuracy by 5-15%. LlamaFactory + QLoRA enables efficient training on consumer hardware.

**Reference:** [xLAM Function Calling Fine-Tuning](https://huggingface.co/learn/cookbook/en/function_calling_fine_tuning_llms_on_xlam)

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 7.6.1 | Create video tool-use training dataset | 500-1000 examples | Dataset validated |
| 7.6.2 | Set up LlamaFactory for LoRA fine-tuning | QLoRA 8-bit training | Training runs |
| 7.6.3 | Fine-tune Llama-3.1-8B on video tools | Train on custom dataset | Model converges |
| 7.6.4 | Evaluate fine-tuned vs base model | Compare accuracy | Delta measured |

### Training Data Format

```json
{
  "conversations": [
    {
      "role": "user",
      "content": "What vehicles have entered the parking lot in the last 30 minutes?"
    },
    {
      "role": "assistant",
      "content": null,
      "tool_calls": [
        {
          "name": "search_history",
          "arguments": {
            "source_id": "parking-cam",
            "time_range_seconds": 1800,
            "class_filter": ["car", "truck", "motorcycle", "bus"]
          }
        }
      ]
    }
  ]
}
```

### Fine-Tuning Config

```yaml
# scripts/finetune/config.yaml

model_name_or_path: meta-llama/Llama-3.1-8B-Instruct
dataset: video_tool_calls
output_dir: ./output/llama3-video-tools
finetuning_type: lora
lora_target: q_proj,v_proj
quantization_bit: 8
per_device_train_batch_size: 4
gradient_accumulation_steps: 4
num_train_epochs: 3
learning_rate: 5e-5
```

---

## Checklist

### Milestone 7.1: Tool Definitions
- [ ] 7.1.1 `get_current_frame` tool
- [ ] 7.1.2 `get_detections` tool
- [ ] 7.1.3 `analyze_frame` tool
- [ ] 7.1.4 `search_history` tool
- [ ] 7.1.5 `track_object` tool

### Milestone 7.2: Tool Handlers
- [ ] 7.2.1 Tool execution router
- [ ] 7.2.2 Connect to providers
- [ ] 7.2.3 Async execution
- [ ] 7.2.4 Error handling

### Milestone 7.3: Agent Integration
- [ ] 7.3.1 Register tools with agent
- [ ] 7.3.2 Tool call parsing
- [ ] 7.3.3 Execute and return
- [ ] 7.3.4 Streaming results

### Milestone 7.4: Agent Design Improvements (Agent Review)
- [ ] 7.4.1 Add `list_sources` discovery tool
- [ ] 7.4.2 Add `list_tracks` discovery tool
- [ ] 7.4.3 Add `get_status` discovery tool
- [ ] 7.4.4 Implement streaming tool results for VLM calls
- [ ] 7.4.5 Add tool-level access control
- [ ] 7.4.6 Add audit logging for tool invocations
- [ ] 7.4.7 Define video-specific error types
- [ ] 7.4.8 Implement ProxyTool pattern for remote deployment

### Milestone 7.5: Tool Schema for LLM Integration
- [ ] 7.5.1 Define JSON schema for all video tools (BFCL-compatible)
- [ ] 7.5.2 Add tool descriptions optimized for LLM understanding
- [ ] 7.5.3 Implement streaming tool results for long operations
- [ ] 7.5.4 Add tool-level access control per source

### Milestone 7.6: Fine-Tuning Pipeline
- [ ] 7.6.1 Create video tool-use training dataset (500-1000 examples)
- [ ] 7.6.2 Set up LlamaFactory for LoRA fine-tuning
- [ ] 7.6.3 Fine-tune Llama-3.1-8B on custom video tool schema
- [ ] 7.6.4 Evaluate fine-tuned vs base model accuracy

### Milestone 7.7: UX Improvements (Agent Review)
- [ ] 7.7.1 Add tool execution indicator ("Analyzing frame...")
- [ ] 7.7.2 Show tool results summary (user-friendly format)
- [ ] 7.7.3 Display tool errors as user-friendly messages
- [ ] 7.7.4 Add tool execution history panel
