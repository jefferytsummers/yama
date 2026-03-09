# Phase 0: UX Discovery

**Duration:** 5-7 days
**Goal:** Validate UX patterns for Content Analyst workflow before committing to backend implementation.

---

## Overview

The existing chat UI was designed for real-time VLM conversation. Content Analyst needs fundamentally different interaction patterns:

- **Batch import** - Drag folder, scan feedback, duplicate detection
- **Progress dashboard** - Multi-stage indexing with pause/resume
- **Search results grid** - Thumbnails with selection mechanics
- **Clip preview** - Video preview panel with export actions
- **Model management** - Download, storage, and first-run wizard

This phase validates these patterns through wireframes and egui prototypes before investing in backend infrastructure.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  CONTENT ANALYST WORKFLOW                                               │
│                                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐│
│  │ 1.IMPORT │→ │ 2.INDEX  │→ │ 3.QUERY  │→ │ 4.REVIEW │→ │ 5.EXTRACT││
│  │          │  │          │  │          │  │          │  │          ││
│  │ Drag     │  │ Progress │  │ Search   │  │ Results  │  │ Clip     ││
│  │ folder   │  │ dashboard│  │ bar +    │  │ grid +   │  │ config + ││
│  │          │  │ + ETA    │  │ filters  │  │ preview  │  │ export   ││
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘│
│                                                              │         │
│                                                              ▼         │
│                                                         ┌──────────┐  │
│                                                         │ 6.REPORT │  │
│                                                         │ Template │  │
│                                                         │ + export │  │
│                                                         └──────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone 0.1: Workflow Mapping

**Duration:** 1 day

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 0.1.1 | Import flow diagram | Drag folder → scan → duplicate detection → confirm | All states documented |
| 0.1.2 | Indexing flow diagram | Multi-stage progress → pause/resume → error recovery | Stage transitions clear |
| 0.1.3 | Search flow diagram | Query input → filter → mode switch → results | Search modes documented |
| 0.1.4 | Review flow diagram | Grid view → preview → selection → starring | Interaction patterns clear |
| 0.1.5 | Extract flow diagram | Clip config → destination → batch progress | Export options documented |
| 0.1.6 | Report flow diagram | Template selection → preview → format export | Report types documented |

### Deliverables

User flow diagrams (Mermaid or draw.io) covering:

1. **IMPORT** - Drag folder, scan feedback, duplicate detection
2. **INDEX** - Multi-stage progress, pause/resume, error recovery
3. **QUERY** - Search bar, filters, mode switching
4. **REVIEW** - Results grid, preview panel, selection mechanics
5. **EXTRACT** - Clip config, destination, batch progress
6. **REPORT** - Template selection, preview, format options

---

## Milestone 0.2: Wireframe Exploration

**Duration:** 2-3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 0.2.1 | Library view wireframe | Main video grid with thumbnails | Layout proportions work |
| 0.2.2 | Import overlay wireframe | Progress during initial import | Clear progress indication |
| 0.2.3 | Search results wireframe | Grid with thumbnails + timestamps | Scannable results |
| 0.2.4 | Preview panel wireframe | Video player + metadata sidebar | Ergonomic for review |
| 0.2.5 | Clip extraction wireframe | Dialog with padding/output config | Clear export options |
| 0.2.6 | Model settings wireframe | Download manager + storage view | Model management clear |
| 0.2.7 | First-run wireframe | Model selection wizard | Onboarding is smooth |

### Key Screens

