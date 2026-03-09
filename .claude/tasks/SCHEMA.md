# Task Tracking Schema

## Purpose

This schema defines the format for tracking implementation tasks. Tasks can be created and updated using Claude Code's task tools (`TaskCreate`, `TaskUpdate`, `TaskList`, `TaskGet`).

---

## Task Fields

### Core Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | auto | Unique identifier (e.g., "1.1.1") |
| `subject` | string | yes | Short task title (imperative form) |
| `description` | string | yes | Detailed requirements |
| `status` | enum | auto | pending, in_progress, completed |

### Extended Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `activeForm` | string | no | Present continuous form for spinner (e.g., "Running tests") |
| `blockedBy` | string[] | no | Task IDs that must complete first |
| `blocks` | string[] | no | Task IDs waiting on this task |
| `metadata` | object | no | Arbitrary key-value pairs |

---

## Task ID Convention

Tasks follow the pattern: `{PHASE}.{MILESTONE}.{TASK}`

Examples:
- `1.1.1` - Phase 1, Milestone 1, Task 1
- `2.3.4` - Phase 2, Milestone 3, Task 4
- `3.2` - Phase 3, Milestone 2 (milestone-level task)

---

## Status Values

| Status | Description | Transitions From |
|--------|-------------|------------------|
| `pending` | Not started | (initial) |
| `in_progress` | Currently being worked on | pending |
| `completed` | Finished and verified | in_progress |
| `blocked` | Waiting on dependencies | pending |

---

## Example Tasks

### Simple Task

```yaml
id: "1.1.1"
subject: "Define VideoDecoder trait"
description: |
  Create the VideoDecoder trait in shared/platform-traits/src/decoder.rs
  with methods: get_metadata, extract_keyframes, decode_frame, extract_audio
status: pending
activeForm: "Defining VideoDecoder trait"
```

### Task with Dependencies

```yaml
id: "2.2.1"
subject: "Implement CLIP model loading"
description: |
  Load ViT-B/32 CLIP model via ONNX Runtime with Metal acceleration.
  Model should load in <5s on M1 Mac.
status: blocked
activeForm: "Loading CLIP model"
blockedBy:
  - "1.4.1"  # ModelManager registry
metadata:
  model: "clip-vit-b-32"
  memory_budget_mb: 350
```

### Task with Acceptance Criteria

```yaml
id: "2.4.4"
subject: "Implement ANN query for sqlite-vss"
description: |
  Add search_vss method to SqliteIndex that performs approximate
  nearest neighbor search using sqlite-vss extension.

  Acceptance Criteria:
  - AC-1: Query returns top-100 results
  - AC-2: Response time <100ms for 500K embeddings
  - AC-3: Results sorted by distance ascending
status: pending
activeForm: "Implementing ANN query"
metadata:
  acceptance_criteria:
    - "Query time <100ms for 500K embeddings"
    - "Returns top-100 results by distance"
```

---

## Metadata Conventions

### Common Metadata Keys

| Key | Type | Description |
|-----|------|-------------|
| `package` | string | Cargo package name |
| `test_command` | string | Test command to verify |
| `acceptance_criteria` | string[] | List of testable criteria |
| `files_created` | string[] | New files to create |
| `files_modified` | string[] | Existing files to modify |

### Example with Metadata

```yaml
id: "1.3.1"
subject: "Implement VideoDecoder with GStreamer"
description: "Implement VideoDecoder trait using GStreamer + VideoToolbox"
status: pending
activeForm: "Implementing GStreamer decoder"
metadata:
  package: "yama-host-apple"
  test_command: "cargo test -p yama-host-apple test_video_decoder"
  files_created:
    - "platform/apple/host/src/providers/gstreamer_decoder.rs"
  files_modified:
    - "platform/apple/host/src/providers/mod.rs"
  acceptance_criteria:
    - "Hardware decode via VideoToolbox"
    - "Frame extraction at correct timestamps"
```

---

## Task Workflow

### Creating Tasks from Roadmap

```
1. Read phase document (e.g., phase-1-core-infrastructure.md)
2. For each milestone:
   a. Create milestone-level task (e.g., "1.1")
   b. Create subtasks for each task item (e.g., "1.1.1", "1.1.2")
3. Set up dependencies (blockedBy/blocks)
```

### Working on Tasks

```
1. TaskList - Find available tasks (pending, no blocks)
2. TaskUpdate - Set status to in_progress
3. TaskGet - Read full description and criteria
4. Implement the task
5. Verify acceptance criteria
6. TaskUpdate - Set status to completed
7. TaskList - Check for newly unblocked tasks
```

### Example Session

```
> TaskList
#1.1.1 [pending] Define VideoDecoder trait
#1.1.2 [pending] Define IndexProvider trait (blocked by #1.1.1)
#1.1.3 [pending] Define BatchProcessor trait

> TaskUpdate #1.1.1 status=in_progress
Updated task #1.1.1 status

> TaskGet #1.1.1
Subject: Define VideoDecoder trait
Description: Create the VideoDecoder trait...
Status: in_progress

[Implement the task...]

> TaskUpdate #1.1.1 status=completed
Updated task #1.1.1 status

> TaskList
#1.1.1 [completed] Define VideoDecoder trait
#1.1.2 [pending] Define IndexProvider trait (now unblocked)
#1.1.3 [pending] Define BatchProcessor trait
```

---

## Task List Initialization

When starting a new phase, create tasks in bulk:

```
TaskCreate: "1.1.1" - Define VideoDecoder trait
TaskCreate: "1.1.2" - Define IndexProvider trait
TaskCreate: "1.1.3" - Define BatchProcessor trait
TaskCreate: "1.1.4" - Define EmbeddingModel trait

TaskUpdate: "1.1.2" addBlockedBy=["1.1.1"]
TaskUpdate: "1.1.3" addBlockedBy=["1.1.1"]
```

---

## Integration with Roadmap

Tasks correspond to roadmap items:

| Roadmap | Task ID | Task Subject |
|---------|---------|--------------|
| Phase 1 | 1 | Core Infrastructure |
| Milestone 1.1 | 1.1 | SQLite Schema & IndexProvider |
| Task 1.1.1 | 1.1.1 | Design schema |
| Task 1.1.2 | 1.1.2 | Create FTS5 tables |
