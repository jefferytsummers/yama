# Phase 3: Agent System

**Duration:** 14 days
**Goal:** Build the conversational agent system with presets, tool execution, chat interface, and artifact generation.

---

## Overview

This phase implements the agent-first user experience discovered in the prototype. Instead of traditional "import → search → extract" workflows, users interact through natural language with AI agents configured via presets. The agent orchestrates tools (search, analyze, extract) to accomplish complex tasks autonomously.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  USER INPUT                                                             │
│  "Find all mentions of 'budget cuts' and extract clips with charts"    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  AGENT ORCHESTRATOR                                                     │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ Preset Config    │  │ Chat Manager     │  │ Tool Executor    │     │
│  │                  │  │                  │  │                  │     │
│  │ • video-analyst  │  │ • history        │  │ • search_videos  │     │
│  │ • doc-reporter   │  │ • context        │  │ • analyze_frame  │     │
│  │ • quick-search   │  │ • streaming      │  │ • extract_clips  │     │
│  │ • custom         │  │                  │  │ • generate_summary│    │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐                            │
│  │ Artifact Store   │  │ VLM Backend      │                            │
│  │                  │  │                  │                            │
│  │ • clips          │  │ • Qwen2.5-VL     │                            │
│  │ • reports        │  │ • LLaVA          │                            │
│  │ • summaries      │  │ • MiniCPM-V      │                            │
│  └──────────────────┘  └──────────────────┘                            │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Agent Presets (from prototype)

| Preset | Models | Purpose |
|--------|--------|---------|
| **Video Analyst** | Qwen2.5-VL-3B, YOLOv8-n, RTMPose-m, CLIP | Find moments, detect objects, track motion |
| **Document Reporter** | SmolDockling, Whisper-base, Gemma-3-4B, CLIP | OCR slides, transcribe meetings, generate reports |
| **Quick Search** | CLIP, Whisper-tiny | Fast visual + transcript search |
| **Custom** | User-selected | Domain-specific workflows |

---

## Milestone 3.1: Agent Presets

**Duration:** 2 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 3.1.1 | Define preset schema | JSON config for agent presets | Schema validated by JSON Schema |
| 3.1.2 | Implement preset loader | Load/save preset configs | Presets persist across sessions |
| 3.1.3 | Model bundle validation | Check required models exist | Clear error if model missing |
| 3.1.4 | Default presets | Ship 4 presets from prototype | All presets load correctly |

### Code: Preset Schema

```rust
// shared/platform-traits/src/agent.rs

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentPreset {
    pub id: String,
    pub name: String,
    pub badge: Option<String>,  // "RECOMMENDED", "FAST", "ADVANCED"
    pub tagline: String,
    pub capabilities: Vec<String>,
    pub models: Vec<ModelRequirement>,
    pub tools: Vec<String>,
    pub download_size: String,
    pub requirements: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelRequirement {
    pub name: String,
    pub role: String,
    pub model_type: ModelType,
    pub required: bool,
}
```

### Verification

```bash
cargo test -p yama-host-apple test_preset_loading
# AC-1: All 4 default presets load
# AC-2: Missing model reports clear error
```

---

## Milestone 3.2: Tool Definitions

**Duration:** 3 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 3.2.1 | search_videos | Multi-modal search | Returns results in <3s |
| 3.2.2 | get_frame | Frame at timestamp | Returns JPEG in <100ms |
| 3.2.3 | analyze_frame | VLM analysis | Returns text in <5s |
| 3.2.4 | extract_clips | Batch FFmpeg export | Creates MP4 files |
| 3.2.5 | generate_summary | VLM synthesis | Returns markdown |
| 3.2.6 | count_occurrences | Count detections/mentions | Returns counts + timestamps |
| 3.2.7 | get_video_info | Video metadata | Returns VideoMetadata |
| 3.2.8 | list_indexed_videos | List all videos | Returns VideoRecord[] |

### Code: Tool Definitions

```rust
// platform/apple/host/src/tools/mod.rs

/// Search across all indexed videos using natural language.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchVideos {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub video_ids: Vec<i64>,
    #[serde(default)]
    pub transcript_only: bool,
}

/// Extract video clips based on search results or timestamps.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractClips {
    pub source: ClipSource,
    pub output_dir: Option<String>,
    #[serde(default = "default_padding")]
    pub padding_before: f64,
    #[serde(default = "default_padding")]
    pub padding_after: f64,
    #[serde(default = "default_merge")]
    pub merge_threshold: f64,
}

/// Analyze a video frame using VLM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeFrame {
    pub video: VideoRef,
    pub timestamp_ms: u64,
    pub prompt: String,
    #[serde(default = "default_true")]
    pub include_context: bool,
}
```

### Verification

```bash
cargo test -p yama-host-apple test_tools
# AC-1: search_videos returns results in <3s
# AC-2: extract_clips creates valid MP4 files
# AC-3: analyze_frame returns coherent analysis
```

---

## Milestone 3.3: Tool Executor

**Duration:** 3 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 3.3.1 | Tool registry | Register all tools with schemas | `tool_definitions()` returns 8 tools |
| 3.3.2 | Execution router | Dispatch by tool name | Correct handler called |
| 3.3.3 | Error handling | Graceful failures | Clear error messages |
| 3.3.4 | Result caching | Cache search results | Follow-up queries use cache |
| 3.3.5 | Streaming output | Stream long operations | Progress visible during extract |

