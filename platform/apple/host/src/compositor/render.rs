//! Metal renderer implementation for Apple Silicon.
//!
//! This module implements the platform `Renderer` trait using wgpu with
//! Metal backend for Apple Silicon GPUs.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, info, warn};

use yama_platform_traits::{
    Color, GpuBuffer, PixelFormat, PlatformError, PlatformResult, Rect, Size,
};
use yama_platform_traits::renderer::{
    MemoryUsage, RenderSurface, Renderer, RendererCapabilities, TextureFormat, TextureHandle,
};

use super::CompositorConfig;

/// Metal renderer using wgpu.
pub struct MetalRenderer {
    /// Renderer capabilities.
    capabilities: RendererCapabilities,
    /// Configuration.
    config: CompositorConfig,
    /// Current output size.
    output_size: Size,
    /// Texture storage.
    textures: HashMap<u64, TextureInfo>,
    /// Next texture ID.
    next_texture_id: u64,
    /// Memory tracking.
    memory_used: u64,
    /// wgpu instance (would be real in production).
    _instance: Option<()>,
}

/// Information about an allocated texture.
#[derive(Debug)]
struct TextureInfo {
    format: TextureFormat,
    size_bytes: u64,
}

impl MetalRenderer {
    /// Create a new Metal renderer.
    pub fn new(config: &CompositorConfig) -> Result<Self> {
        info!("Initializing Metal renderer via wgpu");

        // Detect Metal capabilities
        // In a real implementation, we'd query the GPU here
        let capabilities = RendererCapabilities {
            name: "Metal (wgpu)".to_string(),
            api: "Metal".to_string(),
            max_texture_size: 16384,
            supported_formats: vec![
                PixelFormat::Rgba8,
                PixelFormat::Bgra8,
                PixelFormat::Nv12,
                PixelFormat::Rgba16Float,
            ],
            supports_dma_buf_import: false, // macOS doesn't use DMA-BUF
            supports_dma_buf_export: false,
            max_render_targets: 8,
            gpu_memory_bytes: Self::query_gpu_memory(),
        };

        info!(
            "Metal renderer initialized: max_texture={}x{}, gpu_memory={}MB",
            capabilities.max_texture_size,
            capabilities.max_texture_size,
            capabilities.gpu_memory_bytes / (1024 * 1024)
        );

        Ok(Self {
            capabilities,
            config: config.clone(),
            output_size: Size::new(config.width, config.height),
            textures: HashMap::new(),
            next_texture_id: 1,
            memory_used: 0,
            _instance: None,
        })
    }

    /// Query available GPU memory.
    fn query_gpu_memory() -> u64 {
        // On Apple Silicon, GPU uses unified memory
        // Query total system memory and assume GPU can use half
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("sysctl")
                .args(["-n", "hw.memsize"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|mem| mem / 2)
                .unwrap_or(8 * 1024 * 1024 * 1024) // Default 8GB
        }
        #[cfg(not(target_os = "macos"))]
        {
            8 * 1024 * 1024 * 1024
        }
    }

    /// Allocate a texture ID.
    fn allocate_texture_id(&mut self) -> u64 {
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        id
    }

    /// Calculate texture size in bytes.
    fn texture_size_bytes(format: &TextureFormat) -> u64 {
        let bpp = format.format.bytes_per_pixel();
        if bpp > 0 {
            (format.width as u64) * (format.height as u64) * (bpp as u64)
        } else {
            // Planar formats like NV12
            format.format.buffer_size(format.width, format.height) as u64
        }
    }
}

#[async_trait]
impl Renderer for MetalRenderer {
    fn capabilities(&self) -> &RendererCapabilities {
        &self.capabilities
    }

    fn create_texture(
        &mut self,
        format: TextureFormat,
        data: Option<&[u8]>,
    ) -> PlatformResult<TextureHandle> {
        // Validate format
        if !self.capabilities.supported_formats.contains(&format.format) {
            return Err(PlatformError::TextureError(format!(
                "Unsupported format: {:?}",
                format.format
            )));
        }

        // Validate size
        if format.width > self.capabilities.max_texture_size
            || format.height > self.capabilities.max_texture_size
        {
            return Err(PlatformError::TextureError(format!(
                "Texture size {}x{} exceeds maximum {}",
                format.width, format.height, self.capabilities.max_texture_size
            )));
        }

        let size_bytes = Self::texture_size_bytes(&format);

        // In a real implementation:
        // 1. Create Metal texture descriptor
        // 2. Allocate texture
        // 3. Upload data if provided

        let id = self.allocate_texture_id();
        self.textures.insert(id, TextureInfo { format, size_bytes });
        self.memory_used += size_bytes;

        debug!(
            "Created texture {}: {:?} {}x{}, {} bytes",
            id, format.format, format.width, format.height, size_bytes
        );

        if let Some(_data) = data {
            // Upload initial data
            debug!("Uploading initial texture data for texture {}", id);
        }

        Ok(TextureHandle(id))
    }

    fn create_texture_from_buffer(
        &mut self,
        format: TextureFormat,
        buffer: &GpuBuffer,
    ) -> PlatformResult<TextureHandle> {
        // In a real implementation:
        // 1. Create Metal texture with shared storage mode
        // 2. Point texture to buffer's memory

        let id = self.allocate_texture_id();
        let size_bytes = buffer.size() as u64;

        self.textures.insert(id, TextureInfo { format, size_bytes });
        // Don't add to memory_used since buffer already counts it

        debug!(
            "Created texture {} from buffer {}: {:?} {}x{}",
            id,
            buffer.id(),
            format.format,
            format.width,
            format.height
        );

        Ok(TextureHandle(id))
    }

