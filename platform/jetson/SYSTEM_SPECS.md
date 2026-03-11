# Thor System Specifications

**Machine:** NVIDIA Jetson AGX Thor Developer Kit
**Recorded:** 2026-03-11
**Yama Target:** VLM Inference with LLaVA-1.5-7B

## Hardware

| Component | Value |
|-----------|-------|
| Platform | Jetson AGX Thor |
| Memory | 128GB unified (122GB available) |
| CPU Cores | 14 (ARM) |
| GPU | NVIDIA Thor (Blackwell architecture) |
| GPU Compute | SM 100 |

## Software Stack

| Component | Version |
|-----------|---------|
| L4T | R38.2.2 |
| CUDA | 13.0 |
| Driver | 580.00 |
| OS | Ubuntu 24.04.4 LTS (Noble) |
| Kernel | 6.8.12-tegra |
| Python | 3.12.3 |
| Docker | 29.3.0 |
| Docker Runtime | nvidia (default) |

## VLM Configuration

| Setting | Value |
|---------|-------|
| Model | LLaVA-1.5-7B |
| Precision | FP16 (memory allows full precision) |
| Max Batch Size | 256 |
| Max Tokens | 8192 |
| KV Cache | 90% free GPU memory |
| Shared Memory | 16GB |

## Notes

- L4T R38 with CUDA 13.0 is newer than JetPack 6.2 (L4T R36.x, CUDA 12.6)
- This is JetPack 7.x (Thor preview release)
- 128GB unified memory eliminates memory constraints for 7B models
- Can run FP16 instead of INT4 quantization
- No PCIe bottleneck - unified memory architecture

## Container Strategy

Using custom container based on `vila:r38.4.arm64-sbsa-cu130-24.04-pytorch_2.10`:
- PyTorch 2.10 with SM 110 (Thor/Blackwell) support
- HuggingFace Transformers for LLaVA inference
- Flask-based HTTP API server

**Working Configuration (Verified 2026-03-11):**
- Model: `llava-hf/llava-1.5-7b-hf` (HuggingFace format)
- Load time: ~5 seconds
- GPU Memory: 13.16 GB
- Throughput: ~8 tokens/second

```bash
# Build VLM container (extends vila base with transformers)
cd /home/jeff/source/repos/yama/platform/jetson
./scripts/build_vlm_container.sh

# Download HF-compatible LLaVA model
docker run --rm --runtime nvidia \
  -v /home/jeff/data/models:/data/models \
  yama-vlm:r38 python3 -c "
from huggingface_hub import snapshot_download
snapshot_download('llava-hf/llava-1.5-7b-hf', local_dir='/data/models/llava/llava-1.5-7b-hf')
"

# Start inference server
docker-compose -f docker-compose.triton.yml up -d vlm

# Test API
curl http://localhost:8000/v2/health/ready
```

## Jetson Containers Location

```
/home/jeff/source/repos/jetson-containers
```

## API Endpoints (when running)

| Endpoint | URL |
|----------|-----|
| Health Ready | GET http://localhost:8000/v2/health/ready |
| Health Live | GET http://localhost:8000/v2/health/live |
| Inference | POST http://localhost:8000/v2/models/llava/infer |
| Metrics | GET http://localhost:8000/metrics |

## Metrics Available

The `/metrics` endpoint provides Prometheus-compatible metrics:

| Metric | Type | Description |
|--------|------|-------------|
| `vlm_gpu_memory_allocated_bytes` | gauge | GPU memory currently allocated |
| `vlm_gpu_memory_reserved_bytes` | gauge | GPU memory reserved by PyTorch |
| `vlm_gpu_memory_total_bytes` | gauge | Total GPU memory (128GB unified) |
| `vlm_model_loaded` | gauge | 1 if model loaded, 0 otherwise |
| `vlm_model_load_time_seconds` | gauge | Time to load the model |
| `vlm_inference_requests_total` | counter | Total inference requests |
| `vlm_inference_success_total` | counter | Successful inferences |
| `vlm_inference_failed_total` | counter | Failed inferences |
| `vlm_tokens_generated_total` | counter | Total tokens generated |
| `vlm_inference_latency_avg_ms` | gauge | Average inference latency |
| `vlm_tokens_per_second_avg` | gauge | Average token generation rate |

## Testing

Run the integration test suite:
```bash
python3 scripts/test_vlm_api.py
```

Expected output: 6/6 tests passed
