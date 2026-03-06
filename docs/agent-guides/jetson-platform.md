# Jetson Platform Guide

Deep reference for NVIDIA Jetson Orin implementation details.

## Platform Overview

- **Target**: Jetson Orin NX/AGX, L4T 36+
- **GPU**: Vulkan 1.3
- **Video**: NVDEC (nvv4l2decoder)
- **Inference**: TensorRT, CUDA
- **Display**: DRM/KMS, Smithay Wayland

## File Structure

```
platform/jetson/
├── host/src/
│   ├── main.rs           # Entry point
│   ├── compositor/       # Vulkan + Smithay
│   │   ├── mod.rs
│   │   ├── render.rs     # VulkanRenderer
│   │   ├── surface.rs    # DRM surface management
│   │   └── input.rs      # libinput
│   ├── event_bus/        # WebSocket server
│   └── orchestrator/     # Container management
├── video/src/
│   ├── main.rs           # Video service
│   └── decoder.rs        # NvdecDecoder
└── inference/src/
    ├── main.rs           # Inference service
    └── engine.rs         # CudaInferenceEngine
```

## NVDEC Decoding

### Supported Codecs

| Codec | Element | Orin Support |
|-------|---------|--------------|
| H.264 | nvv4l2decoder | Yes |
| H.265 | nvv4l2decoder | Yes |
| VP9 | nvv4l2decoder | Yes |
| AV1 | nvv4l2decoder | Yes (Orin only) |

### Implementation

```rust
// platform/jetson/video/src/decoder.rs

pub struct NvdecDecoder {
    capabilities: DecoderCapabilities,
    config: Option<CodecConfig>,
    pipeline: Option<gst::Pipeline>,
    frame_queue: Arc<Mutex<VecDeque<DecodedFrame>>>,
}

impl NvdecDecoder {
    fn detect_jetson() -> bool {
        Path::new("/etc/nv_tegra_release").exists()
            || Path::new("/proc/device-tree/compatible")
                .read_to_string()
                .map(|s| s.contains("nvidia,tegra"))
                .unwrap_or(false)
    }

    fn build_pipeline_string(config: &CodecConfig) -> String {
        // Jetson uses nvv4l2decoder + nvvidconv
        format!(
            "appsrc name=src ! nvv4l2decoder ! nvvidconv ! video/x-raw,format=NV12 ! appsink name=sink"
        )
    }
}
```

### Capabilities

```rust
DecoderCapabilities {
    name: "NVDEC (Jetson)".to_string(),
    is_hardware: true,
    supported_codecs: vec![CodecType::H264, CodecType::H265, CodecType::Vp9, CodecType::Av1],
    max_resolution: Size::new(7680, 4320),  // 8K
    output_formats: vec![PixelFormat::Nv12, PixelFormat::I420, PixelFormat::Bgra8],
    supports_zero_copy: true,  // DMA-BUF
    max_sessions: 16,          // Jetson supports more concurrent sessions
}
```

## DMA-BUF Integration

Jetson uses DMA-BUF for zero-copy buffer sharing between hardware blocks.

### DMA-BUF from Decoder

```rust
#[cfg(unix)]
fn extract_dma_buf(buffer: &gst::Buffer) -> Option<DmaBufInfo> {
    // Check for DMA-BUF metadata
    let meta = buffer.meta::<gstreamer_allocators::DmaBufMeta>()?;

    Some(DmaBufInfo {
        fd: meta.fd(),
        offset: meta.offset(),
        stride: meta.stride(),
        modifier: meta.modifier(),
    })
}
```

### DMA-BUF to Vulkan Texture

```rust
// platform/jetson/host/src/compositor/render.rs

use ash::extensions::khr::ExternalMemoryFd;

impl VulkanRenderer {
    fn import_dma_buf(&mut self, fd: RawFd, format: TextureFormat) -> PlatformResult<TextureHandle> {
        let import_info = vk::ImportMemoryFdInfoKHR::builder()
            .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
            .fd(fd);

        let memory = unsafe {
            self.device.allocate_memory(&alloc_info, None)?
        };

        // Create image from external memory
        let image = self.create_image_from_external_memory(memory, format)?;

        Ok(self.register_texture(image))
    }
}
```