#### 1. Main Library View
```
┌─────────────────────────────────────────────────────────────────────────┐
│  ◆ Yama                              [Search...          ] [⚙]        │
├─────────────────────────────────────────────────────────────────────────┤
│  Library (47 videos)     ⊕ Import                                      │
│                                                                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐     │
│  │         │  │         │  │         │  │         │  │         │     │
│  │  thumb  │  │  thumb  │  │  thumb  │  │  thumb  │  │  thumb  │     │
│  │         │  │         │  │         │  │         │  │         │     │
│  ├─────────┤  ├─────────┤  ├─────────┤  ├─────────┤  ├─────────┤     │
│  │ name    │  │ name    │  │ name    │  │ name    │  │ name    │     │
│  │ 12:34   │  │ 5:21    │  │ 45:00   │  │ 8:15    │  │ 22:10   │     │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────┘     │
│                                                                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐     │
│  │  ...    │  │  ...    │  │  ...    │  │  ...    │  │  ...    │     │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 2. Import/Indexing Progress
```
┌─────────────────────────────────────────────────────────────────────────┐
│  ◆ Indexing Videos                                         [Pause] [×] │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Stage 2 of 4: Extracting Keyframes                                    │
│                                                                         │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░  47/100              │
│                                                                         │
│  Current: meeting_q4_planning.mp4                                       │
│  ETA: ~12 minutes remaining                                             │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ ✓ Scanning files (100/100)                                      │   │
│  │ → Extracting keyframes (47/100)                                 │   │
│  │ ○ Generating embeddings                                         │   │
│  │ ○ Transcribing audio                                            │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Errors: 2 videos skipped (unsupported codec)           [View Details] │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 3. Search Results
```
┌─────────────────────────────────────────────────────────────────────────┐
│  ◆ Yama              [person saying "quarterly results" with chart  ] │
├─────────────────────────────────────────────────────────────────────────┤
│  23 results for "quarterly results" + chart                [Clear]    │
│  Mode: [Transcript] [Visual] [Both✓]   Filter: [Last week ▼]         │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ ☐ │ [thumb] │ meeting_q4.mp4 @ 12:34                  │ 0.92 │   │
│  │   │         │ "...quarterly results exceeded our projections..." │   │
│  ├───┼─────────┼─────────────────────────────────────────┼──────┤   │
│  │ ☐ │ [thumb] │ earnings_call.mp4 @ 45:12               │ 0.87 │   │
│  │   │         │ "...results for Q4 show significant growth..."    │   │
│  ├───┼─────────┼─────────────────────────────────────────┼──────┤   │
│  │ ☐ │ [thumb] │ board_meeting.mp4 @ 03:22               │ 0.81 │   │
│  │   │         │ (chart visible, no transcript match)              │   │
│  └───┴─────────┴─────────────────────────────────────────┴──────┘   │
│                                                                         │
│  [Extract Selected (0)]                                                │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 4. Video Preview Panel
```
┌─────────────────────────────────────────────────────────────────────────┐
│  ◆ Yama                                                                │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────┐  ┌─────────────────────┐│
│  │                                           │  │ meeting_q4.mp4      ││
│  │                                           │  │                     ││
│  │              VIDEO PREVIEW                │  │ Duration: 45:00     ││
│  │                                           │  │ Resolution: 1080p   ││
│  │                                           │  │ Indexed: 2 days ago ││
│  │                                           │  │                     ││
│  ├───────────────────────────────────────────┤  │ Match @ 12:34       ││
│  │  ◄◄  ▶  ►►  │▓▓▓▓▓▓░░░░░░░░░░│ 12:34/45:00│  │ "quarterly results" ││
│  └───────────────────────────────────────────┘  │                     ││
│                                                  │ [★ Star] [Extract] ││
│  Transcript:                                     └─────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 12:30 - And now for the quarterly results...                    │   │
│  │ 12:34 - Our quarterly results exceeded projections by 15%       │   │
│  │ 12:40 - As you can see in this chart...                         │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 5. Clip Extraction Dialog
```
┌─────────────────────────────────────────────────────────────────────────┐
│  Extract Clip                                                      [×] │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Source: meeting_q4.mp4                                                 │
│  Match: 12:34 - "quarterly results exceeded..."                        │
│                                                                         │
│  Clip Settings:                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ Start:    [12:30    ] (-4s padding)                             │   │
│  │ End:      [12:45    ] (+11s padding)                            │   │
│  │ Duration: 15 seconds                                            │   │
│  │                                                                 │   │
│  │ Padding:  [5 seconds ▼] before/after match                      │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Output:                                                                │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ Format:   [MP4 (H.264) ▼]                                       │   │
│  │ Location: ~/Desktop/clips/                           [Browse]   │   │
│  │ Filename: meeting_q4_clip_12-34.mp4                             │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│                                           [Cancel]  [Extract Clip]     │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 6. Model Management Settings
```
┌─────────────────────────────────────────────────────────────────────────┐
│  Settings > Models                                                 [×] │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Installed Models                                      Storage: 5.2 GB │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ ✓ CLIP ViT-B/32          350 MB    Used for visual search       │   │
│  │                                                        [Delete] │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │ ✓ Whisper base.en        400 MB    Transcription                │   │
│  │                                                        [Delete] │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │ ✓ LLaVA 7B Q4           4.5 GB    Visual analysis               │   │
│  │                                                        [Delete] │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Available Models                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ ○ Whisper large-v3      1.5 GB    Better accuracy     [Install] │   │
│  │ ○ Qwen2-VL 7B Q4        5.0 GB    Better VLM          [Install] │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Models stored in: ~/.cache/yama/models/                               │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 7. First-Run Experience
```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│                         ◆ Welcome to Yama                               │
│                                                                         │
│       Video analysis for researchers, journalists, and analysts         │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Required Models (~750 MB)                                              │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ ☑ CLIP ViT-B/32         350 MB    Visual search                 │   │
│  │ ☑ Whisper base.en       400 MB    Transcription                 │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Optional Models                                                        │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ ☐ LLaVA 7B Q4          4.5 GB    Visual analysis (Recommended) │   │
│  │ ☐ Whisper large-v3     1.5 GB    Better transcription accuracy │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Total download: 750 MB                                                 │
│                                                                         │
│                              [Download and Continue]                    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Approach

**egui throwaway prototype** using existing `yama-theme` components:
- Validates technical feasibility of layouts
- Tests real rendering performance
- Identifies missing components early
- Can be discarded after validation

---

## Milestone 0.3: Component Inventory

**Duration:** 1 day

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 0.3.1 | Audit existing components | Review yama-theme against UX needs | Gap list complete |
| 0.3.2 | Document search results component | Grid with thumbnails, selection, scores | Spec written |
| 0.3.3 | Document multi-stage progress | Stages, ETA, pause/resume | Spec written |
| 0.3.4 | Document model download UI | Progress, speed, cancel | Spec written |
| 0.3.5 | Document file drop zone | Drag feedback, validation | Spec written |

### Component Gap Analysis

| UX Need | Existing Component | Gap |
|---------|-------------------|-----|
| Video thumbnail grid | `VideoGrid` | Needs selection state, checkbox overlay |
| Status indicators | `StatusIndicator` | ✓ Sufficient |
| Progress bar | `ProgressBar` | Needs multi-stage variant with ETA |
| Cards | `Card` | ✓ Sufficient |
| Buttons | `primary_button`, `secondary_button` | ✓ Sufficient |
| Badges | `Badge` | ✓ Sufficient |
| Detection overlay | `DetectionOverlay` | ✓ Sufficient for preview |
| Alert timeline | `AlertTimeline` | Not needed for Content Analyst |
| **Search results list** | None | **NEW**: Results grid with thumbnails |
| **Multi-stage progress** | None | **NEW**: Progress with stages + ETA |
| **Model download UI** | None | **NEW**: Download progress with speed |
| **File drop zone** | None | **NEW**: Drag-drop target with feedback |

### New Component Specifications

#### SearchResults Component

```rust
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    pub selected: HashSet<usize>,
    pub on_select: Option<Box<dyn Fn(usize, bool)>>,
    pub on_preview: Option<Box<dyn Fn(usize)>>,
}

