# Phase 3: DeepStream Video Pipeline

**Duration:** 10 days
**Goal:** Multi-stream video analytics with hardware-accelerated detection on Jetson.

---

## Overview

This phase implements the DeepStream video pipeline that handles multi-stream RTSP ingestion, hardware-accelerated decoding (NVDEC), batched object detection (YOLOv8), and GPU-based tracking (NvDCF). The pipeline extracts metadata and publishes detections to the event bus.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Video Ingestion (DeepStream 7.x)                                       │
│                                                                         │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐                           │
│  │RTSP #1 │ │RTSP #2 │ │RTSP #3 │ │... #8  │  (NVDEC decode)           │
│  └───┬────┘ └───┬────┘ └───┬────┘ └───┬────┘                           │
│      └──────────┴──────────┴──────────┘                                 │
│                      │                                                  │
│                      ▼                                                  │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  nvstreammux (batch frames from all streams)                     │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                      │                                                  │
│                      ▼                                                  │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  nvinfer (YOLOv8n TensorRT) - batched inference                  │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                      │                                                  │
│                      ▼                                                  │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  nvtracker (NvDCF) - GPU-accelerated tracking                    │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                      │                                                  │
│          ┌───────────┼───────────┐                                      │
│          ▼           ▼           ▼                                      │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐                          │
│  │ Metadata   │ │ Frame      │ │ H.264      │                          │
│  │ Probe      │ │ Sampler    │ │ Encode     │                          │
│  │ (→ Event   │ │ (→ VLM)    │ │ (→ Stream) │                          │
│  │   Bus)     │ │            │ │            │                          │
│  └────────────┘ └────────────┘ └────────────┘                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Technology Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Pipeline | DeepStream 7.x | Native multi-stream batching, NVDEC integration |
| Container | `nvcr.io/nvidia/deepstream-l4t:7.0-samples-jetpack` | **CRITICAL: Use `jetpack` suffix, NOT `multiarch` or `py3-igpu`** |
| Detection | YOLOv8n TensorRT (INT8) | Best speed/accuracy for real-time, INT8 gives 2x speedup |
| Tracker | NvDCF | GPU-accelerated, low latency |
| Output | H.264 via nvv4l2h264enc | Hardware encode for streaming |

> **⚠️ Container Tag Warning:** Always use architecture-specific tags for Jetson:
> - ✅ `deepstream-l4t:7.0-samples-jetpack`
> - ❌ `deepstream-l4t:7.0-triton-multiarch` (will fail on Jetson)

**Why DeepStream over custom GStreamer:**
- Batched inference across streams (single GPU kernel for N streams)
- Built-in tracker metadata management
- Native Triton integration for VLM handoff
- Probe-based frame extraction without pipeline stalls

---

## Milestone 3.1: DeepStream Pipeline Setup

**Duration:** 5 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 3.1.1 | Create DeepStream app skeleton | Basic C++ app structure | Compiles with DeepStream SDK |
| 3.1.2 | Configure multi-source RTSP | 4+ simultaneous streams | 4 streams decode at 30fps |
| 3.1.3 | Add YOLOv8n primary inference | TensorRT inference | Detections appear |
| 3.1.4 | Configure NvDCF tracker | GPU-accelerated tracking | Tracks persist across frames |
| 3.1.5 | Add H.264 encode output | Hardware re-encoding | Re-encoded streams available |

### Configuration: DeepStream Pipeline

