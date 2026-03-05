//! Renderer trait for GPU rendering abstraction.
//!
//! This trait abstracts over different GPU APIs:
//! - Metal (Apple Silicon)
//! - Vulkan (cross-platform)
//! - EGL/OpenGL ES (Jetson, embedded)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::PlatformResult;
use crate::types::{Color, PixelFormat, Rect, Size, Transform};
use crate::allocator::GpuBuffer;

/// Handle to a texture on the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u64);

impl TextureHandle {
    /// Invalid/null texture handle.
    pub const INVALID: Self = Self(0);

    /// Check if handle is valid.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Texture format description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextureFormat {
    /// Pixel format.
    pub format: PixelFormat,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Number of mip levels.
    pub mip_levels: u32,
    /// Number of array layers.
    pub array_layers: u32,
}

impl TextureFormat {
    /// Create a simple 2D texture format.
    #[must_use]
    pub const fn new_2d(format: PixelFormat, width: u32, height: u32) -> Self {
        Self {
            format,
            width,
            height,
            mip_levels: 1,
            array_layers: 1,
        }
    }
}

/// A surface to be rendered.
#[derive(Debug, Clone)]
pub struct RenderSurface {
    /// Texture to render.
    pub texture: TextureHandle,
    /// Source rectangle within texture (None = entire texture).
    pub src_rect: Option<Rect>,
    /// Destination rectangle on screen.
    pub dst_rect: Rect,
    /// Transform to apply.
    pub transform: Transform,
    /// Opacity (0.0 - 1.0).
    pub opacity: f32,
    /// Z-order (higher = on top).
    pub z_order: i32,
}

impl RenderSurface {
    /// Create a new render surface.
    #[must_use]
    pub fn new(texture: TextureHandle, dst_rect: Rect) -> Self {
        Self {
            texture,
            src_rect: None,
            dst_rect,
            transform: Transform::IDENTITY,
            opacity: 1.0,
            z_order: 0,
        }
    }

    /// Set the source rectangle.
    #[must_use]
    pub const fn with_src_rect(mut self, rect: Rect) -> Self {
        self.src_rect = Some(rect);
        self
    }

    /// Set opacity.
    #[must_use]
    pub const fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Set z-order.
    #[must_use]
    pub const fn with_z_order(mut self, z: i32) -> Self {
        self.z_order = z;
        self
    }
}

/// Renderer capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererCapabilities {
    /// Renderer name/identifier.
    pub name: String,
    /// API being used (Metal, Vulkan, OpenGL, etc.).
    pub api: String,
    /// Maximum texture size.
    pub max_texture_size: u32,
    /// Supported texture formats.
    pub supported_formats: Vec<PixelFormat>,
    /// Whether DMA-BUF import is supported.
    pub supports_dma_buf_import: bool,
    /// Whether DMA-BUF export is supported.
    pub supports_dma_buf_export: bool,
    /// Maximum number of simultaneous render targets.
    pub max_render_targets: u32,
    /// GPU memory available (bytes, 0 if unknown).
    pub gpu_memory_bytes: u64,
}

/// GPU rendering abstraction.
///
/// This trait defines the interface for platform-specific GPU renderers.
/// Implementations handle texture management, rendering, and presentation.
#[async_trait]
pub trait Renderer: Send + Sync {
    /// Get renderer capabilities.
    fn capabilities(&self) -> &RendererCapabilities;

    /// Create a texture from raw pixel data.
    fn create_texture(
        &mut self,
        format: TextureFormat,
        data: Option<&[u8]>,
    ) -> PlatformResult<TextureHandle>;

    /// Create a texture from a GPU buffer.
    fn create_texture_from_buffer(
        &mut self,
        format: TextureFormat,
        buffer: &GpuBuffer,
    ) -> PlatformResult<TextureHandle>;

    /// Import a texture from a DMA-BUF file descriptor.
    ///
    /// This is the primary zero-copy path for video frames.
    #[cfg(unix)]
    fn import_dma_buf(
        &mut self,
        fd: std::os::unix::io::RawFd,
        format: TextureFormat,
        offset: u64,
        stride: u32,
    ) -> PlatformResult<TextureHandle>;

    /// Update texture data.
    fn update_texture(
        &mut self,
        texture: TextureHandle,
        rect: Option<Rect>,
        data: &[u8],
    ) -> PlatformResult<()>;

    /// Destroy a texture.
    fn destroy_texture(&mut self, texture: TextureHandle) -> PlatformResult<()>;

    /// Begin a new frame.
    ///
    /// Must be called before any rendering operations.
    fn begin_frame(&mut self) -> PlatformResult<()>;

    /// Clear the render target with a color.
    fn clear(&mut self, color: Color) -> PlatformResult<()>;

    /// Render surfaces to the current target.
    ///
    /// Surfaces are rendered in z-order (lowest first).
    fn render(&mut self, surfaces: &[RenderSurface]) -> PlatformResult<()>;

    /// End the current frame.
    ///
    /// Must be called after all rendering operations.
    fn end_frame(&mut self) -> PlatformResult<()>;

    /// Present the frame to the display.
    ///
    /// This submits the rendered frame for display.
    async fn present(&mut self) -> PlatformResult<()>;

    /// Set the output size (viewport).
    fn set_output_size(&mut self, size: Size) -> PlatformResult<()>;

    /// Get current output size.
    fn output_size(&self) -> Size;

    /// Flush pending operations.
    fn flush(&mut self) -> PlatformResult<()>;

    /// Wait for GPU to complete all pending operations.
    async fn wait_idle(&mut self) -> PlatformResult<()>;

    /// Get GPU memory usage statistics.
    fn memory_usage(&self) -> MemoryUsage;
}

/// GPU memory usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Total GPU memory in bytes.
    pub total_bytes: u64,
    /// Used GPU memory in bytes.
    pub used_bytes: u64,
    /// Number of allocated textures.
    pub texture_count: u32,
    /// Number of allocated buffers.
    pub buffer_count: u32,
}

impl MemoryUsage {
    /// Get available memory in bytes.
    #[must_use]
    pub const fn available_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.used_bytes)
    }

    /// Get usage percentage.
    #[must_use]
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
        }
    }
}
