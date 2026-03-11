---
name: api-endpoint
description: Implement an HTTP API endpoint with proper error handling, validation, and tests. Use for Phase 4 API layer work.
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---

# API Endpoint Implementation

Implement the `$ARGUMENTS` endpoint following Yama API conventions.

## Implementation Steps

### 1. Create Handler
Location: `platform/apple/host/src/http_server/{module}_handlers.rs`

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::ServiceState;

#[derive(Debug, Serialize)]
pub struct ResponseType {
    // Define response fields
}

pub async fn handler_name(
    State(state): State<Arc<ServiceState>>,
    // Add extractors as needed
) -> impl IntoResponse {
    // Implementation
}
```

### 2. For SSE Streaming Endpoints

```rust
use axum::response::sse::{Event, Sse};
use futures_util::stream::Stream;
use std::convert::Infallible;
use tokio_stream::StreamExt;

pub async fn stream_handler(
    State(state): State<Arc<ServiceState>>,
    Path(id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = state.updates.subscribe()
        .filter(|update| update.id == id)
        .map(|update| Ok(Event::default()
            .event("update")
            .json_data(&update)
            .unwrap()));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
    )
}
```

### 3. Register Route
In `platform/apple/host/src/http_server/mod.rs`:

```rust
.route("/api/path", get(handler_name))
.route("/api/path/:id/stream", get(stream_handler))
```

### 4. Add Tests
Location: Same file or `tests/` directory

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_handler() {
        // Setup test state
        // Build request
        // Assert response
    }
}
```

## Checklist

- [ ] Handler returns proper status codes (200, 400, 404, 500)
- [ ] Input validation with clear error messages
- [ ] Logging with tracing (info for success, error for failures)
- [ ] SSE heartbeat if streaming (15s interval)
- [ ] Route registered in mod.rs
- [ ] Tests cover happy path and error cases
- [ ] Response types derive Serialize

## Reference Files

- Existing handlers: `platform/apple/host/src/http_server/inference_handlers.rs`
- Service state: `platform/apple/host/src/http_server/mod.rs`
- Protocol types: `shared/protocol/src/`
