# Yama Agent Team Structure

## Overview

This document defines the agent team structure for developing Yama. Agents are specialized by domain and coordinate through the main orchestrator session.

---

## Team Structure

```
┌─────────────────────────────────────────────────────────────────────┐
│                     ORCHESTRATOR (Main Session)                     │
│                                                                     │
│  Responsibilities:                                                  │
│  • Roadmap progress tracking (TaskCreate/TaskUpdate)                │
│  • Cross-cutting changes to shared/platform-traits/                 │
│  • Integration testing                                              │
│  • Agent coordination and handoffs                                  │
│  • Final code review before commits                                 │
└─────────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│  APPLE AGENT  │      │ JETSON AGENT  │      │   WEB AGENT   │
│               │      │               │      │               │
│ platform/     │      │ platform/     │      │ web/          │
│   apple/      │      │   jetson/     │      │               │
│               │      │               │      │ SvelteKit     │
│ Phases 1-3    │      │ Phase 4       │      │ TypeScript    │
└───────────────┘      └───────────────┘      └───────────────┘
        │                       │                       │
        └───────────────────────┼───────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │   SUPPORT AGENTS      │
                    │                       │
                    │ • Explore (research)  │
                    │ • code-reviewer       │
                    │ • code-architect      │
                    └───────────────────────┘
```

---

## Agent Definitions

### 1. Orchestrator (Main Session)

**Not a subagent** - this is the primary Claude session the user interacts with.

**Scope:**
- `shared/platform-traits/` - All trait definitions
- `shared/protocol/` - Protobuf schemas
- `roadmap/` - Progress tracking
- `.claude/` - Agent configuration
- Cross-platform integration

**Responsibilities:**
- Parse user requests and delegate to appropriate agents
- Maintain task list (`TaskCreate`, `TaskUpdate`, `TaskList`)
- Review agent outputs before committing
- Handle changes that span multiple agents' domains
- Coordinate handoffs between agents

**When to stay in orchestrator:**
- Modifying trait definitions (affects all implementers)
- Protocol buffer changes
- Roadmap updates
- Multi-platform integration tests

---

### 2. Apple Platform Agent

**Invoke with:** `Task` tool, `subagent_type: "Explore"` or custom prompt

**Scope:**
```
platform/apple/
├── host/src/           # Main application
├── video/src/          # GStreamer + VideoToolbox
├── inference/src/      # Metal MPS inference
└── compositor/src/     # wgpu + egui
```

**Roadmap Coverage:**
- Phase 1: Core Infrastructure (Mac implementation)
- Phase 2: Indexing & Embeddings (Mac implementation)
- Phase 3: Agent System (Mac UI + backend)

**Skills Required:**
- Rust, async/await, Tokio
- GStreamer, VideoToolbox
- SQLite, rusqlite
- egui, wgpu
- Metal, MPS (inference)

**Handoff to Orchestrator when:**
- Need to modify `shared/platform-traits/`
- Need to update protobuf schemas
- Work complete, ready for review

---

### 3. Jetson Platform Agent

**Invoke with:** `Task` tool, `subagent_type: "Explore"` or custom prompt

**Scope:**
```
platform/jetson/
├── host/src/           # Headless server
├── ai-server/src/      # Triton + DeepStream
└── video/src/          # NVDEC + Vulkan
```

**Roadmap Coverage:**
- Phase 4: Jetson Thor Deployment (all milestones)

**Skills Required:**
- Rust, async/await, Tokio
- DeepStream 7.x, GStreamer
- Triton Inference Server, gRPC
- TensorRT, CUDA
- Vulkan, DMA-BUF
- WebRTC, Smithay

**Handoff to Orchestrator when:**
- Need to modify `shared/platform-traits/`
- Need remote provider abstractions
- Work complete, ready for review

---

### 4. Web Frontend Agent

**Invoke with:** `Task` tool, `subagent_type: "Explore"` or custom prompt

**Scope:**
```
web/
├── src/
│   ├── routes/         # SvelteKit pages
│   ├── components/     # UI components
│   └── lib/            # Utilities
└── static/
```

**Responsibilities:**
- SvelteKit frontend development
- WebSocket client for event bus
- WebRTC video player
- Style guide maintenance

**Skills Required:**
- TypeScript, Svelte/SvelteKit
- WebSocket, WebRTC
- CSS (Obsidian Lens design system)
- Protobuf-ts

**Handoff to Orchestrator when:**
- Need to modify protobuf schemas
- Need backend API changes
- Work complete, ready for review

---

### 5. Support Agents

#### Built-in Subagent Types

| Agent | Type | Use Case |
|-------|------|----------|
| Explorer | `Explore` | Codebase research, finding patterns |
| Reviewer | `feature-dev:code-reviewer` | Code review before merge |
| Architect | `feature-dev:code-architect` | Design new components |
| Cleanup | `cleanup-crew` | Refactoring, deduplication |

