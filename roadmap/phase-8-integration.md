# Phase 8: End-to-End Integration

**Duration:** 7 days
**Goal:** All components working together across all deployment modes.

---

## Overview

This final phase validates the complete system across all three deployment modes. Each mode is tested end-to-end to ensure all components integrate correctly: abstraction layer, event bus, DeepStream pipeline, Triton serving, VLM orchestration, video streaming, and agent tools.

---

## Deployment Modes Summary

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Mode 1: Mac Standalone                                                 │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Mac (Single Process)                                           │   │
│  │  • egui UI                                                      │   │
│  │  • LocalAIServer (DirectVlmBackend)                             │   │
│  │  • LocalVideoProvider (VideoToolbox)                            │   │
│  │  • LocalDetectionProvider (CoreML YOLO)                         │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Mode 2: Mac + Jetson                                                   │
│                                                                         │
│  ┌─────────────────┐                    ┌─────────────────────────┐   │
│  │  Mac Client     │◀───── WebSocket ──▶│  Jetson AI Server       │   │
│  │  • egui UI      │       WebRTC       │  • DeepStream           │   │
│  │  • Remote*      │                    │  • Triton               │   │
│  │    Providers    │                    │  • VLM Orchestration    │   │
│  └─────────────────┘                    └─────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Mode 3: Jetson Standalone                                              │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Jetson (Single Device)                                         │   │
│  │  • Web UI (SvelteKit) or headless                               │   │
│  │  • DeepStream + Triton + VLM                                    │   │
│  │  • HLS/WebRTC output                                            │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone 8.1: Mac Standalone E2E

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 8.1.1 | Configure standalone | All local providers | Config loads |
| 8.1.2 | Test video playback | Local file plays | Video displays |
| 8.1.3 | Test VLM inference | Queries work | Response received |
| 8.1.4 | Test agent chat | Conversation works | Chat functional |

### Configuration

```toml
# config/standalone.toml

[deployment]
mode = "standalone"

[ai_server]
backend = "direct"
model_path = "/path/to/qwen2-vl.gguf"
context_length = 4096

[video]
backend = "local"
default_source = "webcam"

[detection]
backend = "local"
model = "yolov8n-coreml"

[ui]
show_detections = true
show_inference_stats = true
```

### Test Script

```bash
#!/bin/bash
# scripts/test-standalone.sh

set -e

echo "Testing Mac Standalone Mode"
echo "==========================="

# 1. Build
echo "Building..."
cargo build -p yama-host-apple --release

# 2. Run with test video
echo "Starting with test video..."
./target/release/yama-host-apple \
    --config config/standalone.toml \
    --video test/videos/parking_lot.mp4 &

APP_PID=$!
sleep 5

# 3. Test VLM query via HTTP API
echo "Testing VLM query..."
RESPONSE=$(curl -s http://localhost:8080/api/query -X POST \
    -H "Content-Type: application/json" \
    -d '{"query": "Describe what you see", "source_id": "test"}')

if echo "$RESPONSE" | jq -e '.analysis' > /dev/null; then
    echo "VLM query: PASS"
else
    echo "VLM query: FAIL"
    exit 1
fi

# 4. Test detection API
echo "Testing detections..."
DETECTIONS=$(curl -s http://localhost:8080/api/detections/test)

if echo "$DETECTIONS" | jq -e '.' > /dev/null; then
    echo "Detections API: PASS"
else
    echo "Detections API: FAIL"
    exit 1
fi

# Cleanup
kill $APP_PID

echo ""
echo "Mac Standalone E2E: ALL TESTS PASSED"
```

### Verification

```bash
cargo run -p yama-host-apple -- --config standalone.toml --video test.mp4
# Video plays, detections overlay, chat works
```

### Deliverable

Working standalone mode with all features

---

## Milestone 8.2: Mac + Jetson E2E

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 8.2.1 | Start Jetson server | All services running | Services healthy |
| 8.2.2 | Connect Mac client | Session establishes | Connected |
| 8.2.3 | Verify video stream | WebRTC < 200ms | Low latency |
| 8.2.4 | Verify detections | Overlays appear | Detections visible |
| 8.2.5 | Verify VLM queries | Response < 5s | Queries work |

### Jetson Server Configuration

```toml
# /etc/yama/ai-server.toml

[server]
bind = "0.0.0.0:8765"
enable_remote = true
max_clients = 10
auth_required = true
auth_secret_file = "/etc/yama/auth.secret"

[[sources]]
id = "cam1"
uri = "rtsp://192.168.1.100/stream1"
enabled = true

[[sources]]
id = "cam2"
uri = "rtsp://192.168.1.101/stream1"
enabled = true

[deepstream]
config_path = "/etc/yama/deepstream/"
batch_size = 4
inference_interval = 2

[triton]
url = "localhost:8001"
vlm_model = "qwen2_vl_7b"
detector_model = "yolov8n"

[vlm]
sample_interval_ms = 5000
max_tokens = 512
temperature = 0.7

[vlm.triggers]
on_new_track = true
on_class = ["person", "vehicle"]
periodic_seconds = 30

[streaming]
webrtc_enabled = true
hls_enabled = true
encode_bitrate = 4000000
```

