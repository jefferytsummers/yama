# Phase 4: Triton Model Serving

**Duration:** 9 days
**Goal:** Multi-model inference server with dynamic loading.

---

## Overview

This phase sets up Triton Inference Server on Jetson for serving multiple models (YOLOv8 detection, Qwen2.5-VL vision-language model). Triton provides dynamic batching, model management, and hot-swapping capabilities essential for the multi-stage VLM orchestration.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Triton Inference Server                                                │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Model Repository                                                │   │
│  │  /models/                                                        │   │
│  │  ├── yolov8n/                                                   │   │
│  │  │   ├── config.pbtxt                                           │   │
│  │  │   └── 1/model.engine                                         │   │
│  │  ├── qwen2_vl_7b/                                               │   │
│  │  │   ├── config.pbtxt                                           │   │
│  │  │   └── 1/model.engine                                         │   │
│  │  └── vila_2b/                                                   │   │
│  │      ├── config.pbtxt                                           │   │
│  │      └── 1/model.engine                                         │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                 │
│  │ gRPC Server  │  │ HTTP Server  │  │ Metrics      │                 │
│  │ :8001        │  │ :8000        │  │ :8002        │                 │
│  └──────────────┘  └──────────────┘  └──────────────┘                 │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Dynamic Batching  │  Model Control API  │  Memory Management   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Model Selection

| Model | Size | Orin AGX Performance | Use Case |
|-------|------|----------------------|----------|
| YOLOv8n | 6M params | <10ms/frame | Object detection |
| Qwen2.5-VL-7B | 7B params | 1-3 fps | General analysis |
| VILA-2.7B | 2.7B params | 5-8 fps | Fast scene description |
| Nemotron-Nano-VL-8B | 8B params | 1-2 fps | Document understanding |

---

## Milestone 4.1: Model Repository Setup

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 4.1.1 | Convert YOLOv8n to TensorRT | Build .engine file | Engine generated |
| 4.1.2 | Create YOLOv8 Triton config | config.pbtxt for YOLO | Config valid |
| 4.1.3 | Setup Qwen2.5-VL model | TensorRT-LLM or vLLM backend | Model loads |
| 4.1.4 | Create VLM Triton config | Multimodal input spec | Config valid |

### YOLOv8 Triton Config

> **⚠️ CRITICAL: Verify Output Tensor Names**
> YOLOv8 TensorRT output tensor names vary by export method. Use `polygraphy inspect model model.engine` to verify actual names before deployment.

```
# containers/triton/model_repository/yolov8n/config.pbtxt

name: "yolov8n"
platform: "tensorrt_plan"
max_batch_size: 8

input [
  {
    name: "images"
    data_type: TYPE_FP16
    format: FORMAT_NCHW
    dims: [ 3, 640, 640 ]
  }
]

# NOTE: Output names depend on export method. Options:
# - Ultralytics direct export: "output0" with dims [84, 8400]
# - End2End TensorRT: "num_detections", "detection_boxes", "detection_scores", "detection_classes"
# ALWAYS verify with: polygraphy inspect model yolov8n.engine --show-all
output [
  {
    name: "output0"
    data_type: TYPE_FP16
    dims: [ 84, 8400 ]  # YOLOv8 raw output: [xywh + conf + 80 classes, 8400 anchors]
  }
]

# Alternative for End2End export (with NMS baked in):
# output [
#   { name: "num_detections"    data_type: TYPE_INT32  dims: [ 1 ] }
#   { name: "detection_boxes"   data_type: TYPE_FP32   dims: [ -1, 4 ] }
#   { name: "detection_scores"  data_type: TYPE_FP32   dims: [ -1 ] }
#   { name: "detection_classes" data_type: TYPE_INT32  dims: [ -1 ] }
# ]

dynamic_batching {
  preferred_batch_size: [ 4, 8 ]
  max_queue_delay_microseconds: 5000
}

instance_group [
  {
    count: 1
    kind: KIND_GPU
    gpus: [ 0 ]
  }
]

optimization {
  execution_accelerators {
    gpu_execution_accelerator : [ {
      name : "tensorrt"
      parameters { key: "precision_mode" value: "INT8" }  # INT8 for 2x speedup
      parameters { key: "max_workspace_size_bytes" value: "8589934592" }  # 8GB for Orin
    }]
  }
}
```

### Verification: Inspect TensorRT Engine

```bash
# ALWAYS verify tensor names before deployment
pip install polygraphy
polygraphy inspect model yolov8n.engine --show-all

# Expected output for direct Ultralytics export:
# Input: images, shape: (batch, 3, 640, 640)
# Output: output0, shape: (batch, 84, 8400)
```