```yaml
# platform/jetson/deepstream-config/pipeline.yml

sources:
  - id: "cam1"
    uri: "rtsp://192.168.1.100/stream1"
    type: rtsp
    width: 1920
    height: 1080
    fps: 30

  - id: "cam2"
    uri: "rtsp://192.168.1.101/stream1"
    type: rtsp
    width: 1920
    height: 1080
    fps: 30

streammux:
  batch-size: 4
  width: 1920
  height: 1080
  batched-push-timeout: 33333  # microseconds (exactly 1 frame at 30fps)
  live-source: 1  # Required for RTSP sources
  sync-inputs: 1  # CRITICAL: Synchronizes multi-stream inputs, prevents frame drift
  buffer-pool-size: 16  # Pre-allocate buffers to reduce allocation spikes

primary_gie:
  config-file: config_infer_yolo.txt
  batch-size: 4
  interval: 2  # Infer every 2 frames

tracker:
  config-file: config_tracker_nvdcf.txt
  ll-lib-file: /opt/nvidia/deepstream/deepstream/lib/libnvds_nvmultiobjecttracker.so
  enable-batch-process: 1

sink:
  - type: fakesink  # Primary analysis sink
  - type: rtsp      # Re-encoded output
    port: 8554
    codec: H264
```

### Configuration: YOLOv8 Inference

```ini
# platform/jetson/deepstream-config/config_infer_yolo.txt

[property]
gpu-id=0
net-scale-factor=0.00392157  # 1/255
model-engine-file=yolov8n.engine
labelfile-path=labels.txt
batch-size=4
network-mode=1  # INT8 (CRITICAL: Use INT8 for ~2x speedup, see Milestone 3.4)
num-detected-classes=80
interval=2
gie-unique-id=1
process-mode=1  # Primary detector
network-type=0  # Detector

[class-attrs-all]
pre-cluster-threshold=0.25
topk=300
nms-iou-threshold=0.45
```

### Configuration: NvDCF Tracker

```ini
# platform/jetson/deepstream-config/config_tracker_nvdcf.txt

[tracker]
tracker-width=640
tracker-height=384
gpu-id=0
ll-lib-file=/opt/nvidia/deepstream/deepstream/lib/libnvds_nvmultiobjecttracker.so
ll-config-file=tracker_config.yml
enable-batch-process=1

# tracker_config.yml
NvDCF:
  useUniqueID: 1
  maxTargetsPerStream: 150
  filterLr: 0.11
  minDetectorConfidence: 0.2
  useBufferedOutput: 0
```

### Code: DeepStream App Skeleton

```cpp
// containers/deepstream/src/main.cpp

#include <gst/gst.h>
#include <glib.h>
#include "gstnvdsmeta.h"
#include "nvds_analytics_meta.h"

static GstPadProbeReturn
osd_sink_pad_buffer_probe(GstPad *pad, GstPadProbeInfo *info, gpointer u_data) {
    GstBuffer *buf = GST_PAD_PROBE_INFO_BUFFER(info);
    NvDsBatchMeta *batch_meta = gst_buffer_get_nvds_batch_meta(buf);

    for (NvDsMetaList *l_frame = batch_meta->frame_meta_list;
         l_frame != NULL; l_frame = l_frame->next) {

        NvDsFrameMeta *frame_meta = (NvDsFrameMeta *)(l_frame->data);

        for (NvDsMetaList *l_obj = frame_meta->obj_meta_list;
             l_obj != NULL; l_obj = l_obj->next) {

            NvDsObjectMeta *obj_meta = (NvDsObjectMeta *)(l_obj->data);

            // Extract detection data
            Detection det;
            det.source_id = frame_meta->source_id;
            det.frame_number = frame_meta->frame_num;
            det.class_id = obj_meta->class_id;
            det.class_name = obj_meta->obj_label;
            det.confidence = obj_meta->confidence;
            det.track_id = obj_meta->object_id;
            det.bbox = {
                obj_meta->rect_params.left,
                obj_meta->rect_params.top,
                obj_meta->rect_params.width,
                obj_meta->rect_params.height
            };

            // Publish to event bus
            publish_detection(det);
        }
    }

    return GST_PAD_PROBE_OK;
}

int main(int argc, char *argv[]) {
    gst_init(&argc, &argv);

    // Initialize event bus client
    init_event_bus("/tmp/yama-event.sock");

    // Create pipeline elements
    GstElement *pipeline = gst_pipeline_new("deepstream-pipeline");
    GstElement *streammux = gst_element_factory_make("nvstreammux", "streammux");
    GstElement *pgie = gst_element_factory_make("nvinfer", "primary-gie");
    GstElement *tracker = gst_element_factory_make("nvtracker", "tracker");
    GstElement *osd = gst_element_factory_make("nvdsosd", "osd");
    GstElement *sink = gst_element_factory_make("fakesink", "sink");

    // Configure and link...
    // Add probe for metadata extraction
    GstPad *osd_sink_pad = gst_element_get_static_pad(osd, "sink");
    gst_pad_add_probe(osd_sink_pad, GST_PAD_PROBE_TYPE_BUFFER,
                      osd_sink_pad_buffer_probe, NULL, NULL);

    // Start pipeline
    gst_element_set_state(pipeline, GST_STATE_PLAYING);

    // Run main loop
    GMainLoop *loop = g_main_loop_new(NULL, FALSE);
    g_main_loop_run(loop);

    return 0;
}
```

