# Apple Platform Guide

Deep reference for Apple Silicon implementation details.

## Platform Overview

- **Target**: macOS 14+, iOS 17+
- **GPU**: Metal 3
- **Video**: VideoToolbox (vtdec_hw)
- **Inference**: Metal MPS, Core ML, TTRA
- **Memory**: Unified memory architecture

## File Structure

```
platform/apple/
├── host/src/
│   ├── main.rs           # Entry point
│   ├── compositor/       # Metal rendering
│   │   ├── mod.rs
│   │   ├── render.rs     # MetalRenderer
│   │   ├── surface.rs    # SurfaceManager
│   │   └── input.rs      # InputHandler
│   ├── event_bus/        # WebSocket server
│   └── orchestrator/     # Container management
├── video/src/
│   ├── main.rs           # Video service entry
│   └── decoder.rs        # VideoToolboxDecoder
└── inference/src/
    ├── main.rs           # Inference service
    └── engine.rs         # MetalInferenceEngine
```

## VideoToolbox Decoding

### Supported Codecs

| Codec | Element | Hardware |
|-------|---------|----------|
| H.264 | vtdec_hw | Yes |
| H.265 | vtdec_hw | Yes |
| VP9 | vp9dec | Software |
| AV1 | Software | No (M3+ may add) |

### Implementation

```rust
// platform/apple/video/src/decoder.rs

pub struct VideoToolboxDecoder {
    capabilities: DecoderCapabilities,
    config: Option<CodecConfig>,
    pipeline: Option<gst::Pipeline>,
    frame_queue: Arc<Mutex<VecDeque<DecodedFrame>>>,
}

impl VideoToolboxDecoder {
    fn build_pipeline_string(config: &CodecConfig) -> String {
        let decoder = match config.codec {
            CodecType::H264 | CodecType::H265 => "vtdec_hw",
            CodecType::Vp9 => "vp9dec",
            _ => "avdec_h264",
        };

        format!(
            "appsrc name=src ! {} ! videoconvert ! video/x-raw,format=NV12 ! appsink name=sink",
            decoder
        )
    }
}
```

### Capabilities

```rust
DecoderCapabilities {
    name: "VideoToolbox (GStreamer)".to_string(),
    is_hardware: true,
    supported_codecs: vec![CodecType::H264, CodecType::H265, CodecType::Vp9],
    max_resolution: Size::new(7680, 4320),  // 8K
    output_formats: vec![PixelFormat::Nv12, PixelFormat::I420, PixelFormat::Bgra8],
    supports_zero_copy: true,  // IOSurface
    max_sessions: 8,
}
```

## IOSurface Integration

VideoToolbox outputs IOSurface-backed buffers for zero-copy GPU access.

### IOSurface to Metal Texture

```rust
// platform/apple/host/src/compositor/surface.rs

use metal::MTLTexture;
use io_surface::IOSurface;

pub fn import_io_surface(surface: &IOSurface, device: &metal::Device) -> MTLTexture {
    let desc = metal::TextureDescriptor::new();
    desc.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
    desc.set_width(surface.width() as u64);
    desc.set_height(surface.height() as u64);

    device.new_texture_with_io_surface(surface, &desc)
}
```

### Memory Model

Apple Silicon uses unified memory:
- CPU and GPU share physical memory
- No explicit memory transfers needed
- IOSurface provides zero-copy sharing

```rust
// Unified memory advantages
impl GpuBuffer {
    pub fn cpu_ptr(&self) -> *mut u8 {
        // Direct CPU access to GPU buffer
        self.metal_buffer.contents() as *mut u8
    }
}
```

## Metal Rendering

### MetalRenderer

```rust
// platform/apple/host/src/compositor/render.rs

pub struct MetalRenderer {
    device: metal::Device,
    command_queue: metal::CommandQueue,
    render_pipeline: metal::RenderPipelineState,
    textures: HashMap<TextureHandle, metal::Texture>,
}

impl Renderer for MetalRenderer {
    fn capabilities(&self) -> &RendererCapabilities {
        &RendererCapabilities {
            name: "Metal Renderer".to_string(),
            api: "Metal".to_string(),
            max_texture_size: 16384,
            supports_dma_buf_import: false,  // macOS uses IOSurface
            supports_dma_buf_export: false,
            gpu_memory_bytes: self.query_gpu_memory(),
        }
    }

    fn begin_frame(&mut self) -> PlatformResult<()> {
        self.command_buffer = self.command_queue.new_command_buffer();
        Ok(())
    }

    async fn present(&mut self) -> PlatformResult<()> {
        self.command_buffer.present_drawable(&self.drawable);
        self.command_buffer.commit();
        self.command_buffer.wait_until_completed();
        Ok(())
    }
}
```

