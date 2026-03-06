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

## Deep Reference

- Brand & design system: `docs/brand/BRAND.md`
- Design tokens: `docs/brand/design-tokens.json`
- Platform trait APIs: `docs/agent-guides/platform-traits.md`
- Protocol messages: `docs/agent-guides/protocol-messages.md`
- Apple specifics: `docs/agent-guides/apple-platform.md`
- Jetson specifics: `docs/agent-guides/jetson-platform.md`
- Web frontend: `docs/agent-guides/web-frontend.md`

## Platform Overrides

Each platform directory has its own CLAUDE.md with non-negotiable constraints:
- `platform/apple/CLAUDE.md` - Metal, VideoToolbox, IOSurface requirements
- `platform/jetson/CLAUDE.md` - Vulkan, NVDEC, DMA-BUF, Smithay requirements
- `web/CLAUDE.md` - SvelteKit, TypeScript, WebSocket requirements
