# Phase 4: Inference

**Duration:** 9 days
**Goal:** Local VLM inference using GGUF models with Metal acceleration, plus model management UI.

---

## Overview

This phase implements local vision-language model (VLM) inference for detailed video frame analysis. Unlike the archived Jetson roadmap (which used Triton), this phase focuses entirely on Mac-local inference using GGUF models and Metal acceleration.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  USER QUERY                                                             │
│  "What is the person doing in this frame?"                             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  VLM INFERENCE ENGINE                                                   │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  Image Preprocessing                                              │  │
│  │  • Resize to model input size (224x224 or 336x336)               │  │
│  │  • Normalize pixel values                                         │  │
│  │  • Encode to model-specific format                               │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                    │                                    │
│                                    ▼                                    │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  Model Backend (one of):                                          │  │
│  │                                                                   │  │
│  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐     │  │
│  │  │ llama.cpp      │  │ MLX-VLM        │  │ Ollama         │     │  │
│  │  │ (GGUF models)  │  │ (Apple native) │  │ (subprocess)   │     │  │
│  │  │                │  │                │  │                │     │  │
│  │  │ • LLaVA        │  │ • LLaVA-1.5    │  │ • llava        │     │  │
│  │  │ • Qwen2-VL     │  │ • Qwen2-VL     │  │ • bakllava     │     │  │
│  │  │ • MiniCPM-V    │  │                │  │                │     │  │
│  │  └────────────────┘  └────────────────┘  └────────────────┘     │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  ANALYSIS OUTPUT                                                        │
│  "The person is writing on a whiteboard with a blue marker."           │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Model Selection

| Model | Size | Memory (Q4) | Speed | Quality | Use Case |
|-------|------|-------------|-------|---------|----------|
| LLaVA-1.5-7B | 7B | 4GB | Fast | Good | General analysis |
| Qwen2-VL-7B | 7B | 5GB | Medium | Better | Detailed understanding |
| MiniCPM-V-2.6 | 2.6B | 2GB | Very Fast | Good | Quick queries |
| LLaVA-13B | 13B | 8GB | Slow | Best | Complex reasoning |

**Recommendation:**
- **Default:** LLaVA-1.5-7B Q4 - good balance
- **Fast:** MiniCPM-V-2.6 - for batch processing
- **Quality:** Qwen2-VL-7B Q4 - for detailed analysis

---

## Milestone 4.1: GGUF Model Loading

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 4.1.1 | llama.cpp integration | Load GGUF VLM models | Model loads without error |
| 4.1.2 | Metal GPU backend | Use GPU for inference | GPU utilization >80% |
| 4.1.3 | Model warmup | Pre-compute KV cache | First query faster |
| 4.1.4 | Memory management | Unload when idle | Memory freed after timeout |

### Code: GGUF Model Loader