pub struct SearchResultItem {
    pub thumbnail: Option<egui::TextureHandle>,
    pub video_name: String,
    pub timestamp_ms: u64,
    pub score: f32,
    pub snippet: String,
    pub match_type: MatchType,
}
```

#### MultiStageProgress Component

```rust
pub struct MultiStageProgress {
    pub stages: Vec<ProgressStage>,
    pub current_stage: usize,
    pub current_item: String,
    pub eta_seconds: Option<u64>,
    pub can_pause: bool,
    pub paused: bool,
}

pub struct ProgressStage {
    pub name: String,
    pub completed: u32,
    pub total: u32,
    pub status: StageStatus,
}

pub enum StageStatus {
    Pending,
    InProgress,
    Complete,
    Error(String),
}
```

#### ModelDownload Component

```rust
pub struct ModelDownload {
    pub model_name: String,
    pub model_size_bytes: u64,
    pub downloaded_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub status: DownloadStatus,
}

pub enum DownloadStatus {
    Pending,
    Downloading,
    Verifying,
    Complete,
    Error(String),
}
```

#### DropZone Component

```rust
pub struct DropZone {
    pub accepted_extensions: Vec<String>,
    pub hover_state: bool,
    pub on_drop: Option<Box<dyn Fn(Vec<PathBuf>)>>,
}
```

---

## Milestone 0.4: UX Validation

**Duration:** 1-2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 0.4.1 | Walk through user stories | Trace persona journeys | All stories addressed |
| 0.4.2 | Validate import flow | User story: "I can import a folder" | Wireframe covers it |
| 0.4.3 | Validate search flow | User story: "I can find footage" | Wireframe covers it |
| 0.4.4 | Validate export flow | User story: "I can extract clips" | Wireframe covers it |
| 0.4.5 | Document decisions | Record UX choices with rationale | Decisions documented |

### User Stories Validation

From `personas/content-analyst.md`:

| User Story | Phase | UX Element | Validated |
|------------|-------|------------|-----------|
| Import folder of videos | 0 | Drop zone + progress overlay | ☐ |
| See indexing progress | 0 | Multi-stage progress component | ☐ |
| Search by spoken words | 5 | Search bar with mode tabs | ☐ |
| Search by visual content | 5 | Search bar with visual mode | ☐ |
| Preview matching moments | 5 | Video preview panel | ☐ |
| Extract specific clips | 6 | Extraction dialog | ☐ |
| Generate summary reports | 8 | Report template UI | ☐ |
| Manage storage/models | 4 | Model settings view | ☐ |

---

## Phase 0.5: GStreamer Validation Spike (Parallel)

**Duration:** 2 days (runs parallel with 0.2-0.3)

### Goal

Validate whether CPU-buffer output is sufficient for batch processing.

### Current State

- GStreamer 0.23 fully integrated
- VideoToolbox decoder outputs CPU buffers (not IOSurface)
- This is not zero-copy, but may be acceptable for batch indexing

### Day 1: Benchmark

```bash
# Test configuration
VIDEO_PATH=~/Videos/test_1080p.mp4
DURATION=60  # 1 minute