### FrameData DMA-BUF Variant

```rust
pub enum FrameData {
    CpuBuffer { data: Vec<u8>, strides: Vec<usize> },
    GpuBuffer(GpuBuffer),
    #[cfg(unix)]
    DmaBuf {
        fd: std::os::unix::io::RawFd,
        offset: u64,
        stride: u32,
        modifier: u64,  // DRM format modifier
    },
}
```

## Smithay Compositor

Jetson runs a custom Wayland compositor using Smithay.

### Architecture

```
┌─────────────────────────────────────┐
│  egui UI Layer                      │
├─────────────────────────────────────┤
│  Video Surface Layer                │
├─────────────────────────────────────┤
│  Smithay Compositor                 │
├─────────────────────────────────────┤
│  DRM/KMS Backend                    │
└─────────────────────────────────────┘
```

### Smithay Integration

```rust
// platform/jetson/host/src/compositor/mod.rs

use smithay::{
    backend::drm::{DrmDevice, DrmNode},
    reexports::calloop::EventLoop,
    wayland::compositor::CompositorState,
};

pub struct JetsonCompositor {
    event_loop: EventLoop<'static, AppState>,
    drm_device: DrmDevice,
    drm_surface: DrmSurface,
    renderer: VulkanRenderer,
}

impl JetsonCompositor {
    pub fn new() -> Result<Self> {
        // Find DRM device
        let drm_node = DrmNode::from_path("/dev/dri/card0")?;
        let (drm_device, drm_notifier) = DrmDevice::new(drm_node)?;

        // Create Vulkan renderer
        let renderer = VulkanRenderer::new(&drm_device)?;

        Ok(Self { /* ... */ })
    }
}
```

### Display Modes

```rust
// DRM/KMS display mode setting
impl DisplayBackend for JetsonCompositor {
    fn set_mode(&mut self, output: OutputId, mode: DisplayMode) -> PlatformResult<()> {
        let connector = self.drm_device.connector(output)?;
        let crtc = self.find_crtc_for_connector(&connector)?;

        self.drm_device.set_mode(crtc, connector, &mode)?;
        Ok(())
    }
}
```

## Vulkan Rendering

### VulkanRenderer

```rust
// platform/jetson/host/src/compositor/render.rs

pub struct VulkanRenderer {
    instance: ash::Instance,
    device: ash::Device,
    queue: vk::Queue,
    swapchain: vk::SwapchainKHR,
    textures: HashMap<TextureHandle, VulkanTexture>,
}

impl Renderer for VulkanRenderer {
    fn capabilities(&self) -> &RendererCapabilities {
        &RendererCapabilities {
            name: "Vulkan Renderer".to_string(),
            api: "Vulkan".to_string(),
            max_texture_size: 16384,
            supports_dma_buf_import: true,
            supports_dma_buf_export: true,
            gpu_memory_bytes: self.query_vram(),
        }
    }

    async fn present(&mut self) -> PlatformResult<()> {
        let present_info = vk::PresentInfoKHR::builder()
            .swapchains(&[self.swapchain])
            .image_indices(&[self.current_image]);

        unsafe {
            self.swapchain_loader.queue_present(self.queue, &present_info)?;
        }
        Ok(())
    }
}
```

### Required Extensions

```rust
const REQUIRED_DEVICE_EXTENSIONS: &[&str] = &[
    "VK_KHR_swapchain",
    "VK_KHR_external_memory_fd",
    "VK_EXT_external_memory_dma_buf",
    "VK_EXT_image_drm_format_modifier",
];
```

## CUDA Inference

### CudaInferenceEngine