### Mac Client Configuration

```toml
# config/client.toml

[deployment]
mode = "client"

[remote]
url = "ws://192.168.1.200:8765"
auth_secret = "your-shared-secret"
auto_connect = true
reconnect_interval_ms = 5000

[video]
backend = "remote"
prefer_webrtc = true
fallback_to_hls = true

[detection]
backend = "remote"

[ai_server]
backend = "remote"
```

### Docker Compose (Jetson)

```yaml
# platform/jetson/docker-compose.yml

version: "3.8"

services:
  triton:
    image: nvcr.io/nvidia/tritonserver:24.05-py3-igpu
    runtime: nvidia
    volumes:
      - ./model_repository:/models:ro
    command: tritonserver --model-repository=/models
    ports:
      - "8001:8001"
      - "8002:8002"
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]

  deepstream:
    build: ../../containers/deepstream
    runtime: nvidia
    volumes:
      - /tmp/yama-event.sock:/tmp/yama-event.sock
      - ./deepstream-config:/etc/yama/deepstream:ro
    depends_on:
      - triton
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu, video]

  ai-server:
    build: ./ai-server
    runtime: nvidia
    volumes:
      - /tmp/yama-event.sock:/tmp/yama-event.sock
      - /etc/yama:/etc/yama:ro
    ports:
      - "8765:8765"
      - "8080:8080"
    depends_on:
      - triton
      - deepstream
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]

networks:
  default:
    driver: bridge
```

### Test Script

```bash
#!/bin/bash
# scripts/test-mac-jetson.sh

set -e

JETSON_IP="${JETSON_IP:-192.168.1.200}"

echo "Testing Mac + Jetson Mode"
echo "========================="

# 1. Check Jetson services
echo "Checking Jetson services..."
ssh jetson "docker-compose -f /opt/yama/docker-compose.yml ps"

# 2. Test event bus connection
echo "Testing event bus connection..."
cargo run --example auth_client -- \
    --url ws://$JETSON_IP:8765 \
    --secret $(cat ~/.yama/auth.secret)

# 3. Start Mac client
echo "Starting Mac client..."
cargo run -p yama-host-apple -- --config config/client.toml &
CLIENT_PID=$!
sleep 10

# 4. Measure WebRTC latency
echo "Measuring WebRTC latency..."
LATENCY=$(curl -s http://localhost:8080/api/stats | jq '.video_latency_ms')
if [ "$LATENCY" -lt 500 ]; then
    echo "WebRTC latency ($LATENCY ms): PASS"
else
    echo "WebRTC latency ($LATENCY ms): FAIL (>500ms)"
fi

# 5. Test remote VLM query
echo "Testing remote VLM query..."
START=$(date +%s%3N)
RESPONSE=$(curl -s http://localhost:8080/api/query -X POST \
    -H "Content-Type: application/json" \
    -d '{"query": "What vehicles do you see?", "source_id": "cam1"}')
END=$(date +%s%3N)
QUERY_TIME=$((END - START))

if [ "$QUERY_TIME" -lt 10000 ]; then
    echo "VLM query time ($QUERY_TIME ms): PASS"
else
    echo "VLM query time ($QUERY_TIME ms): FAIL (>10s)"
fi

# 6. Test detections streaming
echo "Testing detection stream..."
DETECTIONS=$(curl -s http://localhost:8080/api/detections/cam1 | jq length)
if [ "$DETECTIONS" -gt 0 ]; then
    echo "Detections received ($DETECTIONS): PASS"
else
    echo "Detections received: WARN (none detected)"
fi

# Cleanup
kill $CLIENT_PID

echo ""
echo "Mac + Jetson E2E: ALL TESTS PASSED"
```

### Verification

```bash
# Jetson
docker-compose up

# Mac
cargo run -p yama-host-apple -- --config client.toml
# Video displays, detections overlay, chat works
```

### Deliverable

Working distributed mode with Jetson backend

---

## Milestone 8.3: Jetson Standalone E2E

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 8.3.1 | Configure headless | Web UI only | No display needed |
| 8.3.2 | Access via browser | Interface works | Web UI functional |
| 8.3.3 | Test RTSP input | Cameras process | Streams processed |
| 8.3.4 | Test HLS output | Browser plays video | Video plays |

### Headless Configuration