```rust
// platform/apple/host/src/inference/gguf.rs

use llama_cpp_rs::{LlamaModel, LlamaContext, LlamaParams};

pub struct GgufVlmBackend {
    model: LlamaModel,
    ctx: LlamaContext,
    config: VlmConfig,
}

pub struct VlmConfig {
    /// Model path (GGUF file)
    pub model_path: PathBuf,

    /// Context window size
    pub context_size: u32,

    /// Number of GPU layers (-1 = all)
    pub gpu_layers: i32,

    /// Batch size for prompt processing
    pub batch_size: u32,

    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// Temperature for sampling
    pub temperature: f32,
}

impl Default for VlmConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("~/.cache/yama/models/llava-1.5-7b-q4_k_m.gguf"),
            context_size: 4096,
            gpu_layers: -1, // All layers on GPU
            batch_size: 512,
            max_tokens: 256,
            temperature: 0.7,
        }
    }
}

impl GgufVlmBackend {
    pub fn load(config: VlmConfig) -> Result<Self> {
        let mut params = LlamaParams::default();
        params.n_ctx = config.context_size;
        params.n_gpu_layers = config.gpu_layers;
        params.n_batch = config.batch_size;

        // Enable Metal on macOS
        #[cfg(target_os = "macos")]
        {
            params.metal = true;
        }

        let model = LlamaModel::load_from_file(&config.model_path, params)?;
        let ctx = model.create_context()?;

        Ok(Self {
            model,
            ctx,
            config,
        })
    }

    pub fn analyze(&mut self, image: &ImageData, prompt: &str) -> Result<String> {
        // 1. Encode image to model format
        let image_tokens = self.encode_image(image)?;

        // 2. Build prompt with image
        let full_prompt = format!(
            "<image>\n{image_tokens}\n</image>\nUser: {prompt}\nAssistant:"
        );

        // 3. Tokenize and evaluate
        let tokens = self.ctx.tokenize(&full_prompt)?;
        self.ctx.eval(&tokens, tokens.len() as i32, 0)?;

        // 4. Generate response
        let mut response = String::new();
        for _ in 0..self.config.max_tokens {
            let token = self.ctx.sample(
                self.config.temperature,
                1.0, // top_p
                40,  // top_k
            )?;

            if token == self.model.token_eos() {
                break;
            }

            response.push_str(&self.model.token_to_str(token)?);
            self.ctx.eval(&[token], 1, self.ctx.get_kv_cache_used_cells() as i32)?;
        }

        Ok(response.trim().to_string())
    }

    fn encode_image(&self, image: &ImageData) -> Result<String> {
        // Resize to model input size (336x336 for LLaVA)
        let resized = resize(image, 336, 336);

        // Convert to base64 for embedding in prompt
        // (This is a simplified approach - real implementation uses CLIP encoder)
        let jpeg = encode_jpeg(&resized, 90)?;
        let base64 = base64::encode(&jpeg);

        Ok(format!("[img-{}]", base64))
    }
}

#[async_trait]
impl VlmProvider for GgufVlmBackend {
    async fn analyze(&mut self, image: &ImageData, prompt: &str) -> Result<String> {
        // Run in blocking thread
        let image = image.clone();
        let prompt = prompt.to_string();

        tokio::task::spawn_blocking(move || {
            self.analyze(&image, &prompt)
        }).await?
    }

    async fn health(&self) -> Result<HealthStatus> {
        Ok(HealthStatus::Healthy)
    }
}
```

### Model Download

```bash
# Create models directory
mkdir -p ~/.cache/yama/models

# Download LLaVA-1.5-7B Q4_K_M (4.4GB)
huggingface-cli download mys/ggml_llava-v1.5-7b \
  ggml-model-q4_k.gguf mmproj-model-f16.gguf \
  --local-dir ~/.cache/yama/models/

# Or using direct curl
curl -L https://huggingface.co/mys/ggml_llava-v1.5-7b/resolve/main/ggml-model-q4_k.gguf \
  -o ~/.cache/yama/models/llava-1.5-7b-q4_k.gguf
```

---

## Milestone 4.2: Image Preprocessing

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 4.2.1 | CLIP vision encoder | Embed images for VLM | Correct embedding shape |
| 4.2.2 | Image resizing | Match model input size | No distortion |
| 4.2.3 | Normalization | Apply model-specific normalization | Values in expected range |

### Code: Vision Preprocessing