### Qwen2.5-VL Triton Config

```
# containers/triton/model_repository/qwen2_vl_7b/config.pbtxt

name: "qwen2_vl_7b"
backend: "tensorrtllm"  # CRITICAL: Use TensorRT-LLM, not Python (2-3x faster)
max_batch_size: 1
model_transaction_policy { decoupled: true }

input [
  {
    name: "image"
    data_type: TYPE_UINT8
    dims: [ -1, -1, 3 ]  # Variable size RGB
  },
  {
    name: "prompt"
    data_type: TYPE_STRING
    dims: [ 1 ]
  },
  {
    name: "max_tokens"
    data_type: TYPE_INT32
    dims: [ 1 ]
    optional: true
  },
  {
    name: "temperature"
    data_type: TYPE_FP32
    dims: [ 1 ]
    optional: true
  }
]

output [
  {
    name: "text"
    data_type: TYPE_STRING
    dims: [ 1 ]
  },
  {
    name: "tokens_generated"
    data_type: TYPE_INT32
    dims: [ 1 ]
  }
]

instance_group [
  {
    count: 1
    kind: KIND_GPU
    gpus: [ 0 ]
  }
]

# Model will be loaded on demand
model_warmup [
  {
    name: "warmup"
    batch_size: 1
    inputs: {
      key: "image"
      value: {
        data_type: TYPE_UINT8
        dims: [ 224, 224, 3 ]
        zero_data: true
      }
    }
    inputs: {
      key: "prompt"
      value: {
        data_type: TYPE_STRING
        dims: [ 1 ]
        input_data_file: "warmup_prompt.txt"
      }
    }
  }
]
```

### Model Conversion Scripts

```python
# scripts/convert_yolov8_tensorrt.py

import tensorrt as trt
from ultralytics import YOLO

def convert_yolov8_to_trt(
    model_path: str,
    output_path: str,
    fp16: bool = True,
    batch_size: int = 8,
):
    """Convert YOLOv8 to TensorRT engine for Triton."""
    model = YOLO(model_path)

    # Export to ONNX first
    onnx_path = model_path.replace('.pt', '.onnx')
    model.export(
        format='onnx',
        dynamic=False,
        batch=batch_size,
        imgsz=640,
    )

    # Convert ONNX to TensorRT
    logger = trt.Logger(trt.Logger.WARNING)
    builder = trt.Builder(logger)
    network = builder.create_network(
        1 << int(trt.NetworkDefinitionCreationFlag.EXPLICIT_BATCH)
    )
    parser = trt.OnnxParser(network, logger)

    with open(onnx_path, 'rb') as f:
        parser.parse(f.read())

    config = builder.create_builder_config()
    config.max_workspace_size = 8 << 30  # 8GB (CRITICAL: Orin requires 8GB for optimal INT8 tactics)

    if fp16:
        config.set_flag(trt.BuilderFlag.FP16)

    engine = builder.build_serialized_network(network, config)

    with open(output_path, 'wb') as f:
        f.write(engine)

    print(f"TensorRT engine saved to {output_path}")

if __name__ == "__main__":
    convert_yolov8_to_trt(
        "yolov8n.pt",
        "model_repository/yolov8n/1/model.engine"
    )
```

### Verification

```bash
tritonserver --model-repository=/models
curl localhost:8000/v2/models  # ["yolov8n", "qwen2_vl_7b"]
perf_analyzer -m yolov8n --input-data random
```

### Deliverable

`containers/triton/model_repository/`

---

## Milestone 4.2: Triton Rust Client

**Duration:** 4 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 4.2.1 | Create gRPC client wrapper | Connect to Triton | Connection works |
| 4.2.2 | Implement YOLOv8 inference | Send image, receive detections | Correct output |
| 4.2.3 | Implement VLM inference | Send image+prompt, receive text | Correct output |
| 4.2.4 | Add model management calls | Load/unload models | Models managed |

### Code: Triton Client

