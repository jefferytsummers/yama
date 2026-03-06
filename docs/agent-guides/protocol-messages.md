# Protocol Messages Reference

Complete reference for Yama Protocol Buffer messages and topic patterns.

## Location

- Definitions: `shared/protocol/proto/`
- Generated Rust: `shared/protocol/src/`

## Envelope Pattern

All messages are wrapped in an Envelope:

```protobuf
message Envelope {
    string message_id = 1;      // UUID for correlation
    string topic = 2;           // Routing topic
    string source = 3;          // Sender service ID
    string target = 4;          // Recipient (empty = broadcast)
    Timestamp timestamp = 5;
    string correlation_id = 6;  // For request/response pairing
    google.protobuf.Any payload = 7;
    int32 priority = 8;         // Higher = more urgent
    int64 ttl_ms = 9;           // Time-to-live
}
```

## Rust Usage

```rust
use yama_protocol::{encode, decode, envelope};
use yama_protocol::common::Envelope;

// Create envelope
let env = envelope("video.frame.decoded", "video-decoder", frame_message);

// Encode to bytes
let bytes = encode(&env)?;

// Decode from bytes
let decoded: Envelope = decode(&bytes)?;

// Access payload
if let Some(payload) = &decoded.payload {
    let frame: VideoFrame = decode(&payload.value)?;
}
```

---

## Topic Naming Convention

```
{domain}.{entity}.{action}
```

### Domains

| Domain | Description |
|--------|-------------|
| system | System management, health, config |
| agent | LLM agent communication |
| video | Video frames, detection |
| tools | Tool execution |

---

## System Messages

### Subscribe/Unsubscribe

```protobuf
message Subscribe {
    repeated string topics = 1;    // Exact topics
    repeated string patterns = 2;  // Wildcards: "video.*"
}

message Unsubscribe {
    repeated string topics = 1;
    repeated string patterns = 2;
}
```

Topics:
- `system.subscribe` - Client subscribes
- `system.unsubscribe` - Client unsubscribes

### Health Check

```protobuf
message HealthCheckRequest {
    string service_id = 1;
}

message HealthCheckResponse {
    string service_id = 1;
    ServiceStatus status = 2;
    string message = 3;
    int64 uptime_ms = 4;
}

enum ServiceStatus {
    SERVICE_STATUS_UNKNOWN = 0;
    SERVICE_STATUS_HEALTHY = 1;
    SERVICE_STATUS_DEGRADED = 2;
    SERVICE_STATUS_UNHEALTHY = 3;
}
```

Topics:
- `system.health.request` - Request health check
- `system.health.response` - Health response

### Service Info

```protobuf
message ServiceInfo {
    string id = 1;
    string name = 2;
    ServiceType type = 3;
    ServiceStatus status = 4;
    string version = 5;
    map<string, string> metadata = 6;
}

enum ServiceType {
    SERVICE_TYPE_UNKNOWN = 0;
    SERVICE_TYPE_VIDEO = 1;
    SERVICE_TYPE_INFERENCE = 2;
    SERVICE_TYPE_AGENT = 3;
    SERVICE_TYPE_TOOL = 4;
}
```

Topics:
- `system.service.started` - Service started
- `system.service.stopped` - Service stopped
- `system.service.list` - List services

---

## Agent Messages

### Conversation

```protobuf
message ConversationRequest {
    string agent_id = 1;
    string connection_id = 2;
    repeated Message messages = 3;
    AgentConfig config = 4;
}

message ConversationResponse {
    string agent_id = 1;
    string connection_id = 2;
    Message message = 3;
    ConversationStatus status = 4;
    repeated ToolCall tool_calls = 5;
}

message Message {
    MessageRole role = 1;
    oneof content {
        string text = 2;
        ImageContent image = 3;
        ToolResultContent tool_result = 4;
    }
    string id = 5;
}

enum MessageRole {
    MESSAGE_ROLE_UNKNOWN = 0;
    MESSAGE_ROLE_USER = 1;
    MESSAGE_ROLE_ASSISTANT = 2;
    MESSAGE_ROLE_SYSTEM = 3;
    MESSAGE_ROLE_TOOL = 4;
}

enum ConversationStatus {
    CONVERSATION_STATUS_UNKNOWN = 0;
    CONVERSATION_STATUS_IN_PROGRESS = 1;
    CONVERSATION_STATUS_COMPLETED = 2;
    CONVERSATION_STATUS_ERROR = 3;
    CONVERSATION_STATUS_TOOL_USE = 4;
}
```

Topics:
- `agent.request` - Start/continue conversation
- `agent.response` - Agent response
- `agent.stream` - Streaming response chunk