```rust
// platform/apple/host/src/inference/vision.rs

pub struct VisionEncoder {
    clip_model: Option<ClipModel>,
    target_size: (u32, u32),
    normalize_mean: [f32; 3],
    normalize_std: [f32; 3],
}

impl VisionEncoder {
    pub fn for_llava() -> Self {
        Self {
            clip_model: None, // Use GGUF built-in
            target_size: (336, 336),
            normalize_mean: [0.48145466, 0.4578275, 0.40821073],
            normalize_std: [0.26862954, 0.26130258, 0.27577711],
        }
    }

    pub fn preprocess(&self, image: &ImageData) -> Result<ProcessedImage> {
        // 1. Resize with aspect ratio preservation + center crop
        let resized = resize_center_crop(image, self.target_size.0, self.target_size.1);

        // 2. Convert to float32 and normalize
        let mut pixels: Vec<f32> = Vec::with_capacity(
            (self.target_size.0 * self.target_size.1 * 3) as usize
        );

        for y in 0..self.target_size.1 {
            for x in 0..self.target_size.0 {
                let pixel = resized.get_pixel(x, y);
                pixels.push((pixel.r as f32 / 255.0 - self.normalize_mean[0]) / self.normalize_std[0]);
                pixels.push((pixel.g as f32 / 255.0 - self.normalize_mean[1]) / self.normalize_std[1]);
                pixels.push((pixel.b as f32 / 255.0 - self.normalize_mean[2]) / self.normalize_std[2]);
            }
        }

        Ok(ProcessedImage {
            data: pixels,
            width: self.target_size.0,
            height: self.target_size.1,
            channels: 3,
        })
    }
}

fn resize_center_crop(image: &ImageData, target_w: u32, target_h: u32) -> ImageData {
    let (w, h) = (image.width, image.height);
    let aspect_src = w as f32 / h as f32;
    let aspect_dst = target_w as f32 / target_h as f32;

    let (crop_w, crop_h) = if aspect_src > aspect_dst {
        // Source is wider, crop sides
        let new_w = (h as f32 * aspect_dst) as u32;
        (new_w, h)
    } else {
        // Source is taller, crop top/bottom
        let new_h = (w as f32 / aspect_dst) as u32;
        (w, new_h)
    };

    let x_offset = (w - crop_w) / 2;
    let y_offset = (h - crop_h) / 2;

    // Crop then resize
    let cropped = crop(image, x_offset, y_offset, crop_w, crop_h);
    resize(&cropped, target_w, target_h)
}
```

---

## Milestone 4.3: Inference Pipeline

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 4.3.1 | Batch inference | Process multiple frames | Throughput >1 fps |
| 4.3.2 | Context window | Include prior context | More coherent responses |
| 4.3.3 | Streaming output | Token-by-token streaming | Responsive UI |
| 4.3.4 | Memory budget | Stay under 8GB total | No OOM errors |

### Code: Inference Pipeline

```rust
// platform/apple/host/src/inference/pipeline.rs

pub struct InferencePipeline {
    backend: Arc<RwLock<GgufVlmBackend>>,
    vision_encoder: VisionEncoder,
    config: PipelineConfig,
}

pub struct PipelineConfig {
    /// Include N previous frames as context
    pub context_frames: u32,

    /// Maximum concurrent requests
    pub max_concurrent: u32,

    /// Cache recent results
    pub cache_size: u32,
}

impl InferencePipeline {
    pub async fn analyze_frame(
        &self,
        frame: &Frame,
        prompt: &str,
        context: Option<&[Frame]>,
    ) -> Result<AnalysisResult> {
        let start = Instant::now();

        // 1. Preprocess current frame
        let processed = self.vision_encoder.preprocess(&frame.data)?;

        // 2. Build context prompt
        let context_prompt = if let Some(ctx_frames) = context {
            self.build_context_prompt(ctx_frames)?
        } else {
            String::new()
        };

        let full_prompt = format!(
            "{context_prompt}Based on this frame, {prompt}"
        );

        // 3. Run inference
        let mut backend = self.backend.write().await;
        let response = backend.analyze(&frame.data, &full_prompt)?;

        Ok(AnalysisResult {
            text: response,
            inference_time_ms: start.elapsed().as_millis() as u64,
            model: backend.model_name(),
        })
    }

    pub async fn analyze_frame_streaming(
        &self,
        frame: &Frame,
        prompt: &str,
    ) -> impl Stream<Item = String> {
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        let frame = frame.clone();
        let prompt = prompt.to_string();
        let backend = self.backend.clone();
        let encoder = self.vision_encoder.clone();

        tokio::spawn(async move {
            let processed = encoder.preprocess(&frame.data).unwrap();
            let mut backend = backend.write().await;

            // Stream tokens
            backend.analyze_streaming(&frame.data, &prompt, |token| {
                let _ = tx.blocking_send(token);
            }).unwrap();
        });

        tokio_stream::wrappers::ReceiverStream::new(rx)
    }

    fn build_context_prompt(&self, frames: &[Frame]) -> Result<String> {
        let mut context = String::from("Previous observations:\n");

        for (i, frame) in frames.iter().enumerate() {
            context.push_str(&format!(
                "- Frame {} ({:.1}s ago): [visual context]\n",
                i + 1,
                frames.len() - i
            ));
        }

        context.push('\n');
        Ok(context)
    }
}
```

