# VLM Inference on Jetson AGX Thor

Research findings and implementation roadmap for production VLM inference.

## Hardware Specifications

| Component | Specification |
|-----------|---------------|
| Platform | Jetson AGX Thor (DRIVE Thor SoC) |
| Memory | 128GB unified (CPU+GPU shared) |
| GPU | Blackwell architecture (SM 100) |
| CUDA | 13.0 |
| L4T | R38.2.2 |
| JetPack | 7.x |

## Current State

**Implementation:** Flask + HuggingFace Transformers
**Model:** LLaVA-1.5-7B-hf
**Performance:** ~6-8 tokens/second
**GPU Memory:** ~13 GB

This is a development baseline, not production-ready.

---

## Critical Finding: TensorRT-LLM NOT Supported

**TensorRT-LLM does NOT support Jetson platforms**, including Thor.

From NVIDIA documentation:
> "TensorRT-LLM is optimized for NVIDIA data center GPUs (A100, H100, etc.) and is not compatible with Jetson's unified memory architecture."

### Why It Doesn't Work

1. **Architecture mismatch** - TensorRT-LLM targets discrete GPUs with separate VRAM
2. **CUDA compute capability** - SM 100 (Thor) not in TensorRT-LLM support matrix
3. **Memory model** - Unified memory requires different optimization strategies
4. **Kernel incompatibility** - Custom CUDA kernels compiled for datacenter GPUs

---

## Recommended Solutions

### Path A: Triton Inference Server + vLLM (Recommended Start)

**Why:** Standard NVIDIA production pattern, existing code support, easier iteration.

```
┌─────────────────────────────────────────────────────┐
│  Triton Inference Server (CUDA 13 build)            │
├─────────────────────────────────────────────────────┤
│  vLLM Backend                                       │
│  ├─ PagedAttention (efficient KV cache)             │
│  ├─ Continuous batching                             │
│  └─ OpenAI-compatible API                           │
├─────────────────────────────────────────────────────┤
│  Qwen2.5-VL-7B (AWQ INT4 quantization)             │
└─────────────────────────────────────────────────────┘
```

**Expected Performance:** 40-50 tokens/second (6-8x improvement)

**Setup Requirements:**
1. Custom Triton build for CUDA 13.0 (or use community container)
2. vLLM with Jetson-compatible wheels
3. AWQ quantization for memory efficiency

**Advantages:**
- Existing `TritonVlmBackend` in codebase
- Standard NVIDIA deployment pattern
- Prometheus metrics built-in
- Dynamic batching
- OpenAI-compatible API

### Path B: TensorRT Edge-LLM (Maximum Performance)

**Why:** Purpose-built for Jetson/DRIVE, native Thor support, best performance.

```
┌─────────────────────────────────────────────────────┐
│  TensorRT Edge-LLM (C++ Runtime)                    │
├─────────────────────────────────────────────────────┤
│  NVFP4 Quantization (native Thor support)           │
│  ├─ 4x memory reduction                             │
│  ├─ ~98% accuracy retention                         │
│  └─ Hardware-accelerated on SM 100                  │
├─────────────────────────────────────────────────────┤
│  Qwen2.5-VL-7B-Instruct                            │
└─────────────────────────────────────────────────────┘
```

**Expected Performance:** 60-80 tokens/second (10-13x improvement)

**Setup Requirements:**
1. TensorRT Edge-LLM SDK from NVIDIA
2. C++ inference wrapper (not Python)
3. Model export with NVFP4 quantization

**Advantages:**
- Maximum performance on Thor hardware
- Native NVFP4 quantization support
- Optimized for unified memory
- Lower latency

**Disadvantages:**
- C++ development required
- Less flexible than Python ecosystem
- Newer/less documented

---

## Model Recommendation: Qwen2.5-VL-7B

| Feature | LLaVA-1.5-7B | Qwen2.5-VL-7B |
|---------|--------------|---------------|
| Video understanding | No (image only) | Native (multi-frame) |
| Quantization | GPTQ/AWQ | NVFP4 native |
| Context length | 2K | 32K |
| Architecture | LLaMA + CLIP | Qwen2 + ViT |
| Performance | Baseline | ~30% faster |

