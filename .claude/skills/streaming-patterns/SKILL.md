---
name: streaming-patterns
description: Reference for SSE and WebSocket streaming patterns in Yama. Read-only knowledge base.
allowed-tools: Read, Glob, Grep
---

# Streaming Patterns Reference

## Server-Sent Events (SSE)

### Rust Backend (axum)

**Basic SSE endpoint:**
```rust
use axum::response::sse::{Event, Sse};
use futures_util::stream::Stream;
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt;

pub async fn events(
    State(state): State<Arc<ServiceState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = tokio_stream::wrappers::BroadcastStream::new(state.events.subscribe())
        .filter_map(|result| result.ok())
        .map(|event| Ok(Event::default()
            .event(&event.event_type)
            .json_data(&event.data)
            .unwrap()
            .id(event.id.to_string())));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping")
    )
}
```

**Broadcast channel setup:**
```rust
use tokio::sync::broadcast;

pub struct ServiceState {
    pub job_events: broadcast::Sender<JobEvent>,
    pub chat_events: broadcast::Sender<ChatChunk>,
}

impl ServiceState {
    pub fn new() -> Self {
        let (job_tx, _) = broadcast::channel(100);
        let (chat_tx, _) = broadcast::channel(100);
        Self {
            job_events: job_tx,
            chat_events: chat_tx,
        }
    }
}
```

**Emitting events:**
```rust
// From anywhere with access to state
let _ = state.job_events.send(JobEvent {
    job_id: job_id.clone(),
    event_type: "frame".to_string(),
    data: serde_json::to_value(&frame_result).unwrap(),
});
```

### SvelteKit Frontend

**Basic SSE consumer:**
```typescript
function subscribeToJob(jobId: string): () => void {
  const source = new EventSource(`/api/inference/jobs/${jobId}/stream`);

  source.addEventListener('frame', (e) => {
    const data = JSON.parse(e.data);
    handleFrame(data);
  });

  source.addEventListener('complete', (e) => {
    const data = JSON.parse(e.data);
    handleComplete(data);
    source.close();
  });

  source.addEventListener('error', () => {
    // Reconnect with exponential backoff
    setTimeout(() => subscribeToJob(jobId), 1000);
  });

  // Return cleanup function
  return () => source.close();
}
```

**Svelte 5 reactive integration:**
```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  let { jobId } = $props<{ jobId: string }>();

  let frames = $state<Frame[]>([]);
  let status = $state<'pending' | 'running' | 'complete' | 'error'>('pending');
  let cleanup: (() => void) | null = null;

  onMount(() => {
    cleanup = subscribeToJob(jobId, {
      onFrame: (frame) => { frames = [...frames, frame]; },
      onComplete: () => { status = 'complete'; },
      onError: () => { status = 'error'; }
    });
  });

  onDestroy(() => cleanup?.());
</script>
```

## Chat Streaming Pattern

### Backend (POST with SSE response)

```rust
pub async fn send_message(
    State(state): State<Arc<ServiceState>>,
    Path(session_id): Path<String>,
    Json(request): Json<SendMessageRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(32);

    // Spawn message processing
    tokio::spawn(async move {
        process_message(tx, session_id, request.content).await;
    });

    // Convert mpsc to SSE stream
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(|chunk| match chunk {
            ChatChunk::Text(t) => Ok(Event::default()
                .event("text")
                .json_data(&json!({"content": t})).unwrap()),
            ChatChunk::ToolCall { id, name, arguments } => Ok(Event::default()
                .event("tool_call")
                .json_data(&json!({"id": id, "name": name, "arguments": arguments})).unwrap()),
            ChatChunk::ToolResult { tool_call_id, success, content } => Ok(Event::default()
                .event("tool_result")
                .json_data(&json!({"tool_call_id": tool_call_id, "success": success, "content": content})).unwrap()),
            ChatChunk::Done { token_count } => Ok(Event::default()
                .event("done")
                .json_data(&json!({"token_count": token_count})).unwrap()),
            ChatChunk::Error(e) => Ok(Event::default()
                .event("error")
                .json_data(&json!({"error": e})).unwrap()),
        });

    Sse::new(stream)
}
```

### Frontend (fetch + ReadableStream)

```typescript
async function sendMessage(sessionId: string, content: string): AsyncGenerator<ChatChunk> {
  const response = await fetch(`/api/chat/sessions/${sessionId}/messages`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Accept': 'text/event-stream',
    },
    body: JSON.stringify({ content }),
  });

  const reader = response.body!.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const events = parseSSE(buffer);
    buffer = events.remaining;

    for (const event of events.parsed) {
      yield event;
    }
  }
}

function parseSSE(buffer: string): { parsed: ChatChunk[], remaining: string } {
  const parsed: ChatChunk[] = [];
  const lines = buffer.split('\n');
  let remaining = '';
  let currentEvent = '';
  let currentData = '';

  for (const line of lines) {
    if (line.startsWith('event:')) {
      currentEvent = line.slice(6).trim();
    } else if (line.startsWith('data:')) {
      currentData = line.slice(5).trim();
    } else if (line === '' && currentEvent && currentData) {
      parsed.push({ type: currentEvent, ...JSON.parse(currentData) });
      currentEvent = '';
      currentData = '';
    } else if (line !== '') {
      remaining += line + '\n';
    }
  }

  return { parsed, remaining };
}
```

## Event Types

### Job Progress Events
```
event: status     → { "status": "extracting" | "inferring" | "completed" | "failed" }
event: frame      → { "frame": number, "total": number, "text": string }
event: complete   → { "results_count": number }
event: error      → { "error": string }
```

### Chat Events
```
event: text        → { "content": string }
event: tool_call   → { "id": string, "name": string, "arguments": object }
event: tool_result → { "tool_call_id": string, "success": boolean, "content": string }
event: done        → { "token_count": number | null }
event: error       → { "error": string }
```

## Best Practices

1. **Always use keepalive** - Prevents proxy/load balancer timeouts
2. **Include event IDs** - Enables `Last-Event-ID` for reconnection
3. **Clean up on unmount** - Close EventSource in onDestroy
4. **Handle reconnection** - Exponential backoff on error
5. **Use typed events** - Don't rely on generic 'message' event
6. **Buffer partial messages** - SSE chunks may split across reads