### Verification

```bash
./deepstream-yama -c config_4streams.txt
nvidia-smi  # GPU ~60-80%
# 4 streams visible with bounding boxes
```

### Deliverable

`containers/deepstream/src/main.cpp`

---

## Milestone 3.2: Metadata Extraction

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 3.2.1 | Add probe to extract NvDsMeta | Access metadata in callback | Metadata accessible |
| 3.2.2 | Convert to Protobuf | `StreamDetections` serializes | Correct serialization |
| 3.2.3 | Publish to event bus | Detections flow to subscribers | Detections received |
| 3.2.4 | Add frame extraction probe | Frames sampled for VLM | Frames extracted |

### Protocol: Detection Messages

```protobuf
// shared/protocol/proto/detection.proto

syntax = "proto3";
package yama.detection;

import "google/protobuf/timestamp.proto";

// Real-time detection results from DeepStream
message StreamDetections {
    string source_id = 1;
    uint64 frame_number = 2;
    google.protobuf.Timestamp timestamp = 3;
    repeated Detection detections = 4;
    repeated TrackedObject tracks = 5;
}

message Detection {
    string class_name = 1;
    uint32 class_id = 2;
    float confidence = 3;
    BoundingBox bbox = 4;
    uint64 track_id = 5;
}

message BoundingBox {
    float x = 1;      // Left
    float y = 2;      // Top
    float width = 3;
    float height = 4;
}

message TrackedObject {
    uint64 track_id = 1;
    string class_name = 2;
    BoundingBox bbox = 3;
    float velocity_x = 4;
    float velocity_y = 5;
    TrackState state = 6;
}

enum TrackState {
    TRACK_STATE_UNKNOWN = 0;
    TRACK_STATE_NEW = 1;
    TRACK_STATE_ACTIVE = 2;
    TRACK_STATE_LOST = 3;
}

// Frame sample for VLM analysis
message FrameSample {
    string source_id = 1;
    uint64 frame_number = 2;
    google.protobuf.Timestamp timestamp = 3;
    bytes jpeg_data = 4;  // JPEG-encoded frame
    uint32 width = 5;
    uint32 height = 6;
    repeated Detection recent_detections = 7;  // Context
}
```

### Code: Frame Sampler