### Code: ToolExecutor

```rust
// platform/apple/host/src/tools/executor.rs

pub struct ToolExecutor {
    search: Arc<SearchService>,
    index: Arc<dyn IndexProvider>,
    decoder: Arc<dyn VideoDecoder>,
    vlm: Arc<dyn VlmProvider>,
    export: Arc<ExportPipeline>,
    result_cache: Arc<RwLock<HashMap<String, Vec<SearchResult>>>>,
}

impl ToolExecutor {
    pub fn tool_definitions() -> Vec<ToolDefinition>;

    pub async fn execute(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value>;
}
```

### Verification

```bash
cargo run --example tool_executor -- search_videos '{"query": "presentation"}'
# AC-1: Returns JSON search results
# AC-2: Error for unknown tool name
# AC-3: Cached results used for follow-up
```

---

## Milestone 3.4: Chat Interface

**Duration:** 4 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 3.4.1 | Chat history | Store conversation state | History persists per project |
| 3.4.2 | Message streaming | Stream VLM responses | Text appears incrementally |
| 3.4.3 | Tool call rendering | Show tool use in chat | Tool calls visible as blocks |
| 3.4.4 | Context management | Maintain conversation context | Agent remembers prior turns |
| 3.4.5 | egui chat widget | Native chat component | Keyboard shortcuts work |

### Code: ChatManager

```rust
// platform/apple/host/src/chat/mod.rs

pub struct ChatManager {
    history: Vec<ChatMessage>,
    context: ConversationContext,
    vlm: Arc<dyn VlmProvider>,
    tool_executor: Arc<ToolExecutor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl ChatManager {
    pub async fn send_message(
        &mut self,
        content: &str,
        stream_tx: mpsc::Sender<StreamChunk>,
    ) -> Result<ChatMessage>;
}
```

### Verification

```bash
cargo run -p yama-host-apple
# AC-1: Chat interface accepts input
# AC-2: Responses stream incrementally
# AC-3: Tool calls shown in chat
# AC-4: Enter key sends, Shift+Enter for newline
```

---

## Milestone 3.5: Artifact Store

**Duration:** 2 days

### Tasks

| ID | Task | Description | Acceptance Criteria |
|----|------|-------------|---------------------|
| 3.5.1 | Define artifact types | Clips, reports, summaries | Schema covers all types |
| 3.5.2 | Storage backend | SQLite + filesystem | Artifacts persist |
| 3.5.3 | Artifact viewer | Preview in UI | Clips play, reports render |
| 3.5.4 | Export options | Download, share | Files export correctly |

### Code: ArtifactStore

```rust
// platform/apple/host/src/artifacts/mod.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub project_id: String,
    pub artifact_type: ArtifactType,
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    VideoClip,
    Report,
    Summary,
    Screenshot,
}

pub struct ArtifactStore {
    db: Arc<dyn IndexProvider>,
    artifacts_dir: PathBuf,
}

impl ArtifactStore {
    pub async fn create(&self, artifact: Artifact) -> Result<String>;
    pub async fn get(&self, id: &str) -> Result<Artifact>;
    pub async fn list(&self, project_id: &str) -> Result<Vec<Artifact>>;
    pub async fn delete(&self, id: &str) -> Result<()>;
}
```

### Verification

```bash
cargo test -p yama-host-apple test_artifacts
# AC-1: Clip artifact created from extract_clips
# AC-2: Artifact appears in project view
# AC-3: Export downloads file
```

---

## Dependencies

- Phase 1 (Core Infrastructure) - ModelManager, IndexProvider
- Phase 2 (Indexing & Embeddings) - Search capabilities, transcripts

## Blocks

- Phase 4 (Jetson Deployment) - needs agent system for remote execution

---

## Checklist

### Milestone 3.1: Agent Presets
- [ ] 3.1.1 Define preset schema
- [ ] 3.1.2 Implement preset loader
- [ ] 3.1.3 Model bundle validation
- [ ] 3.1.4 Default presets

### Milestone 3.2: Tool Definitions
- [ ] 3.2.1 search_videos
- [ ] 3.2.2 get_frame
- [ ] 3.2.3 analyze_frame
- [ ] 3.2.4 extract_clips
- [ ] 3.2.5 generate_summary
- [ ] 3.2.6 count_occurrences
- [ ] 3.2.7 get_video_info
- [ ] 3.2.8 list_indexed_videos

### Milestone 3.3: Tool Executor
- [ ] 3.3.1 Tool registry
- [ ] 3.3.2 Execution router
- [ ] 3.3.3 Error handling
- [ ] 3.3.4 Result caching
- [ ] 3.3.5 Streaming output

### Milestone 3.4: Chat Interface
- [ ] 3.4.1 Chat history
- [ ] 3.4.2 Message streaming
- [ ] 3.4.3 Tool call rendering
- [ ] 3.4.4 Context management
- [ ] 3.4.5 egui chat widget

### Milestone 3.5: Artifact Store
- [ ] 3.5.1 Define artifact types
- [ ] 3.5.2 Storage backend
- [ ] 3.5.3 Artifact viewer
- [ ] 3.5.4 Export options