#### Project-Specific Agents (in `.claude/agents/`)

**Development Agents** (`development-agents/`):

| Agent | File | Purpose |
|-------|------|---------|
| Code Reviewer | `code-reviewer/SKILL.md` | Rust-specific review |
| Pipeline Debugger | `pipeline-debugger/SKILL.md` | GStreamer/DeepStream debugging |
| Video Monitor | `video-monitor/SKILL.md` | Video analysis assistance |
| Model Selector | `model-selector/SKILL.md` | ML model recommendations |
| Privacy Guardian | `privacy-guardian/SKILL.md` | Privacy/security review |

**Roadmap Agents** (`roadmap-agents/`):

| Agent | File | Purpose |
|-------|------|---------|
| Architecture Reviewer | `architecture-reviewer/SKILL.md` | Design review |
| NVIDIA SME | `nvidia-sme/SKILL.md` | Jetson/CUDA expertise |
| UX Reviewer | `ux-reviewer/SKILL.md` | UI/UX feedback |
| Agent Design Reviewer | `agent-design-reviewer/SKILL.md` | Agent architecture |

---

## Coordination Protocols

### Starting Work on a Milestone

```
1. Orchestrator: TaskUpdate milestone to in_progress
2. Orchestrator: Check if work is single-platform or cross-cutting
3. If single-platform:
   a. Spawn appropriate platform agent with milestone prompt
   b. Agent implements and reports back
   c. Orchestrator reviews and commits
4. If cross-cutting:
   a. Orchestrator implements shared trait changes
   b. Spawn platform agents in parallel for implementations
   c. Orchestrator integrates and commits
```

### Agent Handoff Protocol

When an agent needs to hand off to another:

```markdown
## Handoff: [Source Agent] → [Target Agent]

**Context:**
- What was accomplished
- Current state of the code
- Files modified

**Request:**
- What needs to be done next
- Specific requirements

**Blocking Issues:**
- Any problems encountered
- Decisions needed from user
```

### Parallel Work Pattern

For independent work (e.g., Apple + Web simultaneously):

```
Orchestrator spawns:
  ├── Apple Agent (background): "Implement SQLite IndexProvider"
  └── Web Agent (background): "Add project list component"

Orchestrator monitors via TaskOutput, then:
  └── Reviews both, resolves conflicts, commits
```

---

## Agent Invocation Examples

### Spawn Apple Agent for Milestone

```
Task tool:
  subagent_type: "Explore"
  prompt: |
    ## Apple Platform Agent Task

    **Milestone:** 1.3 - GStreamer VideoDecoder

    **Scope:** platform/apple/

    **Requirements:**
    1. Implement VideoDecoder trait using GStreamer + VideoToolbox
    2. Support: get_metadata, extract_keyframes, decode_frame, extract_audio

    **Acceptance Criteria:**
    - AC-1: cargo test -p yama-host-apple test_video_decoder passes
    - AC-2: Hardware decode via VideoToolbox (not software)
    - AC-3: Frame extraction at correct timestamps (±100ms)

    **Output:**
    - List files created/modified
    - Test results
    - Any deviations from plan
```

### Spawn Parallel Agents

```
Single message with multiple Task calls:

Task 1:
  subagent_type: "Explore"
  run_in_background: true
  prompt: "[Apple agent task...]"

Task 2:
  subagent_type: "Explore"
  run_in_background: true
  prompt: "[Web agent task...]"
```

### Code Review Before Commit

```
Task tool:
  subagent_type: "feature-dev:code-reviewer"
  prompt: |
    Review the following files for Phase 1 Milestone 1.3:
    - platform/apple/host/src/providers/gstreamer_decoder.rs
    - platform/apple/host/src/providers/mod.rs

    Focus on:
    - Correctness of GStreamer pipeline
    - Error handling
    - Memory management
    - Project conventions (anyhow, tracing)
```

---

## File Ownership

| Path | Owner | Notes |
|------|-------|-------|
| `shared/platform-traits/` | Orchestrator | Cross-cutting |
| `shared/protocol/` | Orchestrator | Cross-cutting |
| `shared/yama-theme/` | Orchestrator | Design tokens |
| `shared/container-sdk/` | Orchestrator | Shared SDK |
| `platform/apple/` | Apple Agent | Mac-specific |
| `platform/jetson/` | Jetson Agent | Jetson-specific |
| `web/` | Web Agent | Frontend |
| `containers/` | Orchestrator | Container services |
| `roadmap/` | Orchestrator | Planning |
| `.claude/` | Orchestrator | Agent config |
| `docs/` | Orchestrator | Documentation |

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-03 | Orchestrator owns shared traits | Changes affect all platforms |
| 2025-03 | Platform agents are Explore type | Need full codebase access |
| 2025-03 | Background agents for parallel work | Maximize throughput |
| 2025-03 | Code review before commit | Quality gate |