```rust
// platform/jetson/triton-client/src/lib.rs

use tonic::transport::Channel;
use inference_proto::grpc_inference_service_client::GrpcInferenceServiceClient;
use inference_proto::{
    ModelInferRequest, ModelInferResponse,
    InferTensorContents, InferInputTensor, InferRequestedOutputTensor,
};

pub struct TritonClient {
    client: GrpcInferenceServiceClient<Channel>,
    url: String,
}

impl TritonClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let channel = Channel::from_shared(url.to_string())?
            .connect()
            .await?;

        let client = GrpcInferenceServiceClient::new(channel);

        Ok(Self {
            client,
            url: url.to_string(),
        })
    }

    /// Run YOLOv8 inference.
    pub async fn infer_yolo(
        &mut self,
        images: &[ImageData],
    ) -> Result<Vec<Vec<Detection>>> {
        // Preprocess images to NCHW FP16
        let batch_size = images.len();
        let input_data = self.preprocess_yolo_batch(images)?;

        // Build request
        let request = ModelInferRequest {
            model_name: "yolov8n".to_string(),
            model_version: "1".to_string(),
            inputs: vec![
                InferInputTensor {
                    name: "images".to_string(),
                    datatype: "FP16".to_string(),
                    shape: vec![batch_size as i64, 3, 640, 640],
                    contents: Some(InferTensorContents {
                        fp16_contents: input_data,
                        ..Default::default()
                    }),
                },
            ],
            outputs: vec![
                InferRequestedOutputTensor {
                    name: "output0".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let response = self.client.model_infer(request).await?;

        // Postprocess to detections
        self.postprocess_yolo(&response.into_inner())
    }

    /// Run VLM inference.
    pub async fn infer_vlm(
        &mut self,
        image: &ImageData,
        prompt: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<VlmResponse> {
        let request = ModelInferRequest {
            model_name: "qwen2_vl_7b".to_string(),
            model_version: "1".to_string(),
            inputs: vec![
                InferInputTensor {
                    name: "image".to_string(),
                    datatype: "UINT8".to_string(),
                    shape: vec![image.height as i64, image.width as i64, 3],
                    contents: Some(InferTensorContents {
                        uint_contents: image.data.iter().map(|&b| b as u32).collect(),
                        ..Default::default()
                    }),
                },
                InferInputTensor {
                    name: "prompt".to_string(),
                    datatype: "BYTES".to_string(),
                    shape: vec![1],
                    contents: Some(InferTensorContents {
                        bytes_contents: vec![prompt.as_bytes().to_vec()],
                        ..Default::default()
                    }),
                },
            ],
            outputs: vec![
                InferRequestedOutputTensor {
                    name: "text".to_string(),
                    ..Default::default()
                },
                InferRequestedOutputTensor {
                    name: "tokens_generated".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let response = self.client.model_infer(request).await?;
        self.parse_vlm_response(&response.into_inner())
    }

    /// List loaded models.
    pub async fn list_models(&mut self) -> Result<Vec<ModelInfo>> {
        let response = self.client
            .repository_index(RepositoryIndexRequest::default())
            .await?;

        let models = response.into_inner().models.into_iter()
            .map(|m| ModelInfo {
                name: m.name,
                version: m.version,
                state: m.state,
            })
            .collect();

        Ok(models)
    }

    /// Load a model dynamically.
    pub async fn load_model(&mut self, model_name: &str) -> Result<()> {
        let request = RepositoryModelLoadRequest {
            model_name: model_name.to_string(),
            ..Default::default()
        };

        self.client.repository_model_load(request).await?;
        Ok(())
    }

    /// Unload a model to free memory.
    pub async fn unload_model(&mut self, model_name: &str) -> Result<()> {
        let request = RepositoryModelUnloadRequest {
            model_name: model_name.to_string(),
            ..Default::default()
        };

        self.client.repository_model_unload(request).await?;
        Ok(())
    }

    /// Get GPU memory usage.
    pub async fn get_gpu_memory(&mut self) -> Result<GpuMemory> {
        // Query via metrics endpoint
        let metrics = reqwest::get(&format!("{}:8002/metrics", self.url))
            .await?
            .text()
            .await?;

        self.parse_gpu_metrics(&metrics)
    }
}
```

### Verification

```bash
cargo test -p yama-triton-client -- --ignored
# Tests: infer_yolo, infer_vlm, load_model
```

### Deliverable

`platform/jetson/triton-client/`

---

## Milestone 4.3: Dynamic Model Loading

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 4.3.1 | Implement model switch handler | Protocol message works | Switch succeeds |
| 4.3.2 | Add model state tracking | Know which models loaded | State accurate |
| 4.3.3 | Memory-aware loading | Check GPU memory first | Memory checked |
| 4.3.4 | Add model switch API | HTTP/event bus interface | API works |

### Protocol: Model Management