### Tool Use

```protobuf
message ToolCall {
    string id = 1;
    string name = 2;
    string arguments = 3;  // JSON string
}

message ToolResult {
    string tool_call_id = 1;
    oneof result {
        string text = 2;
        bytes binary = 3;
        Error error = 4;
    }
}
```

Topics:
- `agent.tool.call` - Tool execution request
- `agent.tool.result` - Tool execution result

---

## Video Messages

### Video Frame

```protobuf
message VideoFrame {
    string source_id = 1;
    uint32 frame_number = 2;
    int64 pts_us = 3;
    int64 dts_us = 4;
    int64 duration_us = 5;
    uint32 width = 6;
    uint32 height = 7;
    PixelFormat format = 8;
    oneof data {
        bytes raw_data = 9;
        DmaBufRef dma_buf = 10;
    }
}

message DmaBufRef {
    int32 fd = 1;
    uint64 offset = 2;
    uint32 stride = 3;
    uint64 modifier = 4;
}

enum PixelFormat {
    PIXEL_FORMAT_UNKNOWN = 0;
    PIXEL_FORMAT_NV12 = 1;
    PIXEL_FORMAT_I420 = 2;
    PIXEL_FORMAT_BGRA = 3;
    PIXEL_FORMAT_RGBA = 4;
}
```

Topics:
- `video.frame.raw` - Raw camera frame
- `video.frame.decoded` - Hardware decoded frame
- `video.frame.{source_id}` - Source-specific frames

### Detection Results

```protobuf
message DetectionResult {
    string source_id = 1;
    uint32 frame_number = 2;
    int64 timestamp_us = 3;
    repeated Detection detections = 4;
}

message Detection {
    string id = 1;
    string class_name = 2;
    float confidence = 3;
    BoundingBox bbox = 4;
    optional string track_id = 5;
    map<string, string> attributes = 6;
}

message BoundingBox {
    float x = 1;      // Normalized 0-1
    float y = 2;
    float width = 3;
    float height = 4;
}
```

Topics:
- `video.detection` - Detection results
- `video.detection.{source_id}` - Source-specific

---

## Error Messages

```protobuf
message Error {
    ErrorCode code = 1;
    string message = 2;
    string details = 3;
    string source = 4;
}

enum ErrorCode {
    ERROR_CODE_UNKNOWN = 0;
    ERROR_CODE_INVALID_REQUEST = 1;
    ERROR_CODE_NOT_FOUND = 2;
    ERROR_CODE_INTERNAL = 3;
    ERROR_CODE_TIMEOUT = 4;
    ERROR_CODE_UNAVAILABLE = 5;
}
```

---

## Topic Patterns

### Wildcard Matching

```
video.*           # All video topics
system.service.*  # All service events
agent.*           # All agent messages
```

### Common Patterns

| Pattern | Matches |
|---------|---------|
| `video.frame.*` | All frame topics |
| `system.*` | All system messages |
| `agent.response` | Exact match only |
| `tools.*.result` | Tool results |

---

## Message Flow Examples

### Agent Conversation

```
Client                          Host                           Agent
  │                               │                               │
  │─── agent.request ────────────►│                               │
  │                               │─── agent.request ────────────►│
  │                               │                               │
  │                               │◄── agent.response ────────────│
  │◄── agent.response ───────────│                               │
  │                               │                               │
```

### Tool Execution

```
Agent                           Host                           Tool
  │                               │                               │
  │─── agent.tool.call ──────────►│                               │
  │                               │─── tools.{name}.execute ─────►│
  │                               │                               │
  │                               │◄── tools.{name}.result ───────│
  │◄── agent.tool.result ────────│                               │
  │                               │                               │
```

### Video Pipeline

```
Camera                          Decoder                        Inference
  │                               │                               │
  │─── video.frame.raw ──────────►│                               │
  │                               │─── video.frame.decoded ──────►│
  │                               │                               │
  │                               │◄── video.detection ───────────│
  │                               │                               │
```

---

## Serialization

```rust
use prost::Message;

// Encode
fn encode<M: Message>(message: &M) -> Result<Vec<u8>, prost::EncodeError> {
    let mut buf = Vec::with_capacity(message.encoded_len());
    message.encode(&mut buf)?;
    Ok(buf)
}

// Decode
fn decode<M: Message + Default>(buf: &[u8]) -> Result<M, prost::DecodeError> {
    M::decode(buf)
}
```

## Dependencies

```toml
[dependencies]
prost = "0.13"
prost-types = "0.13"

[build-dependencies]
prost-build = "0.13"
```