```cpp
// containers/deepstream/src/frame_sampler.cpp

class FrameSampler {
public:
    FrameSampler(int sample_interval_ms = 5000)
        : sample_interval_ms_(sample_interval_ms) {}

    bool should_sample(int source_id, uint64_t frame_num) {
        auto now = std::chrono::steady_clock::now();
        auto& last = last_sample_[source_id];

        auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
            now - last).count();

        if (elapsed >= sample_interval_ms_) {
            last = now;
            return true;
        }
        return false;
    }

    void sample_frame(GstBuffer* buffer, NvDsFrameMeta* frame_meta) {
        // Map buffer for CPU access
        NvBufSurface *surface;
        if (NvBufSurfaceMap(surface, -1, -1, NVBUF_MAP_READ) != 0) {
            return;
        }

        // Convert to JPEG
        std::vector<uint8_t> jpeg_data;
        encode_jpeg(surface, &jpeg_data);

        NvBufSurfaceUnMap(surface, -1, -1);

        // Create FrameSample message
        FrameSample sample;
        sample.set_source_id(std::to_string(frame_meta->source_id));
        sample.set_frame_number(frame_meta->frame_num);
        sample.set_jpeg_data(jpeg_data.data(), jpeg_data.size());
        sample.set_width(frame_meta->source_frame_width);
        sample.set_height(frame_meta->source_frame_height);

        // Add recent detections as context
        for (auto* obj : frame_meta->obj_meta_list) {
            auto* det = sample.add_recent_detections();
            det->set_class_name(obj->obj_label);
            det->set_confidence(obj->confidence);
            // ... bbox
        }

        // Publish
        publish_frame_sample(sample);
    }

private:
    int sample_interval_ms_;
    std::map<int, std::chrono::steady_clock::time_point> last_sample_;
};
```

### Verification

```bash
cargo run --example detection_subscriber
# Receives detection messages with bbox, class, track_id
```

### Deliverable

`containers/deepstream/src/metadata.cpp`

---

## Milestone 3.3: DeepStream Container

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 3.3.1 | Create Dockerfile | Multi-stage build | Container builds |
| 3.3.2 | Add config volume mount | External configs work | Configs loaded |
| 3.3.3 | Add event bus connection | Connect to event-bus service | Connected |
| 3.3.4 | Create docker-compose entry | Integrate with stack | Stack runs |

### Dockerfile

```dockerfile
# containers/deepstream/Dockerfile

# CRITICAL: Use jetpack suffix for Jetson compatibility
FROM nvcr.io/nvidia/deepstream-l4t:7.0-samples-jetpack AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    libprotobuf-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY containers/deepstream/src/ ./src/
COPY shared/protocol/proto/ ./proto/

# Generate protobuf
RUN protoc --cpp_out=./src/ ./proto/*.proto

# Build
RUN mkdir build && cd build && \
    cmake .. && \
    make -j$(nproc)

# Runtime image
FROM nvcr.io/nvidia/deepstream-l4t:7.0-samples-jetpack

WORKDIR /app

# Copy built binary
COPY --from=builder /app/build/deepstream-yama /usr/local/bin/

# Copy default configs
COPY platform/jetson/deepstream-config/ /etc/yama/deepstream/

# Copy model files
COPY models/yolov8n.engine /models/
COPY models/labels.txt /models/

ENV YAMA_EVENT_SOCKET=/tmp/yama-event.sock
ENV YAMA_CONFIG_PATH=/etc/yama/deepstream/

CMD ["deepstream-yama"]
```

### Docker Compose Entry

```yaml
# docker-compose.yml

services:
  deepstream:
    build: ./containers/deepstream
    runtime: nvidia
    volumes:
      - /tmp/yama-event.sock:/tmp/yama-event.sock
      - ./config/deepstream:/etc/yama/deepstream:ro
      - ./models:/models:ro
    environment:
      - YAMA_EVENT_SOCKET=/tmp/yama-event.sock
    depends_on:
      - event-bus
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu, video]
```

### Verification

```bash
docker build -t yama-deepstream containers/deepstream/
docker run --runtime nvidia yama-deepstream
# Processes test streams
```

### Deliverable

`containers/deepstream/Dockerfile`

---

## Dependencies

- Phase 1 (Abstraction Layer) - for trait implementations (VideoProvider, DetectionProvider)
- Phase 4 (Triton) - for model files (optional, can use standalone TensorRT)

## Blocks

- Phase 5 (VLM Orchestration) - needs detection stream
- Phase 6 (Video Streaming) - needs encoded output
- Phase 8 (E2E Integration) - core component

