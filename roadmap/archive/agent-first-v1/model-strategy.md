# Model Strategy

## Overview

This document defines the model selection strategy for Yama's Content Analyst workflow. The focus is Mac-local inference using GGUF models with Metal acceleration, supporting video indexing, transcription, semantic search, and visual analysis.

---

## Model Categories

| Category | Purpose | Models |
|----------|---------|--------|
| Vision-Language (VLM) | Frame analysis, scene description | LLaVA, Qwen2-VL, MiniCPM-V |
| Speech-to-Text | Audio transcription | Whisper (tiny → large-v3) |
| Visual Embedding | Semantic image search | CLIP ViT-B/32 |
| Text Embedding | Semantic text search | all-MiniLM-L6-v2 |

---

## Vision-Language Models (VLM)

### Model Comparison

| Model | Size | Memory (Q4) | Speed | Quality | Use Case |
|-------|------|-------------|-------|---------|----------|
| LLaVA-1.5-7B | 7B | 4GB | Fast | Good | General analysis |
| Qwen2-VL-7B | 7B | 5GB | Medium | Better | Detailed understanding |
| MiniCPM-V-2.6 | 2.6B | 2GB | Very Fast | Good | Quick queries, batch |
| LLaVA-13B | 13B | 8GB | Slow | Best | Complex reasoning |

### Selection by Use Case

| Use Case | Primary | Fallback | Rationale |
|----------|---------|----------|-----------|
| Batch indexing | MiniCPM-V-2.6 | LLaVA-1.5-7B | Speed for bulk processing |
| Interactive queries | LLaVA-1.5-7B | Qwen2-VL-7B | Balance of speed/quality |
| Detailed analysis | Qwen2-VL-7B | LLaVA-13B | Best visual understanding |
| Memory-constrained | MiniCPM-V-2.6 | - | 2GB fits any Mac |

### Recommended Default

**LLaVA-1.5-7B Q4_K_M** - Good balance of speed, quality, and memory usage. Fits comfortably on 16GB Mac with room for other models.

### Download Commands

```bash
# Create models directory
mkdir -p ~/.cache/yama/models

# LLaVA-1.5-7B Q4_K_M (4.4GB)
huggingface-cli download mys/ggml_llava-v1.5-7b \
  ggml-model-q4_k.gguf mmproj-model-f16.gguf \
  --local-dir ~/.cache/yama/models/

# MiniCPM-V-2.6 (fast, 2GB)
huggingface-cli download openbmb/MiniCPM-V-2_6-gguf \
  minicpm-v-2_6-q4_k_m.gguf \
  --local-dir ~/.cache/yama/models/
```

---

## Speech-to-Text (Whisper)

### Model Comparison

| Model | Parameters | Memory | Speed Factor | WER (English) | Use Case |
|-------|------------|--------|--------------|---------------|----------|
| tiny.en | 39M | 200MB | 32x | 9.9% | Quick indexing |
| base.en | 74M | 400MB | 16x | 6.7% | Default balance |
| small.en | 244M | 1GB | 6x | 5.0% | Important content |
| medium.en | 769M | 3GB | 2x | 4.3% | High accuracy |
| large-v3 | 1.5B | 6GB | 1x | 3.0% | Best quality |

### Selection by Use Case

| Use Case | Model | Rationale |
|----------|-------|-----------|
| Fast bulk indexing | tiny.en | 32x realtime, good enough for search |
| Default | base.en | 16x realtime, 6.7% WER |
| Meeting recordings | small.en | Better accuracy for speech |
| Professional content | medium.en | Low error rate |

### Recommended Default

**base.en** - Best balance of speed (16x realtime) and accuracy (6.7% WER). Memory efficient at 400MB.

### Download Commands

```bash
# Download Whisper models (whisper.cpp format)
mkdir -p ~/.cache/yama/models

# tiny.en (39MB) - fastest
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin \
  -o ~/.cache/yama/models/ggml-tiny.en.bin

# base.en (142MB) - recommended default
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin \
  -o ~/.cache/yama/models/ggml-base.en.bin

# small.en (466MB) - higher quality
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin \
  -o ~/.cache/yama/models/ggml-small.en.bin
```

---

## Visual Embeddings (CLIP)

### Model Selection

| Model | Embedding Dim | Memory | Speed | Quality |
|-------|---------------|--------|-------|---------|
| ViT-B/32 | 512 | 350MB | Fast | Good |
| ViT-B/16 | 512 | 350MB | Medium | Better |
| ViT-L/14 | 768 | 900MB | Slow | Best |

### Recommended Default

**ViT-B/32** - Fast enough for batch indexing, good embedding quality. 512-dimensional embeddings are efficient for SQLite storage.

### Key Properties

- **Text-to-Image matching**: Embed queries as text, compare to image embeddings
- **Zero-shot classification**: No training required for new concepts
- **Batch processing**: Efficient GPU acceleration via Metal

### Integration

```rust
// CLIP embeddings stored as BLOB in SQLite
// 512 dimensions × 4 bytes = 2KB per embedding
CREATE TABLE keyframes (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    embedding BLOB,  -- 512-dim float32
    FOREIGN KEY (video_id) REFERENCES videos(id)
);

// Cosine similarity for search
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}
```

---

## Text Embeddings

For semantic search over transcripts and VLM-generated descriptions.

### Model Selection

| Model | Dimensions | Memory | Use Case |
|-------|------------|--------|----------|
| all-MiniLM-L6-v2 | 384 | 90MB | Fast, general |
| all-mpnet-base-v2 | 768 | 420MB | Higher quality |
| e5-small-v2 | 384 | 130MB | Cross-lingual |

