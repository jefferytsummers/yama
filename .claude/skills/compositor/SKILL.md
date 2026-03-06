---
name: compositor
description: Native rendering with wgpu, Metal/Vulkan backends, and egui integration
compatibility:
  - platform/apple/host/src/compositor/
  - platform/jetson/host/src/compositor/
  - shared/platform-traits/src/renderer.rs
---

# Compositor & Rendering

Native GPU compositor using wgpu with egui for UI.

## Stack

- **wgpu 0.20** - GPU abstraction (Metal on Apple, Vulkan on Jetson)
- **egui 0.29** - Immediate mode UI
- **Smithay 0.3** - Wayland compositor (Jetson only)

## Architecture

```
┌─────────────────────────────────────┐
│  egui Context (UI Layer)            │
├─────────────────────────────────────┤
│  SurfaceManager (Video Surfaces)    │
├─────────────────────────────────────┤
│  Renderer (wgpu)                    │
├─────────────────────────────────────┤
│  Metal / Vulkan Backend             │
└─────────────────────────────────────┘
```

## Key Types

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
    pub src_rect: Option<Rect>,  // Source within texture
    pub dst_rect: Rect,          // Destination on screen
    pub transform: Transform,
    pub opacity: f32,            // 0.0 - 1.0
    pub z_order: i32,            // Higher = on top
}
```

## Renderer Trait

```rust
#[async_trait]
pub trait Renderer: Send + Sync {
    fn capabilities(&self) -> &RendererCapabilities;
    fn create_texture(&mut self, format: TextureFormat, data: Option<&[u8]>) -> PlatformResult<TextureHandle>;
    fn create_texture_from_buffer(&mut self, format: TextureFormat, buffer: &GpuBuffer) -> PlatformResult<TextureHandle>;
    fn update_texture(&mut self, texture: TextureHandle, rect: Option<Rect>, data: &[u8]) -> PlatformResult<()>;
    fn destroy_texture(&mut self, texture: TextureHandle) -> PlatformResult<()>;
    fn begin_frame(&mut self) -> PlatformResult<()>;
    fn clear(&mut self, color: Color) -> PlatformResult<()>;
    fn render(&mut self, surfaces: &[RenderSurface]) -> PlatformResult<()>;
    fn end_frame(&mut self) -> PlatformResult<()>;
    async fn present(&mut self) -> PlatformResult<()>;
}
```

## Key Files

- `platform/apple/host/src/compositor/mod.rs` - Apple compositor entry
- `platform/apple/host/src/compositor/render.rs` - Metal renderer
- `platform/apple/host/src/compositor/surface.rs` - Surface management
- `platform/jetson/host/src/compositor/mod.rs` - Jetson compositor
- `shared/platform-traits/src/renderer.rs` - Renderer trait

## Platform Specifics

### Apple (Metal)

- Use IOSurface for zero-copy video textures
- Handle Retina display scale factor
- Metal command buffers for rendering
- wgpu with Metal backend

```rust
// Retina scale handling
let scale_factor = window.scale_factor();
let physical_size = logical_size.to_physical::<u32>(scale_factor);
```

### Jetson (Vulkan + Smithay)

- DMA-BUF import for zero-copy video
- DRM/KMS for display output
- Smithay for Wayland compositor
- wgpu with Vulkan backend

```rust
// DMA-BUF import
#[cfg(unix)]
fn import_dma_buf(&mut self, fd: RawFd, format: TextureFormat, offset: u64, stride: u32) -> PlatformResult<TextureHandle>;
```

## Frame Loop

```rust
let frame_time = Duration::from_secs_f64(1.0 / fps);

while running {
    let frame_start = Instant::now();

    // Process input
    input.process_events(&mut surfaces)?;

    // Update surfaces
    surfaces.update()?;

    // Render
    renderer.begin_frame()?;
    renderer.clear(Color::BLACK)?;
    renderer.render(&surfaces.to_render_surfaces())?;
    renderer.end_frame()?;
    renderer.present().await?;

    // Frame timing
    let elapsed = frame_start.elapsed();
    if elapsed < frame_time {
        tokio::time::sleep(frame_time - elapsed).await;
    }
}
```

## CompositorConfig

```rust
pub struct CompositorConfig {
    pub backend: String,    // "drm" or "winit"
    pub renderer: String,   // "wgpu" or "vulkan"
    pub fps: u32,           // Target frame rate
    pub vsync: bool,
    pub width: u32,
    pub height: u32,
}
```

## egui Integration

egui runs as a layer on top of video surfaces:

```rust
// Paint egui UI
let full_output = egui_ctx.run(raw_input, |ctx| {
    // UI code here
});

// Render egui output to wgpu
egui_renderer.render(&device, &queue, &full_output);
```

## Memory Management

Track GPU memory usage:

```rust
pub struct MemoryUsage {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub texture_count: u32,
    pub buffer_count: u32,
}
```

## Pixel Formats

Prefer these formats for video textures:
- `NV12` - Hardware decode native format
- `BGRA8` - Direct render (slower, uses conversion)