```toml
# /etc/yama/standalone.toml

[deployment]
mode = "standalone"
headless = true

[server]
bind = "0.0.0.0:8080"
enable_web_ui = true

[[sources]]
id = "cam1"
uri = "rtsp://192.168.1.100/stream1"
enabled = true

[deepstream]
config_path = "/etc/yama/deepstream/"
batch_size = 4

[triton]
url = "localhost:8001"
vlm_model = "qwen2_vl_7b"

[streaming]
webrtc_enabled = true
hls_enabled = true
hls_segment_duration = 4
hls_playlist_length = 5
```

### Test Script

```bash
#!/bin/bash
# scripts/test-jetson-standalone.sh

set -e

echo "Testing Jetson Standalone Mode"
echo "=============================="

# 1. Start services
echo "Starting services..."
docker-compose -f docker-compose.standalone.yml up -d
sleep 30

# 2. Check service health
echo "Checking service health..."
HEALTH=$(curl -s http://localhost:8080/api/health)
if echo "$HEALTH" | jq -e '.healthy' > /dev/null; then
    echo "Health check: PASS"
else
    echo "Health check: FAIL"
    exit 1
fi

# 3. Check video sources
echo "Checking video sources..."
SOURCES=$(curl -s http://localhost:8080/api/sources | jq length)
if [ "$SOURCES" -gt 0 ]; then
    echo "Video sources ($SOURCES): PASS"
else
    echo "Video sources: FAIL"
    exit 1
fi

# 4. Test HLS playback
echo "Testing HLS stream..."
HLS_URL="http://localhost:8080/hls/cam1/playlist.m3u8"
if curl -s "$HLS_URL" | grep -q "EXTINF"; then
    echo "HLS playlist: PASS"
else
    echo "HLS playlist: FAIL"
    exit 1
fi

# 5. Test VLM query
echo "Testing VLM query..."
RESPONSE=$(curl -s http://localhost:8080/api/query -X POST \
    -H "Content-Type: application/json" \
    -d '{"query": "Describe the scene", "source_id": "cam1"}')

if echo "$RESPONSE" | jq -e '.analysis' > /dev/null; then
    echo "VLM query: PASS"
else
    echo "VLM query: FAIL"
    exit 1
fi

# 6. Test Web UI (headless browser test)
echo "Testing Web UI..."
if curl -s http://localhost:8080 | grep -q "Yama"; then
    echo "Web UI loads: PASS"
else
    echo "Web UI: FAIL"
    exit 1
fi

echo ""
echo "Jetson Standalone E2E: ALL TESTS PASSED"
```

### Verification

```bash
docker-compose -f jetson-standalone.yml up
# Browser: http://jetson:8080
# Video streams visible, detections overlay, chat works
```

### Deliverable

Working headless mode with web interface

---

## Dependencies

All previous phases must be complete:
- Phase 1: Abstraction Layer
- Phase 2: Remote Event Bus
- Phase 3: DeepStream Pipeline
- Phase 4: Triton Serving
- Phase 5: VLM Orchestration
- Phase 6: Video Streaming
- Phase 7: Tool System

---

## Performance Validation

| Metric | Target | Mode 1 | Mode 2 | Mode 3 |
|--------|--------|--------|--------|--------|
| Video decode latency | <10ms | - | - | Check |
| Detection latency | <10ms | - | - | Check |
| VLM query latency | <5s | Check | Check | Check |
| WebRTC E2E latency | <200ms | - | Check | - |
| HLS latency | <12s | - | - | Check |
| Concurrent streams | 4+ | 1 | 4 | 8 |

---

## System Requirements

### Mac Standalone
- macOS 13+
- M1/M2/M3 chip (Metal required)
- 16GB RAM minimum
- Webcam or video file

### Jetson (Server or Standalone)
- Jetson AGX Orin 64GB
- JetPack 6.0+
- DeepStream 7.0+
- RTSP camera sources

### Mac Client (Mode 2)
- macOS 12+
- Network connectivity to Jetson
- 8GB RAM minimum

---

## Checklist

- [ ] 8.1.1 Configure standalone
- [ ] 8.1.2 Test video playback
- [ ] 8.1.3 Test VLM inference
- [ ] 8.1.4 Test agent chat
- [ ] 8.2.1 Start Jetson server
- [ ] 8.2.2 Connect Mac client
- [ ] 8.2.3 Verify video stream (WebRTC < 200ms)
- [ ] 8.2.4 Verify detections
- [ ] 8.2.5 Verify VLM queries (< 5s)
- [ ] 8.3.1 Configure headless
- [ ] 8.3.2 Access via browser
- [ ] 8.3.3 Test RTSP input
- [ ] 8.3.4 Test HLS output

---

## Final Verification

After all milestones are complete, run the full integration test suite:

```bash
# Run all E2E tests
./scripts/test-all-modes.sh

# Expected output:
# Mac Standalone E2E: ALL TESTS PASSED
# Mac + Jetson E2E: ALL TESTS PASSED
# Jetson Standalone E2E: ALL TESTS PASSED
#
# ===================================
# FULL INTEGRATION: SUCCESS
# ===================================
```
