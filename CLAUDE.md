# Yama

Vision AI compositor: native egui app (end users) + web UI (system engineers).

## Architecture

```
platform/apple/   Metal, VideoToolbox, MPS - macOS/iOS targets
platform/jetson/  Vulkan, NVDEC, CUDA, Smithay - Jetson Orin targets
shared/           Cross-platform traits, protocol, SDKs
web/              SvelteKit frontend for Host UI
containers/       Containerized services (agent, tools)
```

## Standards

- Rust 2024 edition, `clippy::pedantic`
- `async-trait` for async trait methods
- `anyhow::Result` for application errors, `thiserror` for library errors
- `tracing` for logging (not `log` or `println!`)
- Protocol Buffers for IPC, JSON for REST API

## Build & Test

```bash
cargo run -p yama-host-apple    # Run Apple host
cargo run -p yama-host-jetson   # Run Jetson host
cargo test --workspace          # Run all tests
cd web && npm run dev           # Web frontend dev server
```

## Key Paths

- Entry: `platform/{apple,jetson}/host/src/main.rs`
- Config: `yama.toml`
- Traits: `shared/platform-traits/src/`
- Protocol: `shared/protocol/src/`

## Services

- Native UI: egui window (launches automatically)
- Host UI: http://localhost:8080 (web admin panel)
- Event Bus: ws://localhost:8765 (Protobuf pub/sub)
- Unix Socket: /tmp/yama-event.sock (container IPC)

## Current Phase: 4 (API Layer)

Active plan: `.claude/plans/crystalline-jumping-muffin.md`

### Phase 4 Skills (API Development)
- `/api-endpoint` - Implement HTTP endpoints with SSE
- `/streaming-patterns` - SSE/WebSocket reference

### Phase 5 Skills (UI Development)
- `/sveltekit-component` - Create SvelteKit components
- `/frontend-design` - Design system reference

### Review Agents
- `api-reviewer` - Review API implementations
- `component-reviewer` - Review SvelteKit components
- `test-validator` - Validate test coverage

## Progressive Disclosure

For frontend design/components:
@.claude/skills/frontend-design/SKILL.md

For GStreamer video pipelines:
@.claude/skills/gstreamer/SKILL.md

For compositor/rendering:
@.claude/skills/compositor/SKILL.md

For ML inference:
@.claude/skills/inference/SKILL.md

For event bus IPC:
@.claude/skills/event-bus/SKILL.md

For container services:
@.claude/skills/container-dev/SKILL.md

For video analysis skill:
@.claude/skills/video-analysis/SKILL.md

For code review skill:
@.claude/skills/code-review/SKILL.md

For API endpoint patterns:
@.claude/skills/api-endpoint/SKILL.md

For SSE streaming patterns:
@.claude/skills/streaming-patterns/SKILL.md

For SvelteKit components:
@.claude/skills/sveltekit-component/SKILL.md

## Deep Reference

- Brand & design system: `docs/brand/BRAND.md`
- Design tokens: `docs/brand/design-tokens.json`
- Platform trait APIs: `docs/agent-guides/platform-traits.md`
- Protocol messages: `docs/agent-guides/protocol-messages.md`
- Apple specifics: `docs/agent-guides/apple-platform.md`
- Jetson specifics: `docs/agent-guides/jetson-platform.md`
- Web frontend: `docs/agent-guides/web-frontend.md`

## Worktree Isolation

This repo uses git worktrees for parallel feature development. **Each Claude session should operate within a single worktree.**

| Worktree | Branch | Purpose |
|----------|--------|---------|
| `yama/` | `dev` | Main development (source of truth) |
| `yama-components/` | `feature/unified-ui` | UI component development |
| `yama-tool-prompts/` | `feature/tool-driven-prompts` | Tool/prompt experiments |
| `yama-vlm-tuning/` | `vlm-tuning` | VLM model tuning |

**Rules:**
- One Claude session per worktree - never operate across worktrees
- Check `git worktree list` at session start to confirm location
- `dev` branch is the integration target - feature branches merge here
- If work spans worktrees, coordinate via explicit merge/rebase

**Before starting work:**
```bash
pwd                    # Confirm you're in the right worktree
git branch --show-current  # Confirm expected branch
git status             # Check for uncommitted changes
```

## Platform Overrides

Each platform directory has its own CLAUDE.md with non-negotiable constraints:
- `platform/apple/CLAUDE.md` - Metal, VideoToolbox, IOSurface requirements
- `platform/jetson/CLAUDE.md` - Vulkan, NVDEC, DMA-BUF, Smithay requirements
- `web/CLAUDE.md` - SvelteKit, TypeScript, WebSocket requirements
