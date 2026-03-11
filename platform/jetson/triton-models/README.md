# Triton Model Repository for Jetson

This directory contains the Triton Inference Server model repository for VLM inference on NVIDIA Jetson.

## Directory Structure

```
triton-models/
├── README.md
└── qwen2_5_vl/
    ├── config.pbtxt      # Model configuration
    └── 1/                # Model version 1
        └── [engine files]
```

## Building TensorRT-LLM Engine

### Prerequisites

1. NVIDIA Jetson Orin (AGX, NX, or Nano)
2. JetPack 6.0+ with TensorRT 8.6+
3. At least 16GB unified memory (8GB minimum for 3B model)
4. Docker with NVIDIA Container Toolkit

### Build Steps

1. **Download the model:**
   ```bash
   huggingface-cli download Qwen/Qwen2.5-VL-7B-Instruct --local-dir ./qwen2_5_vl_hf
   ```

2. **Convert to TensorRT-LLM format:**
   ```bash
   docker run --rm --gpus all \
     -v $(pwd):/workspace \
     nvcr.io/nvidia/pytorch:24.01-py3 \
     python3 /opt/tensorrt_llm/examples/qwen2/convert_checkpoint.py \
       --model_dir /workspace/qwen2_5_vl_hf \
       --output_dir /workspace/qwen2_5_vl_ckpt \
       --dtype float16
   ```

3. **Build TensorRT engine:**
   ```bash
   docker run --rm --gpus all \
     -v $(pwd):/workspace \
     nvcr.io/nvidia/tensorrt:24.01-py3 \
     trtllm-build \
       --checkpoint_dir /workspace/qwen2_5_vl_ckpt \
       --output_dir /workspace/triton-models/qwen2_5_vl/1 \
       --max_batch_size 1 \
       --max_input_len 2048 \
       --max_output_len 512 \
       --gpt_attention_plugin float16 \
       --gemm_plugin float16
   ```

4. **Start Triton:**
   ```bash
   docker-compose -f docker-compose.triton.yml up -d
   ```

## Model Variants

| Model | Size | Memory | Throughput |
|-------|------|--------|------------|
| Qwen2.5-VL-3B | 3B params | ~8GB | ~30 tok/s |
| Qwen2.5-VL-7B | 7B params | ~16GB | ~15 tok/s |

For Jetson Orin NX (8GB), use the 3B model with INT8 quantization.

## Testing

```bash
# Check server health
curl http://localhost:8000/v2/health/ready

# Check model status
curl http://localhost:8000/v2/models/qwen2_5_vl

# Run inference
curl -X POST http://localhost:8000/v2/models/qwen2_5_vl/infer \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": [
      {"name": "prompt", "shape": [1], "datatype": "BYTES", "data": ["Describe this image"]},
      {"name": "image", "shape": [3, 448, 448], "datatype": "UINT8", "data": [...]}
    ]
  }'
```

## Troubleshooting

### Model not loading
- Check GPU memory: `tegrastats`
- Verify engine compatibility: `trtexec --loadEngine=model.engine`

### Slow inference
- Enable TensorRT timing cache
- Use FP16 instead of FP32
- Reduce max_output_len if possible

### Out of memory
- Use smaller model (3B vs 7B)
- Enable INT8 quantization
- Reduce batch size to 1
