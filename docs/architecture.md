# Yama Architecture

## System Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│  Yama Host Process (Tokio Runtime)                              │
├─────────────────────────────────────────────────────────────────┤
│  Event Bus (WS: 8765)  │  HTTP Server (8080)  │  Orchestrator   │
└─────────────────────────────────────────────────────────────────┘
            │                       │
            ▼                       ▼
┌───────────────────────┐  ┌────────────────────────────┐
│  Application UI       │  │  Host UI (Web/Svelte)      │
│  (Native/egui)        │  │  http://localhost:8080     │
├───────────────────────┤  ├────────────────────────────┤
│  END USER VIEW        │  │  SYSTEM ENGINEER VIEW      │
│  • Camera feeds       │  │  • Service management      │
│  • AI detection       │  │  • System metrics          │
│  • Agent chat         │  │  • Configuration           │
│  • Simple UX          │  │  • Logs and debugging      │
└───────────────────────┘  └────────────────────────────┘
```

## Key Concepts

- **Application UI**: Native egui window for end users. Shows camera feeds, AI detections, and agent chat. No technical details exposed.

- **Host UI**: Web-based admin panel for system engineers. Service management, metrics, configuration, and logs.

- **Event Bus**: Binary Protobuf pub/sub over WebSocket (port 8765) and Unix socket. Used for inter-service communication.

- **Orchestrator**: Manages Docker containers for video, inference, and agent services. Handles health checks and auto-restart.

## Project Structure

```
yama/
├── platform/apple/host/src/
│   ├── main.rs              # Entry point, native UI
│   ├── http_server/         # Axum REST API
│   │   ├── mod.rs           # Server setup
│   │   ├── routes.rs        # Route definitions
│   │   └── handlers.rs      # Request handlers
│   ├── event_bus/           # WebSocket pub/sub
│   ├── orchestrator/        # Docker container management
│   └── compositor/          # Metal rendering
├── web/                     # Svelte frontend (Host UI)
│   ├── src/
│   │   ├── routes/          # SvelteKit pages
│   │   ├── components/      # UI components
│   │   └── lib/             # API client, stores
│   └── package.json
├── shared/                  # Shared crates
│   ├── protocol/            # Protobuf definitions
│   └── platform-traits/     # Cross-platform interfaces
└── yama.toml               # Runtime configuration
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Health check |
| `/api/services` | GET | List all services |
| `/api/services/{id}/start` | POST | Start a service |
| `/api/services/{id}/stop` | POST | Stop a service |
| `/api/config` | GET | Get configuration |
| `/api/metrics` | GET | System metrics |
| `/api/video-sources` | GET | List video sources |

## Configuration

Edit `yama.toml` in the repository root:

```toml
[http_server]
bind = "0.0.0.0"
port = 8080
static_path = "web/build"
cors_enabled = true

[event_bus]
websocket_bind = "0.0.0.0:8765"
unix_socket = "/tmp/yama-event.sock"

[orchestrator]
runtime = "docker"
health_interval = 5
```

## Troubleshooting

### Docker not connected
```bash
docker info  # Verify Docker is running
```

### Port already in use
```bash
lsof -i :8080  # Check what's using the port
lsof -i :8765
```

### Svelte build errors
```bash
cd web
rm -rf node_modules .svelte-kit
npm install
```