# Metrics to collect:
# - Decode time (wall clock)
# - Peak memory usage
# - CPU utilization
# - GPU utilization (Activity Monitor)

# Test 1: Single video decode
cargo run --example decode_benchmark -- $VIDEO_PATH

# Test 2: Batch decode (10 videos)
cargo run --example batch_decode_benchmark -- ~/Videos/

# Profile with Instruments
xcrun xctrace record --template 'Time Profiler' --output decode_trace.trace \
  --launch -- ./target/release/yama-host-apple --benchmark $VIDEO_PATH
```

### Day 2: Decision Matrix

| Result | Decision |
|--------|----------|
| <3x realtime decode | **ACCEPT** - CPU buffers sufficient, proceed to Phase 1 |
| ≥3x realtime decode | **OPTIMIZE NOW** - Implement IOSurface/Metal path before Phase 1 |

If optimization needed, add 3-5 days:
1. IOSurface output from GStreamer appsink
2. Metal texture import from IOSurface
3. Zero-copy frame extraction

### Deliverable

Technical spike report with:
- Benchmark data (decode time, memory, CPU/GPU usage)
- Go/no-go decision
- If no-go: IOSurface implementation plan

---

## Dependencies

This phase has no dependencies on other phases.

## Blocks

- Phase 1 (Foundation) - needs UX validation complete
- All phases - component gap analysis informs implementation

---

## Exit Criteria

- [ ] User flow diagrams for all 6 workflows
- [ ] egui prototype with 7 key screens (throwaway)
- [ ] Component gap analysis with specs for new components
- [ ] UX validated against persona user stories
- [ ] GStreamer spike complete with go/no-go decision

---

## Checklist

### Milestone 0.1: Workflow Mapping
- [ ] 0.1.1 Import flow diagram
- [ ] 0.1.2 Indexing flow diagram
- [ ] 0.1.3 Search flow diagram
- [ ] 0.1.4 Review flow diagram
- [ ] 0.1.5 Extract flow diagram
- [ ] 0.1.6 Report flow diagram

### Milestone 0.2: Wireframe Exploration
- [ ] 0.2.1 Library view wireframe
- [ ] 0.2.2 Import overlay wireframe
- [ ] 0.2.3 Search results wireframe
- [ ] 0.2.4 Preview panel wireframe
- [ ] 0.2.5 Clip extraction wireframe
- [ ] 0.2.6 Model settings wireframe
- [ ] 0.2.7 First-run wireframe

### Milestone 0.3: Component Inventory
- [ ] 0.3.1 Audit existing components
- [ ] 0.3.2 Document search results component
- [ ] 0.3.3 Document multi-stage progress
- [ ] 0.3.4 Document model download UI
- [ ] 0.3.5 Document file drop zone

### Milestone 0.4: UX Validation
- [ ] 0.4.1 Walk through user stories
- [ ] 0.4.2 Validate import flow
- [ ] 0.4.3 Validate search flow
- [ ] 0.4.4 Validate export flow
- [ ] 0.4.5 Document decisions

### Phase 0.5: GStreamer Spike
- [ ] 0.5.1 Benchmark single video decode
- [ ] 0.5.2 Benchmark batch decode (10 videos)
- [ ] 0.5.3 Profile with Instruments
- [ ] 0.5.4 Document go/no-go decision
