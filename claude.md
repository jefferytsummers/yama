# Yama

Vision AI compositor with dual UI: native egui app for end users, web UI (Svelte) for system engineers.

## Quick Start

```bash
# Run host process (native UI + HTTP server + event bus)
cargo run -p yama-host-apple

# Run web frontend (dev mode, in separate terminal)
cd web && npm install && npm run dev
```

- Native UI: launches automatically
- Host UI: http://localhost:8080 (or :5173 in dev)
- Event Bus: ws://localhost:8765
- Config: `yama.toml`

## Key Paths

- `platform/apple/host/src/main.rs` - Entry point, native UI
- `platform/apple/host/src/http_server/` - REST API (Axum)
- `web/src/` - Svelte frontend
- `yama.toml` - Runtime config

See `docs/architecture.md` for detailed architecture, API reference, and troubleshooting.
