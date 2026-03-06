# Platform Traits API Reference

Complete API documentation for cross-platform trait abstractions.

## Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
│         (Compositor, Agent Runtime, Services)               │
└─────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                   Platform Traits                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │
│  │ Renderer │ │ Decoder  │ │Allocator │ │ Inference    │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘   │
└─────────────────────────────────────────────────────────────┘
                             │
             ┌───────────────┼───────────────┐
             ▼               ▼               ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  Apple Silicon  │ │  NVIDIA Jetson  │ │   Generic/SW    │
│  (Metal, VT)    │ │  (CUDA, NVDEC)  │ │   (Fallback)    │
└─────────────────┘ └─────────────────┘ └─────────────────┘
```

## Location

`shared/platform-traits/src/`

## VideoDecoder Trait

Hardware video decoding abstraction.

### Trait Definition

```rust
#[async_trait]
pub trait VideoDecoder: Send + Sync {
    fn capabilities(&self) -> &DecoderCapabilities;
    fn configure(&mut self, config: CodecConfig) -> PlatformResult<()>;
    async fn decode(&mut self, packet: &DecodePacket) -> PlatformResult<Vec<DecodedFrame>>;
    async fn flush(&mut self) -> PlatformResult<Vec<DecodedFrame>>;
    fn reset(&mut self) -> PlatformResult<()>;
    fn is_codec_supported(&self, codec: CodecType) -> bool;
    fn current_config(&self) -> Option<&CodecConfig>;
    fn statistics(&self) -> DecoderStatistics;
}
```

### DecoderCapabilities

```rust
pub struct DecoderCapabilities {
    pub name: String,                    // "VideoToolbox (GStreamer)"
    pub is_hardware: bool,               // true for HW acceleration
    pub supported_codecs: Vec<CodecType>,
    pub max_resolution: Size,            // e.g., 7680x4320 (8K)
    pub output_formats: Vec<PixelFormat>,
    pub supports_zero_copy: bool,        // DMA-BUF/IOSurface
    pub max_sessions: u32,               // Concurrent decode sessions
}
```

### CodecConfig

```rust
pub struct CodecConfig {
    pub codec: CodecType,
    pub width: u32,
    pub height: u32,
    pub output_format: Option<PixelFormat>,
    pub extra_data: Option<Vec<u8>>,     // SPS/PPS for H.264
    pub low_latency: bool,
    pub reference_frames: u32,
}

impl CodecConfig {
    pub fn h264(width: u32, height: u32) -> Self;
    pub fn h265(width: u32, height: u32) -> Self;
}
```

### CodecType

```rust
pub enum CodecType {
    H264,   // AVC
    H265,   // HEVC
    Vp8,
    Vp9,
    Av1,
    Mjpeg,
}
```

### DecodedFrame

```rust
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub pts: i64,           // Presentation timestamp (μs)
    pub dts: i64,           // Decode timestamp (μs)
    pub duration: i64,
    pub is_keyframe: bool,
    pub data: FrameData,
}

pub enum FrameData {
    CpuBuffer { data: Vec<u8>, strides: Vec<usize> },
    GpuBuffer(GpuBuffer),
    #[cfg(unix)]
    DmaBuf { fd: RawFd, offset: u64, stride: u32, modifier: u64 },
}
```

### DecoderStatistics

```rust
pub struct DecoderStatistics {
    pub frames_decoded: u64,
    pub frames_dropped: u64,
    pub avg_decode_time_us: u64,
    pub peak_decode_time_us: u64,
    pub queue_depth: u32,
}
```

---

## Renderer Trait

GPU rendering abstraction.

### Trait Definition

```rust
#[async_trait]
pub trait Renderer: Send + Sync {
    fn capabilities(&self) -> &RendererCapabilities;
    fn create_texture(&mut self, format: TextureFormat, data: Option<&[u8]>) -> PlatformResult<TextureHandle>;
    fn create_texture_from_buffer(&mut self, format: TextureFormat, buffer: &GpuBuffer) -> PlatformResult<TextureHandle>;
    #[cfg(unix)]
    fn import_dma_buf(&mut self, fd: RawFd, format: TextureFormat, offset: u64, stride: u32) -> PlatformResult<TextureHandle>;
    fn update_texture(&mut self, texture: TextureHandle, rect: Option<Rect>, data: &[u8]) -> PlatformResult<()>;
    fn destroy_texture(&mut self, texture: TextureHandle) -> PlatformResult<()>;
    fn begin_frame(&mut self) -> PlatformResult<()>;
    fn clear(&mut self, color: Color) -> PlatformResult<()>;
    fn render(&mut self, surfaces: &[RenderSurface]) -> PlatformResult<()>;
    fn end_frame(&mut self) -> PlatformResult<()>;
    async fn present(&mut self) -> PlatformResult<()>;
    fn set_output_size(&mut self, size: Size) -> PlatformResult<()>;
    fn output_size(&self) -> Size;
    fn flush(&mut self) -> PlatformResult<()>;
    async fn wait_idle(&mut self) -> PlatformResult<()>;
    fn memory_usage(&self) -> MemoryUsage;
}
```

### RendererCapabilities

```rust
pub struct RendererCapabilities {
    pub name: String,                    // "Metal Renderer"
    pub api: String,                     // "Metal", "Vulkan"
    pub max_texture_size: u32,
    pub supported_formats: Vec<PixelFormat>,
    pub supports_dma_buf_import: bool,
    pub supports_dma_buf_export: bool,
    pub max_render_targets: u32,
    pub gpu_memory_bytes: u64,
}
```

### TextureHandle

```rust
pub struct TextureHandle(pub u64);