### Recommended Default

**all-MiniLM-L6-v2** - Compact (90MB), fast, good quality for transcript search.

---

## Memory Budget (Mac 16GB)

### Simultaneous Load

| Component | Model | Memory | Status |
|-----------|-------|--------|--------|
| VLM | LLaVA-1.5-7B Q4 | 4.5GB | On-demand |
| Vision encoder | CLIP ViT-B/32 | 350MB | Always loaded |
| Whisper | base.en | 400MB | On-demand |
| Text embeddings | MiniLM-L6 | 90MB | Always loaded |
| SQLite + indexes | - | 500MB | Always |
| KV cache | - | 500MB | During inference |
| System reserve | macOS + apps | 4GB | Reserved |
| **Total Used** | | ~10.3GB | Peak |
| **Headroom** | | ~5.7GB | Available |

### Memory Management

```rust
pub struct ModelManager {
    // Always loaded (small)
    clip_encoder: ClipEncoder,
    text_embedder: TextEmbedder,

    // Loaded on-demand
    vlm: Option<VlmBackend>,
    whisper: Option<WhisperModel>,

    // Auto-unload after timeout
    idle_timeout: Duration,
}

impl ModelManager {
    pub async fn get_vlm(&mut self) -> Result<&mut VlmBackend> {
        if self.vlm.is_none() {
            // Unload Whisper if loaded (share memory)
            self.unload_whisper();
            self.vlm = Some(VlmBackend::load(&self.vlm_path)?);
        }
        Ok(self.vlm.as_mut().unwrap())
    }
}
```

---

## Inference Backends

### Mac (Apple Silicon)

| Backend | Models | Acceleration |
|---------|--------|--------------|
| llama.cpp | GGUF VLMs | Metal MPS |
| whisper.cpp | Whisper | Metal MPS |
| mlx-clip | CLIP | Metal MPS |
| sentence-transformers | Text embeddings | CPU (fast enough) |

### Configuration

```toml
[inference]
# VLM settings
vlm_model = "~/.cache/yama/models/llava-1.5-7b-q4_k.gguf"
vlm_context_size = 4096
vlm_gpu_layers = -1  # All layers on GPU

# Whisper settings
whisper_model = "~/.cache/yama/models/ggml-base.en.bin"
whisper_language = "en"

# CLIP settings
clip_model = "ViT-B/32"
clip_batch_size = 32

# Memory management
idle_unload_seconds = 300  # 5 minutes
max_concurrent_models = 2
```

---

## Quantization Guide

### GGUF Quantization Types

| Type | Bits | Quality Loss | Memory Savings | Recommended |
|------|------|--------------|----------------|-------------|
| Q4_0 | 4 | Moderate | 75% | Budget devices |
| Q4_K_M | 4 | Low | 75% | **Default** |
| Q5_K_M | 5 | Very Low | 68% | Quality focus |
| Q8_0 | 8 | Minimal | 50% | High quality |
| F16 | 16 | None | 0% | Development |

### Recommendation

**Q4_K_M** for all production VLM deployments. Best balance of size reduction and quality preservation.

---

## Benchmarks

### VLM Inference (LLaVA-1.5-7B Q4_K_M)

| Device | Prompt Eval | Generation | Memory |
|--------|-------------|------------|--------|
| M1 Pro 16GB | 45 t/s | 18 t/s | 4.5GB |
| M2 Max 32GB | 65 t/s | 28 t/s | 4.5GB |
| M3 Max 48GB | 80 t/s | 35 t/s | 4.5GB |

### Whisper Transcription (base.en)

| Device | Realtime Factor | Memory |
|--------|-----------------|--------|
| M1 Pro 16GB | 16x | 400MB |
| M2 Max 32GB | 22x | 400MB |
| M3 Max 48GB | 28x | 400MB |

### CLIP Embedding (ViT-B/32)

| Device | Images/sec | Memory |
|--------|------------|--------|
| M1 Pro 16GB | 85 | 350MB |
| M2 Max 32GB | 120 | 350MB |
| M3 Max 48GB | 150 | 350MB |

---

## Reference Links

### Vision-Language Models
- [LLaVA Project](https://llava-vl.github.io/)
- [Qwen2-VL](https://github.com/QwenLM/Qwen2-VL)
- [MiniCPM-V](https://github.com/OpenBMB/MiniCPM-V)
- [llama.cpp VLM Support](https://github.com/ggerganov/llama.cpp/discussions)

### Speech Recognition
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp)
- [OpenAI Whisper](https://github.com/openai/whisper)
- [whisper-rs (Rust bindings)](https://github.com/tazz4843/whisper-rs)

### Embeddings
- [OpenAI CLIP](https://github.com/openai/CLIP)
- [Sentence Transformers](https://www.sbert.net/)
- [MLX Examples](https://github.com/ml-explore/mlx-examples)

### Mac Inference
- [MLX Framework](https://github.com/ml-explore/mlx)
- [llama.cpp Metal](https://github.com/ggerganov/llama.cpp/blob/master/docs/metal.md)

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-03 | Mac-first focus | Content Analyst persona targets desktop users |
| 2025-03 | LLaVA-1.5-7B default VLM | Balance of speed/quality/memory |
| 2025-03 | Whisper base.en default | 16x realtime, 6.7% WER |
| 2025-03 | CLIP ViT-B/32 for embeddings | Fast, 512-dim efficient |
| 2025-03 | Q4_K_M quantization | Best quality/size tradeoff |
| 2025-03 | On-demand model loading | Share memory between VLM/Whisper |