```protobuf
// shared/protocol/proto/vlm.proto (extended)

// Request to switch active VLM model
message ModelSwitchRequest {
    string model_name = 1;      // Model to activate
    bool unload_current = 2;    // Free memory from current model
}

message ModelSwitchResponse {
    bool success = 1;
    string active_model = 2;
    uint64 memory_used_bytes = 3;
    string error_message = 4;
}

// Query available models
message ModelListRequest {}

message ModelListResponse {
    repeated ModelInfo models = 1;
}

message ModelInfo {
    string name = 1;
    string version = 2;
    ModelState state = 3;
    uint64 memory_bytes = 4;
    string model_type = 5;  // "vlm", "detector", "specialized"
}

enum ModelState {
    MODEL_STATE_UNKNOWN = 0;
    MODEL_STATE_UNLOADED = 1;
    MODEL_STATE_LOADING = 2;
    MODEL_STATE_READY = 3;
    MODEL_STATE_UNLOADING = 4;
}
```

### Code: Model Manager

```rust
// ai-server/src/triton/model_manager.rs

pub struct ModelManager {
    triton: TritonClient,
    loaded_models: RwLock<HashMap<String, ModelState>>,
    gpu_memory: RwLock<GpuMemory>,
    model_specs: HashMap<String, ModelSpec>,
}

impl ModelManager {
    pub async fn switch_model(
        &self,
        request: ModelSwitchRequest,
    ) -> Result<ModelSwitchResponse> {
        // Check if model exists
        let spec = self.model_specs.get(&request.model_name)
            .ok_or_else(|| anyhow!("Unknown model: {}", request.model_name))?;

        // Check memory
        let memory = self.gpu_memory.read().await;
        if memory.available < spec.memory_required {
            if request.unload_current {
                // Unload models to free memory
                self.free_memory(spec.memory_required).await?;
            } else {
                return Ok(ModelSwitchResponse {
                    success: false,
                    error_message: "Insufficient GPU memory".to_string(),
                    ..Default::default()
                });
            }
        }
        drop(memory);

        // Load new model
        self.triton.load_model(&request.model_name).await?;

        // Update state
        {
            let mut loaded = self.loaded_models.write().await;
            loaded.insert(request.model_name.clone(), ModelState::Ready);
        }

        // Get updated memory usage
        let memory = self.triton.get_gpu_memory().await?;

        Ok(ModelSwitchResponse {
            success: true,
            active_model: request.model_name,
            memory_used_bytes: memory.used,
            error_message: String::new(),
        })
    }

    async fn free_memory(&self, required: u64) -> Result<()> {
        let loaded = self.loaded_models.read().await;

        // Sort by priority (keep detectors, unload VLMs first)
        let mut candidates: Vec<_> = loaded.iter()
            .filter(|(_, state)| matches!(state, ModelState::Ready))
            .filter(|(name, _)| {
                self.model_specs.get(*name)
                    .map(|s| s.model_type == "vlm")
                    .unwrap_or(false)
            })
            .collect();

        drop(loaded);

        // Unload until we have enough memory
        let mut freed = 0u64;
        for (name, _) in candidates {
            if freed >= required {
                break;
            }

            let spec = &self.model_specs[name];
            self.triton.unload_model(name).await?;
            freed += spec.memory_required;

            let mut loaded = self.loaded_models.write().await;
            loaded.insert(name.clone(), ModelState::Unloaded);
        }

        Ok(())
    }
}
```

### Verification

```bash
curl -X POST localhost:8080/api/models/switch -d '{"model":"nemotron"}'
curl localhost:8080/api/models/active  # {"model":"nemotron"}
```

### Deliverable

`ai-server/src/triton/model_manager.rs`

---

## Dependencies

- Phase 1 (Abstraction Layer) - for AIServerProvider implementation
- Phase 3 (DeepStream) - optional, for detector model

## Blocks

- Phase 5 (VLM Orchestration) - needs model serving
- Phase 8 (E2E Integration) - core component

---

## Memory Budget (Jetson AGX Orin 64GB)

| Component | Allocation | Notes |
|-----------|------------|-------|
| YOLOv8n TensorRT | 500MB | Always loaded |
| Qwen2.5-VL-7B (Q4) | 8GB | Primary VLM |
| VILA-2.7B (Q4) | 3GB | Fast VLM |
| Triton overhead | 2GB | Scheduler, buffers |
| System reserve | 4GB | OS, display |
| **Available for additional models** | ~46GB | |

---

## Milestone 4.4: Tool-Reasoning Model Integration

**Duration:** 3 days

### Rationale

Tool-reasoning models are critical for agentic video workflows. The agent must reliably select and invoke the correct video analysis tools based on user queries. Research shows that Llama-3.1-8B (Groq-tuned) achieves 89% on the Berkeley Function Calling Leaderboard (BFCL), making it the best open-source option.

