---
name: container-dev
description: Containerized service development with EventBusClient and VideoFrameSender
compatibility:
  - containers/
  - shared/container-sdk/
---

# Container Service Development

Building containerized services that integrate with the Yama host.

## Architecture

```
┌─────────────────────────────────────────┐
│  Yama Host Process                      │
├─────────────────────────────────────────┤
│  Orchestrator                           │
│  (Docker container management)          │
└────────────────┬────────────────────────┘
                 │ Unix Socket IPC
    ┌────────────┴────────────┐
    ▼                         ▼
┌──────────┐            ┌──────────┐
│ Video    │            │ Agent    │
│ Container│            │ Container│
└──────────┘            └──────────┘
```

## Container SDK

Location: `shared/container-sdk/`

### Key Exports

```rust
pub use client::EventBusClient;
pub use health::HealthReporter;
pub use video::{VideoFrameReceiver, VideoFrameSender};
```

## EventBusClient

Primary communication interface for containers:

```rust
use yama_container_sdk::EventBusClient;

// Connect via Unix socket (inside container)
let client = EventBusClient::connect("/tmp/yama-event.sock").await?;

// Subscribe to topics
client.subscribe(&["agent.request", "video.frame.*"]).await?;

// Publish messages
client.publish(envelope).await?;

// Receive messages
while let Some(envelope) = client.recv().await {
    handle_message(envelope);
}
```

## HealthReporter

Automatic health check responses:

```rust
use yama_container_sdk::HealthReporter;

let client = EventBusClient::connect("/tmp/yama-event.sock").await?;
let health = HealthReporter::new(&client, "my-service");
health.start().await?;

// Responds to system.health.request automatically
// Reports status: healthy, degraded, unhealthy
```

## VideoFrameSender

Zero-copy video frame publishing:

```rust
use yama_container_sdk::VideoFrameSender;

let sender = VideoFrameSender::new(&client, "video-source-1");

// Send decoded frame (DMA-BUF on Linux)
sender.send_frame(&decoded_frame).await?;

// Topic: video.frame.{source_id}
```

## VideoFrameReceiver

Subscribe to video frames:

```rust
use yama_container_sdk::VideoFrameReceiver;

let receiver = VideoFrameReceiver::new(&client, &["camera-1", "camera-2"]);

while let Some(frame) = receiver.recv().await {
    process_frame(frame);
}
```

## Container Template

```rust
// containers/my-service/src/main.rs

use anyhow::Result;
use tracing::info;
use yama_container_sdk::{EventBusClient, HealthReporter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::init();

    info!("Starting my-service");

    // Connect to event bus
    let client = EventBusClient::connect("/tmp/yama-event.sock").await?;

    // Start health reporter
    let health = HealthReporter::new(&client, "my-service");
    health.start().await?;

    // Subscribe to relevant topics
    client.subscribe(&["my.topic.*"]).await?;

    // Main loop
    while let Some(envelope) = client.recv().await {
        if let Err(e) = handle_message(&envelope).await {
            tracing::error!("Error handling message: {}", e);
        }
    }

    Ok(())
}

async fn handle_message(envelope: &Envelope) -> Result<()> {
    match envelope.topic.as_str() {
        "my.topic.request" => { /* handle */ }
        _ => {}
    }
    Ok(())
}
```

## Dockerfile Template

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

RUN cargo build --release -p my-service

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/my-service /usr/local/bin/

CMD ["my-service"]
```

## Existing Containers

| Container | Purpose | Topics |
|-----------|---------|--------|
| containers/agent | LLM agent runtime | agent.* |
| containers/tools/code-executor | Code execution | tools.code.* |
| containers/tools/file-system | File operations | tools.fs.* |
| containers/tools/browser | Web browsing | tools.browser.* |

## Key Files

- `shared/container-sdk/src/lib.rs` - SDK entry point
- `shared/container-sdk/src/client.rs` - EventBusClient
- `shared/container-sdk/src/health.rs` - HealthReporter
- `shared/container-sdk/src/video.rs` - Video frame utilities
- `containers/agent/src/main.rs` - Agent container example

## DMA-BUF (Linux/Jetson)

For zero-copy video frame sharing:

```rust
// In video container
let dma_buf_fd = get_frame_fd()?;  // From hardware decoder

// Send via event bus (fd passing over Unix socket)
sender.send_dma_buf_frame(dma_buf_fd, width, height, format).await?;

// In receiver container
let frame = receiver.recv().await?;
if let FrameData::DmaBuf { fd, offset, stride, modifier } = frame.data {
    // Import into GPU (Vulkan)
}
```

## Configuration

Containers read config from environment:

```bash
YAMA_EVENT_SOCKET=/tmp/yama-event.sock
YAMA_SERVICE_ID=my-service
YAMA_LOG_LEVEL=info
```

## Orchestrator Integration

The host orchestrator manages container lifecycle:

```rust
// platform/*/host/src/orchestrator/mod.rs

pub struct Orchestrator {
    runtime: DockerRuntime,
    services: HashMap<String, ServiceInfo>,
}

impl Orchestrator {
    pub async fn start_service(&self, id: &str) -> Result<()>;
    pub async fn stop_service(&self, id: &str) -> Result<()>;
    pub async fn health_check(&self, id: &str) -> Result<ServiceHealth>;
}
```
