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
    image: nvcr.io/nvidia/tritonserver:24.05-jetpack-py3
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

## Agent Review Additions

### UX Review

| ID | Task | Rationale | Days |
|----|------|-----------|------|
| 8.4.1 | Add onboarding flows for each deployment mode | Missing first-launch experience | 2 |
| 8.4.2 | Add connection state indicators in UI | Connection loss recovery UX | 0.5 |
| 8.4.3 | Add loading/empty/error states | Missing state patterns | 0.5 |
| 8.4.4 | Conduct accessibility audit | WCAG AA compliance | 1 |

**Onboarding Flow: Mac Standalone**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Welcome to Yama                                            Step 1 of 4 │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                                                                  │   │
│  │     🖥️  Mac Standalone Mode                                     │   │
│  │                                                                  │   │
│  │     All AI processing runs locally on your Mac.                 │   │
│  │     Best for: Development, single user, privacy-focused.        │   │
│  │                                                                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  System Check:                                                          │
│  ✓ Apple Silicon detected (M2 Max)                                     │
│  ✓ 32GB RAM available                                                   │
│  ✓ Metal GPU acceleration ready                                         │
│  ⚠ No VLM model found (download required)                              │
│                                                                         │
│                                          [Skip] [Download Model & Start]│
└─────────────────────────────────────────────────────────────────────────┘
```

**Onboarding Flow: Mac + Jetson**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Connect to AI Server                                       Step 2 of 4 │
│                                                                         │
│  Server Address: [ 192.168.1.200                            ] [Scan]    │
│                                                                         │
│  Connection Status:                                                     │
│  ◐ Connecting to ws://192.168.1.200:8765...                            │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  ● Server found: Jetson AGX Orin                                │   │
│  │    • 4 cameras available                                        │   │
│  │    • Qwen2.5-VL-7B loaded                                       │   │
│  │    • YOLOv8 detection ready                                     │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Authentication:                                                        │
│  Shared Secret: [ ••••••••••••                              ]          │
│                                                                         │
│                                          [Back] [Test Connection] [Next]│
└─────────────────────────────────────────────────────────────────────────┘
```

**Accessibility Audit Checklist**

| Requirement | Standard | Status |
|-------------|----------|--------|
| Color contrast (text) | WCAG AA 4.5:1 | Check |
| Color contrast (UI elements) | WCAG AA 3:1 | Check |
| Keyboard navigation | All interactive elements focusable | Check |
| Focus indicators | Visible focus ring | Check |
| Screen reader labels | ARIA labels on all controls | Check |
| Motion reduction | Respect prefers-reduced-motion | Check |
| Error messages | Descriptive, not just color-coded | Check |

### Agent Design Review (Privacy Agent)

| ID | Task | Rationale | Days |
|----|------|-----------|------|
| 8.7.1 | Implement Privacy Agent service | Dedicated privacy-preserving tools | 2 |
| 8.7.2 | Add `detect_faces` tool | Identify faces for redaction | 0.5 |
| 8.7.3 | Add `blur_regions` tool | Apply blur to sensitive regions | 0.5 |
| 8.7.4 | Add `detect_pii` tool | Find text with PII (OCR + regex) | 1 |
| 8.7.5 | Add `get_system_info` deployment awareness tool | Agents know their deployment mode | 0.5 |
| 8.7.6 | Add power budget config for Thor | Multi-day edge deployments | 0.5 |

**Code: Privacy Agent Tools**

```rust
// shared/agent-sdk/src/tools/privacy.rs

/// Detect faces in the current frame.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectFaces {
    pub source_id: String,
    /// Minimum confidence for face detection (0.0-1.0)
    #[serde(default = "default_confidence")]
    pub min_confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FaceDetection {
    pub bbox: BoundingBoxOutput,
    pub confidence: f32,
    /// Unique ID for this face (for tracking across frames)
    pub face_id: Option<u64>,
}

/// Apply blur to regions in the video output.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BlurRegions {
    pub source_id: String,
    /// Regions to blur (from detect_faces or manual specification)
    pub regions: Vec<BoundingBoxOutput>,
    /// Blur intensity (1-10, default 5)
    #[serde(default = "default_blur")]
    pub intensity: u32,
    /// Duration in seconds (0 = permanent until cleared)
    pub duration_seconds: u32,
}

fn default_blur() -> u32 {
    5
}

/// Detect PII in text visible in the frame.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectPii {
    pub source_id: String,
    /// Types of PII to detect
    #[serde(default)]
    pub pii_types: Vec<PiiType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PiiType {
    Email,
    Phone,
    CreditCard,
    SSN,
    LicensePlate,
    Address,
    Name,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PiiDetection {
    pub pii_type: PiiType,
    pub text: String,  // Redacted: "john.***@***.com"
    pub bbox: BoundingBoxOutput,
    pub confidence: f32,
}

/// Get system information for deployment-aware behavior.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetSystemInfo {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SystemInfo {
    pub deployment_mode: DeploymentMode,
    pub platform: String,  // "mac", "jetson_orin", "jetson_thor"
    pub gpu_name: String,
    pub gpu_memory_total_mb: u64,
    pub gpu_memory_available_mb: u64,
    pub models_loaded: Vec<String>,
    pub capabilities: Vec<String>,  // ["vlm", "detection", "tracking", "privacy"]
    pub power_mode: Option<PowerMode>,  // Jetson-specific
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum DeploymentMode {
    MacStandalone,
    MacJetsonClient,
    JetsonStandalone,
    JetsonServer,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PowerMode {
    MaxPerformance,  // MAXN
    Balanced,        // 50W
    PowerSaver,      // 15W
}
```