---

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Decode latency | <5ms/frame | GStreamer probe |
| Detection latency | <10ms/frame | Triton metrics |
| Tracking latency | <2ms/frame | DeepStream probe |
| Streams supported | 4-8 @ 1080p30 | nvidia-smi |
| GPU utilization | <80% | nvidia-smi |

---

## Agent Review Additions (NVIDIA SME)

The following optimizations were identified by NVIDIA ecosystem review:

| ID | Task | Rationale | Priority |
|----|------|-----------|----------|
| 3.4.1 | Use INT8 quantization for YOLOv8 | ~2x faster, <2% accuracy loss on COCO | High |
| 3.4.2 | Direct TensorRT export (not ONNX→TRT) | Skip intermediate step, preserves optimizations | High |
| 3.4.3 | Add nvstreammux `sync-inputs: 1` | Prevents multi-stream frame drift | Critical |
| 3.4.4 | Add buffer pool configuration | Pre-allocate to avoid allocation spikes | Medium |
| 3.4.5 | Use DMA-BUF for FrameSample | Zero-copy GPU→GPU transfer (60x faster than CPU copy) | High |
| 3.4.6 | Fix container tag to `jetpack` | `multiarch` won't run on Jetson | Critical |

**INT8 Quantization:**
```python
# ~2x faster, minimal accuracy loss
from ultralytics import YOLO
model = YOLO('yolov8n.pt')
model.export(
    format='engine',
    int8=True,
    data='coco128.yaml',  # Calibration dataset
    workspace=8,  # GB - required for Orin
)
```

**Direct TensorRT Export:**
```python
# Skip ONNX intermediate - preserves optimizations:
model.export(format='engine')  # NOT: model.export(format='onnx') + trtexec
```

**DMA-BUF Zero-Copy Frame Passing:**
```cpp
// INSTEAD OF: Map→Encode JPEG→Copy to protobuf (30-50ms)
// USE: Pass DMA-BUF fd directly (0.5ms)
int dma_fd = NvBufSurfaceGetFd(surface, 0);
sample.set_dma_buf_fd(dma_fd);
sample.set_width(surface->surfaceList[0].width);
sample.set_height(surface->surfaceList[0].height);
// Consumer imports DMA-BUF directly into VLM preprocessing
```

**Performance Impact:**
| Approach | Latency | Memory |
|----------|---------|--------|
| CPU copy + JPEG | 30-50ms | 1920×1080×3 bytes copied |
| DMA-BUF zero-copy | 0.5ms | fd passing only |

---

## Checklist

### Milestone 3.1: Pipeline Setup
- [ ] 3.1.1 Create DeepStream app skeleton
- [ ] 3.1.2 Configure multi-source RTSP
- [ ] 3.1.3 Add YOLOv8n primary inference (INT8)
- [ ] 3.1.4 Configure NvDCF tracker
- [ ] 3.1.5 Add H.264 encode output

### Milestone 3.2: Metadata Extraction
- [ ] 3.2.1 Add probe to extract NvDsMeta
- [ ] 3.2.2 Convert to Protobuf
- [ ] 3.2.3 Publish to event bus
- [ ] 3.2.4 Add frame extraction probe (DMA-BUF)

### Milestone 3.3: Container
- [ ] 3.3.1 Create Dockerfile (use `jetpack` tag)
- [ ] 3.3.2 Add config volume mount
- [ ] 3.3.3 Add event bus connection
- [ ] 3.3.4 Create docker-compose entry

### Milestone 3.4: NVIDIA Optimizations (Critical)
- [ ] 3.4.1 Use INT8 quantization for YOLOv8 (~2x speedup)
- [ ] 3.4.2 Direct TensorRT export (skip ONNX intermediate)
- [ ] 3.4.3 Add nvstreammux `sync-inputs: 1` (prevents drift)
- [ ] 3.4.4 Add buffer pool configuration (reduce allocation spikes)
- [ ] 3.4.5 Implement DMA-BUF zero-copy frame passing
- [ ] 3.4.6 Verify container uses `jetpack` suffix (NOT `multiarch`)