**Reference:** [NVIDIA Agentic Video Workflow Blueprint](https://developer.nvidia.com/blog/build-an-agentic-video-workflow-with-video-search-and-summarization/)

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 4.4.1 | Add Llama-3.1-8B (Groq) to repository | Download and convert model | Model loads in Triton |
| 4.4.2 | Configure tool-calling inference endpoint | TensorRT-LLM backend for Llama | Endpoint responds |
| 4.4.3 | Add BFCL-style evaluation | Test tool selection accuracy | >85% on video tools |
| 4.4.4 | Document model selection rationale | Architecture decision record | Doc complete |

### Model Candidates

| Model | BFCL Score | Memory (Q4) | Latency | Notes |
|-------|------------|-------------|---------|-------|
| Llama-3.1-8B (Groq) | 89.06% | 3.8GB | Fast | Best tool calling |
| NexusRaven-V2-13B | 82.01% | 5GB | Medium | Nested calls |
| Qwen2.5-VL-7B | ~75% | 5.7GB | Medium | Native vision |

### Triton Config: Tool-Calling Model

```
# containers/triton/model_repository/llama3_tool/config.pbtxt

name: "llama3_tool"
backend: "tensorrtllm"
max_batch_size: 1
model_transaction_policy { decoupled: true }

input [
  {
    name: "input_ids"
    data_type: TYPE_INT32
    dims: [ -1 ]
  },
  {
    name: "tools"
    data_type: TYPE_STRING
    dims: [ -1 ]
    optional: true
  }
]

output [
  {
    name: "output_ids"
    data_type: TYPE_INT32
    dims: [ -1 ]
  },
  {
    name: "tool_calls"
    data_type: TYPE_STRING
    dims: [ -1 ]
  }
]

instance_group [
  {
    count: 1
    kind: KIND_GPU
    gpus: [ 0 ]
  }
]
```

---

## Milestone 4.5: Jetson Thor Optimization

**Duration:** 2 days

### Rationale

Jetson Thor (Blackwell, 128GB, FP4 support) enables running 70B+ models for complex reasoning. FP4 quantization doubles effective model capacity compared to FP8.

**Reference:** [Jetson Thor Technical Blog](https://developer.nvidia.com/blog/introducing-nvidia-jetson-thor-the-ultimate-platform-for-physical-ai/)

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 4.5.1 | Enable FP4 quantization path | TensorRT-LLM FP4 support | FP4 models load |
| 4.5.2 | Configure MIG for mixed workloads | Partition GPU for inference | MIG partitions work |
| 4.5.3 | Update memory budget for 128GB | Revise model loading limits | Budget documented |
| 4.5.4 | Benchmark 70B model feasibility | Test Llama-3.1-70B latency | Latency measured |

### Jetson Thor Memory Budget

| Component | Allocation | Notes |
|-----------|------------|-------|
| DeepStream pipeline | 8GB | Scaled for 10+ streams |
| YOLOv8n TensorRT (INT8) | 250MB | Always loaded |
| Llama-3.1-70B (FP4) | ~27GB | Complex reasoning |
| Llama-3.1-8B (Q4) | 3.8GB | Tool calling |
| Qwen2.5-VL-7B (Q4) | 5.7GB | Vision queries |
| VILA-2.7B (Q4) | 2.4GB | Fast characterization |
| Triton overhead | 4GB | Larger for 70B |
| System reserve | 8GB | OS, headroom |
| **Available for additional** | ~68GB | |

---

## Checklist

- [ ] 4.1.1 Convert YOLOv8n to TensorRT
- [ ] 4.1.2 Create YOLOv8 Triton config
- [ ] 4.1.3 Setup Qwen2.5-VL model
- [ ] 4.1.4 Create VLM Triton config
- [ ] 4.2.1 Create gRPC client wrapper
- [ ] 4.2.2 Implement YOLOv8 inference
- [ ] 4.2.3 Implement VLM inference
- [ ] 4.2.4 Add model management calls
- [ ] 4.3.1 Implement model switch handler
- [ ] 4.3.2 Add model state tracking
- [ ] 4.3.3 Memory-aware loading
- [ ] 4.3.4 Add model switch API
- [ ] 4.4.1 Add Llama-3.1-8B (Groq variant) to model repository
- [ ] 4.4.2 Configure tool-calling inference endpoint
- [ ] 4.4.3 Add BFCL-style evaluation for video tool schema
- [ ] 4.4.4 Document model selection rationale
- [ ] 4.5.1 Enable FP4 quantization path for Jetson Thor
- [ ] 4.5.2 Configure MIG for mixed workloads
- [ ] 4.5.3 Update memory budget for 128GB platform
- [ ] 4.5.4 Benchmark 70B model feasibility for complex queries