**Why Qwen2.5-VL:**
- Native video understanding (doesn't require frame-by-frame)
- Better quantization support for Thor
- Longer context for video summaries
- Active development from Alibaba

---

## Quantization Options

### NVFP4 (Recommended for Thor)

- **Memory:** 4x reduction (7B model: ~14GB → ~3.5GB)
- **Accuracy:** ~98% of FP16
- **Hardware:** Native SM 100 support
- **Availability:** TensorRT Edge-LLM

### AWQ INT4 (Alternative)

- **Memory:** 4x reduction
- **Accuracy:** ~97% of FP16
- **Hardware:** Software implementation
- **Availability:** vLLM, HuggingFace

### GPTQ INT4

- **Memory:** 4x reduction
- **Accuracy:** ~96% of FP16
- **Hardware:** Software implementation
- **Availability:** Widely supported

---

## Video Indexing Architecture

**Philosophy:** "Index once, query forever" - run expensive inference once, cache results for cheap retrieval.

### Pipeline

```
┌──────────────────────────────────────────────────────────────┐
│  Video File                                                  │
└─────────────────────────┬────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  Frame Extraction (FFmpeg/GStreamer)                         │
│  ├─ Scene change detection                                   │
│  ├─ I-frame extraction                                       │
│  ├─ Interval sampling (1 fps fallback)                       │
│  └─ Motion detection for action sequences                    │
└─────────────────────────┬────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  VLM Inference (batch processing)                            │
│  ├─ Per-frame descriptions                                   │
│  ├─ Scene summaries                                          │
│  └─ Content classification                                   │
└─────────────────────────┬────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  Storage (hybrid)                                            │
│  ├─ SQLite: metadata, embeddings, search index               │
│  ├─ Sidecar JSON: per-video analysis                         │
│  └─ Filesystem: thumbnails, extracted frames                 │
└─────────────────────────┬────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  Query Interface                                             │
│  ├─ Natural language search                                  │
│  ├─ Timeline navigation                                      │
│  └─ Event filtering                                          │
└──────────────────────────────────────────────────────────────┘
```

### Frame Selection Strategies

| Strategy | Use Case | Frames/Min |
|----------|----------|------------|
| Scene change | Narrative content | 2-10 |
| Fixed interval | Surveillance | 1-2 |
| I-frame only | Fast indexing | 0.5-2 |
| Motion detection | Action sequences | Variable |

### Storage Schema

```sql
-- Videos table
CREATE TABLE videos (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    duration_ms INTEGER,
    frame_count INTEGER,
    indexed_at TIMESTAMP,
    summary TEXT,
    status TEXT  -- pending, indexing, complete, failed
);

-- Frames table
CREATE TABLE frames (
    id TEXT PRIMARY KEY,
    video_id TEXT REFERENCES videos(id),
    timestamp_ms INTEGER,
    description TEXT,
    embedding BLOB,  -- For semantic search
    thumbnail_path TEXT
);

-- Annotations table (future: YOLO, pose, etc.)
CREATE TABLE annotations (
    id TEXT PRIMARY KEY,
    frame_id TEXT REFERENCES frames(id),
    model TEXT,
    data JSON
);
```

---

## API Design

### Job-based Inference

```
POST /api/inference/videos
  → Create indexing job
  → Returns job_id

GET /api/inference/jobs/{job_id}
  → Job status and progress

GET /api/inference/jobs/{job_id}/stream
  → SSE stream of frame results

GET /api/videos/{video_id}/frames
  → Retrieved indexed frames

GET /api/videos/{video_id}/search?q=...
  → Semantic search within video
```

### SSE Events

```
event: status     → { "status": "extracting" | "inferring" | "completed" }
event: frame      → { "frame": 42, "total": 100, "text": "..." }
event: complete   → { "results_count": 100 }
event: error      → { "error": "..." }
```

---

## Performance Targets

| Metric | Current | Target (Triton+vLLM) | Target (Edge-LLM) |
|--------|---------|----------------------|-------------------|
| Tokens/sec | 6-8 | 40-50 | 60-80 |
| First token latency | ~2s | ~500ms | ~300ms |
| Memory usage | 13GB | 8GB (AWQ) | 4GB (NVFP4) |
| Frames/min (video) | 3-4 | 20-30 | 40-50 |

---

## Implementation Roadmap

### Phase 1: Triton + vLLM Setup (Current Priority)
1. Build/obtain Triton container for CUDA 13
2. Configure vLLM backend
3. Deploy Qwen2.5-VL-7B with AWQ
4. Update `TritonVlmBackend` client
5. Benchmark and validate

### Phase 2: Video Indexing Pipeline
1. Implement frame extraction strategies
2. Create SQLite storage schema
3. Build batch inference pipeline
4. Add SSE streaming for progress
5. Implement search API

### Phase 3: Production Optimization
1. Evaluate TensorRT Edge-LLM for performance
2. Add NVFP4 quantization if using Edge-LLM
3. Implement caching layer
4. Add multi-model support (YOLO, pose)

---

## References

- [TensorRT Edge-LLM](https://github.com/NVIDIA/TensorRT-Edge-LLM)
- [vLLM on Jetson](https://docs.vllm.ai/en/latest/getting_started/jetson-installation.html)
- [Triton Inference Server](https://github.com/triton-inference-server/server)
- [Qwen2.5-VL](https://github.com/QwenLM/Qwen2.5-VL)
- [NVFP4 Quantization](https://developer.nvidia.com/blog/accelerating-llm-inference-with-nvfp4-quantization/)

---

## Files

| File | Purpose |
|------|---------|
| `scripts/vlm_server.py` | Current Flask VLM server (dev baseline) |
| `docker-compose.triton.yml` | Container orchestration |
| `Dockerfile.vlm` | VLM container build |
| `host/src/inference/` | Rust VLM client code |

---

*Last updated: 2026-03-11*