---

## Memory Budget (Mac 16GB)

| Component | Allocation | Notes |
|-----------|------------|-------|
| VLM (7B Q4_K_M) | 4.5GB | LLaVA or Qwen2-VL |
| Vision encoder | 500MB | CLIP or built-in |
| KV cache | 500MB | Context window |
| SQLite + index | 500MB | From Phase 2 |
| Whisper (base.en) | 400MB | From Phase 3 |
| System reserve | 4GB | macOS + apps |
| **Headroom** | ~5GB | For peak usage |

---

## Milestone 4.4: Model Management UI

**Duration:** 2 days

### Problem

Users must manually run `curl` or `huggingface-cli` to download models:
```bash
# Current manual process
mkdir -p ~/.cache/yama/models
huggingface-cli download mys/ggml_llava-v1.5-7b ggml-model-q4_k.gguf --local-dir ~/.cache/yama/models/
```

This creates friction for first-time users and makes model management opaque.

### Solution

Integrated model management with:
1. **Model registry** - Configuration defines available models
2. **Download manager** - Progress reporting with speed/ETA
3. **Storage manager** - Show sizes, delete unused models
4. **First-run wizard** - Guide initial model selection

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 4.4.1 | Model registry | JSON config defines available models | Config loads |
| 4.4.2 | Download manager | HTTP download with progress | Updates every second |
| 4.4.3 | Storage manager | Show sizes, delete unused | UI reflects disk state |
| 4.4.4 | First-run wizard | Blocks until essential models downloaded | Wizard shows on empty cache |

### Code: Model Registry

```rust
// platform/apple/host/src/models/registry.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub models: Vec<ModelDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    pub model_type: ModelType,
    pub required: bool,
    pub recommended: bool,
    pub download_url: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    Clip,
    Whisper,
    Vlm,
}

impl ModelRegistry {
    pub fn default_registry() -> Self {
        Self {
            models: vec![
                ModelDefinition {
                    id: "clip-vit-b32".to_string(),
                    name: "CLIP ViT-B/32".to_string(),
                    description: "Visual search embeddings".to_string(),
                    size_bytes: 350 * 1024 * 1024,
                    model_type: ModelType::Clip,
                    required: true,
                    recommended: true,
                    download_url: "https://huggingface.co/.../clip-vit-b32.onnx".to_string(),
                    sha256: "...".to_string(),
                },
                ModelDefinition {
                    id: "whisper-base-en".to_string(),
                    name: "Whisper base.en".to_string(),
                    description: "English transcription".to_string(),
                    size_bytes: 400 * 1024 * 1024,
                    model_type: ModelType::Whisper,
                    required: true,
                    recommended: true,
                    download_url: "https://huggingface.co/.../whisper-base-en.bin".to_string(),
                    sha256: "...".to_string(),
                },
                ModelDefinition {
                    id: "llava-7b-q4".to_string(),
                    name: "LLaVA 7B Q4".to_string(),
                    description: "Visual analysis (recommended)".to_string(),
                    size_bytes: 4500 * 1024 * 1024,
                    model_type: ModelType::Vlm,
                    required: false,
                    recommended: true,
                    download_url: "https://huggingface.co/.../llava-7b-q4.gguf".to_string(),
                    sha256: "...".to_string(),
                },
            ],
        }
    }
}
```

### Code: Download Manager