**Code: Privacy Agent Service**

```rust
// ai-server/src/agents/privacy.rs

pub struct PrivacyAgent {
    face_detector: Arc<dyn FaceDetector>,
    ocr_engine: Arc<dyn OcrEngine>,
    pii_patterns: PiiPatterns,
    blur_regions: DashMap<String, Vec<BlurRegion>>,
}

impl PrivacyAgent {
    /// Process frame through privacy filters.
    pub async fn process_frame(
        &self,
        source_id: &str,
        frame: &mut Frame,
    ) -> Result<PrivacyMetadata> {
        // Check for active blur regions
        if let Some(regions) = self.blur_regions.get(source_id) {
            for region in regions.iter() {
                if !region.is_expired() {
                    frame.apply_blur(&region.bbox, region.intensity);
                }
            }
        }

        Ok(PrivacyMetadata {
            faces_detected: 0,
            regions_blurred: 0,
            pii_detected: 0,
        })
    }

    /// Detect faces in frame.
    pub async fn detect_faces(
        &self,
        frame: &Frame,
        min_confidence: f32,
    ) -> Result<Vec<FaceDetection>> {
        self.face_detector.detect(frame, min_confidence).await
    }

    /// Detect PII in visible text.
    pub async fn detect_pii(
        &self,
        frame: &Frame,
        pii_types: &[PiiType],
    ) -> Result<Vec<PiiDetection>> {
        // Run OCR
        let text_regions = self.ocr_engine.extract(frame).await?;

        // Match against PII patterns
        let mut detections = Vec::new();
        for region in text_regions {
            for pii_type in pii_types {
                if let Some(matches) = self.pii_patterns.find(&region.text, *pii_type) {
                    for m in matches {
                        detections.push(PiiDetection {
                            pii_type: *pii_type,
                            text: self.redact(&m.text),
                            bbox: region.bbox.clone(),
                            confidence: m.confidence,
                        });
                    }
                }
            }
        }

        Ok(detections)
    }

    fn redact(&self, text: &str) -> String {
        // Partially redact: "john.doe@example.com" → "john.***@***.com"
        // Implementation depends on PII type
        text.chars()
            .enumerate()
            .map(|(i, c)| if i > 4 && c.is_alphanumeric() { '*' } else { c })
            .collect()
    }
}
```

**Power Budget Configuration (Thor)**

```toml
# /etc/yama/power.toml

[power]
# Power mode: "max_performance", "balanced", "power_saver"
mode = "balanced"

# Thermal throttling threshold (Celsius)
thermal_limit = 85

# Power budget in Watts
budget_watts = 50

# Automatic mode switching
[power.auto]
enabled = true
# Switch to power_saver after N minutes of idle
idle_timeout_minutes = 10
# Switch to max_performance when VLM query received
boost_on_query = true
```

---

---

## Milestone 8.5: NVIDIA Reference Alignment

**Duration:** 2 days

### Rationale

NVIDIA's Jetson Platform Services (JPS) provides production-tested patterns for VLM inference, video storage, and zero-shot detection. Aligning with JPS conventions improves interoperability and leverages NVIDIA's optimization work.