```rust
// platform/jetson/inference/src/engine.rs

pub struct CudaInferenceEngine {
    capabilities: InferenceCapabilities,
    models: HashMap<String, TensorRTEngine>,
    cuda_device: i32,
    cuda_stream: cudaStream_t,
}

impl InferenceEngine for CudaInferenceEngine {
    async fn load_model(&mut self, path: &Path, name: Option<&str>) -> PlatformResult<ModelInfo> {
        let engine = TensorRTEngine::load(path)?;
        // ...
    }

    async fn infer(&mut self, request: InferenceRequest) -> PlatformResult<InferenceResponse> {
        let engine = self.models.get(&request.model)?;

        // Copy inputs to GPU
        for input in &request.inputs {
            cuda_memcpy_htod(engine.input_buffer, &input.data)?;
        }

        // Execute inference
        engine.execute_async(self.cuda_stream)?;

        // Copy outputs from GPU
        cuda_memcpy_dtoh(&mut output_data, engine.output_buffer)?;

        Ok(InferenceResponse { /* ... */ })
    }
}
```

### TensorRT Integration

```rust
struct TensorRTEngine {
    runtime: *mut nvinfer1::IRuntime,
    engine: *mut nvinfer1::ICudaEngine,
    context: *mut nvinfer1::IExecutionContext,
    input_buffer: *mut c_void,
    output_buffer: *mut c_void,
}

impl TensorRTEngine {
    fn load(path: &Path) -> Result<Self> {
        let runtime = create_infer_runtime()?;
        let engine = runtime.deserialize_cuda_engine(path)?;
        let context = engine.create_execution_context()?;
        // ...
    }
}
```

## libinput Integration

```rust
// platform/jetson/host/src/compositor/input.rs

use input::{Libinput, LibinputInterface};

pub struct InputHandler {
    libinput: Libinput,
}

impl InputHandler {
    pub fn process_events(&mut self, surfaces: &mut SurfaceManager) -> Result<()> {
        self.libinput.dispatch()?;

        for event in &mut self.libinput {
            match event {
                Event::Keyboard(kb) => self.handle_keyboard(kb, surfaces)?,
                Event::Pointer(ptr) => self.handle_pointer(ptr, surfaces)?,
                Event::Touch(touch) => self.handle_touch(touch, surfaces)?,
                _ => {}
            }
        }

        Ok(())
    }
}
```

## Build Configuration

### Cargo.toml

```toml
[target.'cfg(target_os = "linux")'.dependencies]
ash = "0.37"
smithay = { version = "0.3", features = ["backend_drm", "backend_libinput"] }
gstreamer-allocators = "0.23"
input = "0.9"

[features]
jetson = ["vulkan", "cuda"]
vulkan = ["ash"]
cuda = []
```

### Cross-Compilation

```bash
# Target: aarch64-unknown-linux-gnu
cargo build --target aarch64-unknown-linux-gnu --features jetson
```

## GStreamer on Jetson

### Jetson Multimedia Plugins

```bash
# Installed with JetPack
/usr/lib/aarch64-linux-gnu/gstreamer-1.0/
├── libgstnvv4l2.so          # nvv4l2decoder, nvv4l2encoder
├── libgstnvvidconv.so       # nvvidconv
├── libgstnvjpeg.so          # nvjpegdec, nvjpegenc
└── libgstnvcompositor.so    # nvcompositor
```

### Pipeline Examples

```bash
# H.264 hardware decode
gst-launch-1.0 filesrc location=video.mp4 ! qtdemux ! h264parse ! nvv4l2decoder ! nvvidconv ! xvimagesink

# RTSP stream
gst-launch-1.0 rtspsrc location=rtsp://... ! rtph264depay ! nvv4l2decoder ! nvvidconv ! appsink
```

## Troubleshooting

### Common Issues

| Issue | Cause | Fix |
|-------|-------|-----|
| nvv4l2decoder not found | JetPack not installed | Install JetPack 6.0+ |
| DRM permission denied | User not in video group | `sudo usermod -aG video $USER` |
| Vulkan driver missing | NVIDIA driver issue | Reinstall JetPack |
| DMA-BUF import fails | Incompatible modifier | Use linear modifier |

### Debug Commands

```bash
# Check NVDEC capabilities
cat /sys/class/video4linux/video*/name

# Check DRM devices
ls -la /dev/dri/

# Check Vulkan
vulkaninfo | grep -i nvidia

# Check CUDA
nvidia-smi
```

### Environment Variables

```bash
# Force Vulkan rendering
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json

# GStreamer debug
export GST_DEBUG=nvv4l2decoder:5

# CUDA visible devices
export CUDA_VISIBLE_DEVICES=0
```