```rust
// platform/apple/host/src/models/downloader.rs

pub struct ModelDownloader {
    client: reqwest::Client,
    models_dir: PathBuf,
}

pub struct DownloadProgress {
    pub model_id: String,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub eta_seconds: Option<u64>,
    pub status: DownloadStatus,
}

pub enum DownloadStatus {
    Pending,
    Downloading,
    Verifying,
    Complete,
    Error(String),
}

impl ModelDownloader {
    pub async fn download(
        &self,
        model: &ModelDefinition,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<PathBuf> {
        let dest = self.models_dir.join(&model.id);

        let response = self.client
            .get(&model.download_url)
            .send()
            .await?;

        let total = response.content_length().unwrap_or(model.size_bytes);
        let mut downloaded = 0u64;
        let mut file = tokio::fs::File::create(&dest).await?;
        let mut stream = response.bytes_stream();

        let start = Instant::now();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            let elapsed = start.elapsed().as_secs_f64();
            let speed = (downloaded as f64 / elapsed) as u64;
            let remaining = total.saturating_sub(downloaded);
            let eta = if speed > 0 { Some(remaining / speed) } else { None };

            progress_tx.send(DownloadProgress {
                model_id: model.id.clone(),
                total_bytes: total,
                downloaded_bytes: downloaded,
                speed_bytes_per_sec: speed,
                eta_seconds: eta,
                status: DownloadStatus::Downloading,
            }).await?;
        }

        // Verify SHA256
        progress_tx.send(DownloadProgress {
            model_id: model.id.clone(),
            status: DownloadStatus::Verifying,
            ..Default::default()
        }).await?;

        let hash = compute_sha256(&dest).await?;
        if hash != model.sha256 {
            return Err(anyhow!("SHA256 mismatch"));
        }

        progress_tx.send(DownloadProgress {
            model_id: model.id.clone(),
            status: DownloadStatus::Complete,
            ..Default::default()
        }).await?;

        Ok(dest)
    }
}
```

### First-Run Experience

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│                         ◆ Welcome to Yama                               │
│                                                                         │
│       Video analysis for researchers, journalists, and analysts         │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Required Models (~750 MB)                                              │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ ☑ CLIP ViT-B/32         350 MB    Visual search                 │   │
│  │ ☑ Whisper base.en       400 MB    Transcription                 │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Optional Models                                                        │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ ☐ LLaVA 7B Q4          4.5 GB    Visual analysis (Recommended) │   │
│  │ ☐ Whisper large-v3     1.5 GB    Better transcription accuracy │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Total download: 750 MB                                                 │
│                                                                         │
│                              [Download and Continue]                    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Verification

```bash
# Test: First run shows wizard
rm -rf ~/.cache/yama/models
cargo run -p yama-host-apple
# Expect: First-run wizard appears

# Test: Download progress updates
# Select CLIP, observe progress bar updates every second

# Test: Model appears in settings after download
# Navigate to Settings > Models
# Expect: Downloaded models listed with sizes
```

---

## Dependencies

- Phase 0 (UX Discovery) - Model settings wireframe validated
- Phase 1 (Foundation) - VlmProvider trait
- Phase 2 (Video Indexing) - frame extraction

## Blocks

- Phase 5 (Search) - VLM for query understanding
- Phase 7 (Tools) - analyze_frame tool

---

## Checklist

### Milestone 4.1: GGUF Model Loading
- [ ] 4.1.1 llama.cpp integration
- [ ] 4.1.2 Metal GPU backend
- [ ] 4.1.3 Model warmup
- [ ] 4.1.4 Memory management

### Milestone 4.2: Image Preprocessing
- [ ] 4.2.1 CLIP vision encoder
- [ ] 4.2.2 Image resizing
- [ ] 4.2.3 Normalization

### Milestone 4.3: Inference Pipeline
- [ ] 4.3.1 Batch inference
- [ ] 4.3.2 Context window
- [ ] 4.3.3 Streaming output
- [ ] 4.3.4 Memory budget

### Milestone 4.4: Model Management UI
- [ ] 4.4.1 Model registry configuration
- [ ] 4.4.2 Download manager with progress
- [ ] 4.4.3 Storage manager (show sizes, delete)
- [ ] 4.4.4 First-run wizard