    #[cfg(unix)]
    fn import_dma_buf(
        &mut self,
        fd: std::os::unix::io::RawFd,
        format: TextureFormat,
        _offset: u64,
        _stride: u32,
    ) -> PlatformResult<TextureHandle> {
        // DMA-BUF import is primarily a Linux feature for zero-copy sharing.
        // On macOS, we would typically use IOSurface instead.
        // For now, return an error indicating this is not supported.
        debug!(
            "import_dma_buf called with fd={}, format={:?} - not supported on macOS",
            fd, format.format
        );
        Err(PlatformError::unsupported(
            "DMA-BUF import is not supported on macOS; use IOSurface instead",
        ))
    }

    fn update_texture(
        &mut self,
        texture: TextureHandle,
        rect: Option<Rect>,
        data: &[u8],
    ) -> PlatformResult<()> {
        let info = self
            .textures
            .get(&texture.0)
            .ok_or_else(|| PlatformError::NotFound("Texture not found".to_string()))?;

        // In a real implementation:
        // 1. Get Metal texture
        // 2. Replace region with new data

        let region = rect.unwrap_or(Rect::from_size(info.format.width, info.format.height));
        debug!(
            "Updating texture {} region {:?} with {} bytes",
            texture.0,
            region,
            data.len()
        );

        Ok(())
    }

    fn destroy_texture(&mut self, texture: TextureHandle) -> PlatformResult<()> {
        if let Some(info) = self.textures.remove(&texture.0) {
            self.memory_used = self.memory_used.saturating_sub(info.size_bytes);
            debug!("Destroyed texture {}", texture.0);
            Ok(())
        } else {
            Err(PlatformError::NotFound("Texture not found".to_string()))
        }
    }

    fn begin_frame(&mut self) -> PlatformResult<()> {
        // In a real implementation:
        // 1. Acquire next drawable from CAMetalLayer
        // 2. Create command buffer
        // 3. Begin render pass

        debug!("Beginning frame");
        Ok(())
    }

    fn clear(&mut self, color: Color) -> PlatformResult<()> {
        // In a real implementation:
        // Set clear color on render pass descriptor

        debug!("Clearing with color: {:?}", color);
        Ok(())
    }

    fn render(&mut self, surfaces: &[RenderSurface]) -> PlatformResult<()> {
        // Sort surfaces by z-order
        let mut sorted_surfaces: Vec<_> = surfaces.iter().collect();
        sorted_surfaces.sort_by_key(|s| s.z_order);

        // In a real implementation:
        // For each surface:
        // 1. Bind texture
        // 2. Set transform and opacity uniforms
        // 3. Draw quad

        debug!("Rendering {} surfaces", surfaces.len());
        for surface in sorted_surfaces {
            if !self.textures.contains_key(&surface.texture.0) {
                warn!("Skipping invalid texture {}", surface.texture.0);
                continue;
            }
            debug!(
                "  - texture {} at {:?}, z={}, opacity={}",
                surface.texture.0, surface.dst_rect, surface.z_order, surface.opacity
            );
        }

        Ok(())
    }

    fn end_frame(&mut self) -> PlatformResult<()> {
        // In a real implementation:
        // 1. End render pass
        // 2. Commit command buffer

        debug!("Ending frame");
        Ok(())
    }

    async fn present(&mut self) -> PlatformResult<()> {
        // In a real implementation:
        // 1. Present drawable
        // 2. Wait for completion if needed

        debug!("Presenting frame");
        Ok(())
    }

    fn set_output_size(&mut self, size: Size) -> PlatformResult<()> {
        self.output_size = size;
        info!("Output size set to {}x{}", size.width, size.height);
        Ok(())
    }

    fn output_size(&self) -> Size {
        self.output_size
    }

    fn flush(&mut self) -> PlatformResult<()> {
        // In a real implementation:
        // Flush pending GPU commands

        debug!("Flushing renderer");
        Ok(())
    }

    async fn wait_idle(&mut self) -> PlatformResult<()> {
        // In a real implementation:
        // Wait for all GPU work to complete

        debug!("Waiting for GPU idle");
        Ok(())
    }

    fn memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            total_bytes: self.capabilities.gpu_memory_bytes,
            used_bytes: self.memory_used,
            texture_count: self.textures.len() as u32,
            buffer_count: 0,
        }
    }
}

/// Create the platform-appropriate renderer.
pub fn create_renderer(config: &CompositorConfig) -> Result<Box<dyn Renderer>> {
    Ok(Box::new(MetalRenderer::new(config)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CompositorConfig {
        CompositorConfig {
            width: 1920,
            height: 1080,
            renderer: "metal".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_create_texture() {
        let mut renderer = MetalRenderer::new(&test_config()).unwrap();

        let format = TextureFormat::new_2d(PixelFormat::Rgba8, 256, 256);
        let handle = renderer.create_texture(format, None).unwrap();

        assert!(handle.is_valid());
        assert_eq!(renderer.textures.len(), 1);
    }

    #[test]
    fn test_destroy_texture() {
        let mut renderer = MetalRenderer::new(&test_config()).unwrap();

        let format = TextureFormat::new_2d(PixelFormat::Rgba8, 256, 256);
        let handle = renderer.create_texture(format, None).unwrap();

        renderer.destroy_texture(handle).unwrap();
        assert_eq!(renderer.textures.len(), 0);
    }
}