impl TextureHandle {
    pub const INVALID: Self = Self(0);
    pub fn is_valid(&self) -> bool { self.0 != 0 }
}
```

### RenderSurface

```rust
pub struct RenderSurface {
    pub texture: TextureHandle,
    pub src_rect: Option<Rect>,
    pub dst_rect: Rect,
    pub transform: Transform,
    pub opacity: f32,
    pub z_order: i32,
}
```

### MemoryUsage

```rust
pub struct MemoryUsage {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub texture_count: u32,
    pub buffer_count: u32,
}
```

---

## InferenceEngine Trait

ML inference abstraction.

### Trait Definition

```rust
#[async_trait]
pub trait InferenceEngine: Send + Sync {
    fn capabilities(&self) -> &InferenceCapabilities;
    async fn load_model(&mut self, path: &Path, name: Option<&str>) -> PlatformResult<ModelInfo>;
    async fn load_model_bytes(&mut self, data: &[u8], format: &str, name: &str) -> PlatformResult<ModelInfo>;
    fn unload_model(&mut self, name: &str) -> PlatformResult<()>;
    fn model_info(&self, name: &str) -> Option<&ModelInfo>;
    fn loaded_models(&self) -> Vec<String>;
    async fn infer(&mut self, request: InferenceRequest) -> PlatformResult<InferenceResponse>;
    async fn infer_batch(&mut self, requests: Vec<InferenceRequest>) -> PlatformResult<Vec<InferenceResponse>>;
    fn memory_usage(&self) -> InferenceMemoryUsage;
    async fn warmup(&mut self, model: &str) -> PlatformResult<()>;
}
```

### InferenceCapabilities

```rust
pub struct InferenceCapabilities {
    pub name: String,
    pub backend: String,          // "Metal", "CUDA"
    pub device: String,           // "Apple M3 Max"
    pub supported_dtypes: Vec<DataType>,
    pub supported_formats: Vec<String>,
    pub max_batch_size: u32,
    pub supports_async: bool,
    pub available_memory: u64,
    pub compute_tflops: f32,
}
```

### Tensor

```rust
pub struct Tensor {
    pub name: String,
    pub dtype: DataType,
    pub shape: TensorShape,
    pub data: Vec<u8>,
}

pub enum DataType {
    Float32, Float16, BFloat16,
    Int32, Int64, Int8, Uint8, Bool,
}

pub struct TensorShape { pub dims: Vec<usize> }
```

---

## GpuAllocator Trait

GPU memory management.

### Trait Definition

```rust
pub trait GpuAllocator: Send + Sync {
    fn allocate(&mut self, size: usize, usage: BufferUsage) -> PlatformResult<GpuBuffer>;
    fn free(&mut self, buffer: GpuBuffer) -> PlatformResult<()>;
    fn map(&mut self, buffer: &GpuBuffer) -> PlatformResult<*mut u8>;
    fn unmap(&mut self, buffer: &GpuBuffer) -> PlatformResult<()>;
    fn memory_available(&self) -> u64;
    fn memory_used(&self) -> u64;
}

pub struct GpuBuffer {
    pub id: u64,
    pub size: usize,
    pub usage: BufferUsage,
    pub offset: u64,
}

pub enum BufferUsage {
    Vertex, Index, Uniform, Storage, Transfer,
}
```

---

## DisplayBackend Trait

Display output management.

### Trait Definition

```rust
pub trait DisplayBackend: Send + Sync {
    fn outputs(&self) -> &[OutputInfo];
    fn set_mode(&mut self, output: OutputId, mode: DisplayMode) -> PlatformResult<()>;
    fn current_mode(&self, output: OutputId) -> Option<DisplayMode>;
    fn enable_output(&mut self, output: OutputId) -> PlatformResult<()>;
    fn disable_output(&mut self, output: OutputId) -> PlatformResult<()>;
}

pub struct OutputInfo {
    pub id: OutputId,
    pub name: String,
    pub physical_size: Size,
    pub modes: Vec<DisplayMode>,
    pub current_mode: Option<DisplayMode>,
    pub enabled: bool,
}

pub struct DisplayMode {
    pub resolution: Size,
    pub refresh_rate: u32,  // mHz
}
```

---

## Common Types

### Size

```rust
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Self;
}
```

### Rect

```rust
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
```

### Color

```rust
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
}
```

### PixelFormat

```rust
pub enum PixelFormat {
    Rgba8, Bgra8, Rgb8, Bgr8,
    Nv12, I420, Yuy2,
    R8, Rg8,
}
```

### Transform

```rust
pub struct Transform {
    pub matrix: [[f32; 3]; 3],
}

impl Transform {
    pub const IDENTITY: Self;
    pub fn rotate(angle: f32) -> Self;
    pub fn scale(sx: f32, sy: f32) -> Self;
    pub fn translate(tx: f32, ty: f32) -> Self;
}
```

---

## Error Types

```rust
pub enum PlatformError {
    InitializationFailed(String),
    NotSupported(String),
    NotFound(String),
    InvalidParameter(String),
    OutOfMemory(String),
    DeviceLost(String),
    CodecNotSupported(String),
    DecodeError(String),
    RenderError(String),
    InferenceError(String),
}

pub type PlatformResult<T> = Result<T, PlatformError>;
```

---

## Platform Detection

```rust
pub fn detect_platform() -> PlatformInfo;

pub struct PlatformInfo {
    pub platform: Platform,
    pub capabilities: PlatformCapabilities,
}

pub enum Platform {
    AppleSilicon,
    JetsonOrin,
    Generic,
}

pub struct PlatformCapabilities {
    pub has_hardware_decode: bool,
    pub has_hardware_encode: bool,
    pub has_gpu_inference: bool,
    pub has_unified_memory: bool,
}
```