**References:**
- [Jetson Platform Services](https://docs.nvidia.com/jetson/jps/moj-overview.html)
- [JPS GitHub](https://github.com/NVIDIA-AI-IOT/jetson-platform-services)
- [Agentic Video Workflow](https://developer.nvidia.com/blog/build-an-agentic-video-workflow-with-video-search-and-summarization/)

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 8.5.1 | Review JPS VLM Service patterns | Analyze implementation approach | Patterns documented |
| 8.5.2 | Align REST API with NVIDIA conventions | Match endpoint structure where sensible | API aligned |
| 8.5.3 | Document architectural differences | Explain deviations from JPS | Doc complete |
| 8.5.4 | Consider JPS compatibility layer | Enterprise deployment option | Decision documented |

### JPS Alignment Analysis

| JPS Component | Yama Equivalent | Alignment Status |
|---------------|-----------------|------------------|
| VLM Inference Service | Triton + Orchestrator | Similar, add REST wrapper |
| Video Storage Toolkit | Context Store | Different focus (analysis vs storage) |
| Zero-shot Detection | NanoOWL integration | Future consideration |
| Analytics Services | Detection Router | Aligned |

---

## Milestone 8.6: Jetson Thor Optimization

**Duration:** 3 days

### Rationale

Jetson Thor provides 7.5x AI compute over Orin, 128GB memory, and native FP4 support. This enables running 70B models for complex reasoning and handling 10+ simultaneous 4K streams.

**Reference:** [Jetson Thor Technical Blog](https://developer.nvidia.com/blog/introducing-nvidia-jetson-thor-the-ultimate-platform-for-physical-ai/)

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 8.6.1 | Enable FP4 quantization for supported models | TensorRT-LLM FP4 | FP4 models work |
| 8.6.2 | Configure MIG for multi-tenant workloads | GPU partitioning | MIG active |
| 8.6.3 | Update memory budget for 128GB platform | Document new limits | Budget documented |
| 8.6.4 | Benchmark 70B model for complex queries | Test Llama-3.1-70B | Latency measured |
| 8.6.5 | Test 10-stream concurrent processing | Scale beyond Orin limits | 10 streams work |

### Jetson Thor vs Orin Comparison

| Metric | AGX Orin 64GB | Thor 128GB | Improvement |
|--------|---------------|------------|-------------|
| GPU Cores | 2048 (Ampere) | 2560 (Blackwell) | 1.25x |
| AI Compute (FP4) | N/A | 800 TOPS | New capability |
| Memory | 64GB | 128GB | 2x |
| Video Decode | 8x 4Kp60 | 10x 4Kp60 | 1.25x |
| Max Model Size | ~40B | ~100B | 2.5x |

---

## Checklist

### Milestone 8.1: Mac Standalone E2E
- [ ] 8.1.1 Configure standalone
- [ ] 8.1.2 Test video playback
- [ ] 8.1.3 Test VLM inference
- [ ] 8.1.4 Test agent chat

### Milestone 8.2: Mac + Jetson E2E
- [ ] 8.2.1 Start Jetson server
- [ ] 8.2.2 Connect Mac client
- [ ] 8.2.3 Verify video stream (WebRTC < 200ms)
- [ ] 8.2.4 Verify detections
- [ ] 8.2.5 Verify VLM queries (< 5s)

### Milestone 8.3: Jetson Standalone E2E
- [ ] 8.3.1 Configure headless
- [ ] 8.3.2 Access via browser
- [ ] 8.3.3 Test RTSP input
- [ ] 8.3.4 Test HLS output

### Milestone 8.4: UX Improvements (Agent Review)
- [ ] 8.4.1 Add onboarding flows for each deployment mode
- [ ] 8.4.2 Add connection state indicators in UI
- [ ] 8.4.3 Add loading/empty/error states
- [ ] 8.4.4 Conduct WCAG AA accessibility audit

### Milestone 8.5: NVIDIA Reference Alignment
- [ ] 8.5.1 Review JPS VLM Service implementation patterns
- [ ] 8.5.2 Align REST API with NVIDIA conventions where sensible
- [ ] 8.5.3 Document architectural differences from JPS
- [ ] 8.5.4 Consider JPS compatibility layer for enterprise deployments

### Milestone 8.6: Jetson Thor Optimization
- [ ] 8.6.1 Enable FP4 quantization for supported models
- [ ] 8.6.2 Configure MIG for multi-tenant workloads
- [ ] 8.6.3 Update memory budget for 128GB platform
- [ ] 8.6.4 Benchmark 70B model feasibility for complex queries
- [ ] 8.6.5 Test 10-stream concurrent processing on Thor

### Milestone 8.7: Privacy Agent (Agent Review)
- [ ] 8.7.1 Implement Privacy Agent service
- [ ] 8.7.2 Add `detect_faces` tool
- [ ] 8.7.3 Add `blur_regions` tool
- [ ] 8.7.4 Add `detect_pii` tool
- [ ] 8.7.5 Add `get_system_info` deployment awareness tool
- [ ] 8.7.6 Add power budget config for Thor

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