### Retina Display Handling

```rust
fn handle_scale_factor(window: &Window, config: &mut CompositorConfig) {
    let scale = window.scale_factor();
    config.width = (config.width as f64 * scale) as u32;
    config.height = (config.height as f64 * scale) as u32;
}
```

## Metal Inference

### MetalInferenceEngine

```rust
// platform/apple/inference/src/engine.rs

pub struct MetalInferenceEngine {
    capabilities: InferenceCapabilities,
    models: HashMap<String, ModelInfo>,
    client: reqwest::Client,  // TTRA HTTP client
    ttra_url: String,
}

impl MetalInferenceEngine {
    fn detect_metal_device() -> String {
        // sysctl -n machdep.cpu.brand_string
        // Returns: "Apple M3 Max"
    }

    fn query_gpu_memory() -> u64 {
        // sysctl -n hw.memsize
        // Returns total RAM, assume half for GPU
    }

    fn estimate_tflops() -> f32 {
        // M3 Max: 14.2 TFLOPS
        // M2 Max: 13.6 TFLOPS
        // M1 Max: 10.4 TFLOPS
    }
}
```

### TTRA Integration

TTRA provides OpenAI-compatible API with Metal acceleration:

```rust
async fn chat_completion(&self, model: &str, messages: Vec<Value>) -> Result<String> {
    let response = self.client
        .post(&format!("{}/v1/chat/completions", self.ttra_url))
        .json(&json!({
            "model": model,
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 512,
        }))
        .send()
        .await?;

    let json: Value = response.json().await?;
    Ok(json["choices"][0]["message"]["content"].as_str().unwrap().to_string())
}
```

### Supported Model Formats

| Format | Extension | Notes |
|--------|-----------|-------|
| GGUF | .gguf | llama.cpp, quantized |
| MLX | .mlx | Apple MLX framework |
| SafeTensors | .safetensors | HuggingFace format |
| Core ML | .mlmodel | Native Apple format |

## egui Integration

### wgpu with Metal Backend

```rust
// Compositor uses wgpu with Metal backend
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::METAL,
    ..Default::default()
});

let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
    power_preference: wgpu::PowerPreference::HighPerformance,
    compatible_surface: Some(&surface),
    force_fallback_adapter: false,
}).await?;
```

### egui-wgpu

```rust
use egui_wgpu::Renderer as EguiRenderer;

let mut egui_renderer = EguiRenderer::new(&device, output_format, None, 1);

// Render egui
egui_renderer.render(
    &mut encoder,
    &view,
    &full_output.shapes,
    &full_output.textures_delta,
);
```

## Build Configuration

### Cargo.toml

```toml
[target.'cfg(target_os = "macos")'.dependencies]
metal = "0.28"
cocoa = "0.25"
core-graphics = "0.23"
io-surface = "0.15"

[features]
metal = ["wgpu/metal"]
```

### Conditional Compilation

```rust
#[cfg(target_os = "macos")]
mod metal_impl {
    // macOS-specific code
}

#[cfg(target_os = "macos")]
pub use metal_impl::*;
```

## GStreamer on macOS

### Installation

```bash
brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad
```

### Environment

```bash
export GST_PLUGIN_PATH=/opt/homebrew/lib/gstreamer-1.0
export PKG_CONFIG_PATH=/opt/homebrew/lib/pkgconfig
```

### VideoToolbox Plugin

The `vtdec_hw` element requires:
- gst-plugins-bad with VideoToolbox support
- macOS 10.8+ (Mountain Lion)

## Troubleshooting

### Common Issues

| Issue | Cause | Fix |
|-------|-------|-----|
| vtdec_hw not found | GStreamer VideoToolbox plugin missing | `brew reinstall gst-plugins-bad` |
| Metal device nil | Running on Intel Mac | Use software fallback |
| IOSurface import fails | Incompatible pixel format | Convert to BGRA8 |
| Low performance | Discrete GPU not selected | Set PowerPreference::HighPerformance |

### Debug Commands

```bash
# Check GStreamer plugins
gst-inspect-1.0 vtdec_hw

# Check Metal support
system_profiler SPDisplaysDataType | grep Metal

# Check unified memory
sysctl hw.memsize
```
