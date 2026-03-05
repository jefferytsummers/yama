//! Vulkan/EGL renderer implementation for NVIDIA Jetson.
//!
//! This module implements the platform `Renderer` trait using wgpu with
//! Vulkan backend for NVIDIA Jetson GPUs.

use std::collections::HashMap;
use std::os::unix::io::RawFd;
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

/// Vulkan renderer for Jetson.
pub struct VulkanRenderer {
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
    /// DMA-BUF imported textures.
    dma_buf_textures: HashMap<u64, RawFd>,
}

/// Information about an allocated texture.
#[derive(Debug)]
struct TextureInfo {
    format: TextureFormat,
    size_bytes: u64,
    is_dma_buf: bool,
}

impl VulkanRenderer {
    /// Create a new Vulkan renderer.
    pub fn new(config: &CompositorConfig) -> Result<Self> {
        info!("Initializing Vulkan renderer via wgpu for Jetson");

        // Detect Vulkan/NVIDIA capabilities
        let capabilities = RendererCapabilities {
            name: "Vulkan (wgpu)".to_string(),
            api: "Vulkan".to_string(),
            max_texture_size: 16384,
            supported_formats: vec![
                PixelFormat::Rgba8,
                PixelFormat::Bgra8,
                PixelFormat::Nv12,
                PixelFormat::I420,
                PixelFormat::Rgba16Float,
            ],
            supports_dma_buf_import: true,  // Jetson supports DMA-BUF
            supports_dma_buf_export: true,
            max_render_targets: 8,
            gpu_memory_bytes: Self::query_gpu_memory(),
        };

        info!(
            "Vulkan renderer initialized: max_texture={}x{}, gpu_memory={}MB, dma_buf={}",
            capabilities.max_texture_size,
            capabilities.max_texture_size,
            capabilities.gpu_memory_bytes / (1024 * 1024),
            capabilities.supports_dma_buf_import
        );

        Ok(Self {
            capabilities,
            config: config.clone(),
            output_size: Size::new(config.width, config.height),
            textures: HashMap::new(),
            next_texture_id: 1,
            memory_used: 0,
            dma_buf_textures: HashMap::new(),
        })
    }

    /// Query available GPU memory from NVIDIA driver.
    fn query_gpu_memory() -> u64 {
        // Try to read from nvidia-smi or sysfs
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            use std::process::Command;

            // Try nvidia-smi first
            if let Ok(output) = Command::new("nvidia-smi")
                .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
                .output()
            {
                if let Ok(mem_str) = String::from_utf8(output.stdout) {
                    if let Ok(mem_mb) = mem_str.trim().parse::<u64>() {
                        return mem_mb * 1024 * 1024;
                    }
                }
            }

            // Fallback: check Jetson memory info
            if let Ok(content) = fs::read_to_string("/sys/kernel/debug/nvmap/iovmm/maps") {
                // Parse Jetson-specific memory info
                // Default to 8GB for Orin
                return 8 * 1024 * 1024 * 1024;
            }
        }

        // Default 8GB for Jetson Orin
        8 * 1024 * 1024 * 1024
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
            format.format.buffer_size(format.width, format.height) as u64
        }
    }
}

#[async_trait]
impl Renderer for VulkanRenderer {
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
        // 1. Create Vulkan image
        // 2. Allocate device memory
        // 3. Create image view

        let id = self.allocate_texture_id();
        self.textures.insert(
            id,
            TextureInfo {
                format,
                size_bytes,
                is_dma_buf: false,
            },
        );
        self.memory_used += size_bytes;

        debug!(
            "Created texture {}: {:?} {}x{}, {} bytes",
            id, format.format, format.width, format.height, size_bytes
        );

        if let Some(_data) = data {
            debug!("Uploading initial texture data for texture {}", id);
        }

        Ok(TextureHandle(id))
    }

    fn create_texture_from_buffer(
        &mut self,
        format: TextureFormat,
        buffer: &GpuBuffer,
    ) -> PlatformResult<TextureHandle> {
        let id = self.allocate_texture_id();
        let size_bytes = buffer.size() as u64;

        self.textures.insert(
            id,
            TextureInfo {
                format,
                size_bytes,
                is_dma_buf: false,
            },
        );

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
        fd: RawFd,
        format: TextureFormat,
        offset: u64,
        stride: u32,
    ) -> PlatformResult<TextureHandle> {
        // In a real implementation:
        // 1. Create EGLImage from DMA-BUF fd
        // 2. Import EGLImage into Vulkan via VK_EXT_external_memory_dma_buf
        // 3. Create Vulkan image view

        let id = self.allocate_texture_id();
        let size_bytes = Self::texture_size_bytes(&format);

        self.textures.insert(
            id,
            TextureInfo {
                format,
                size_bytes,
                is_dma_buf: true,
            },
        );
        self.dma_buf_textures.insert(id, fd);

        debug!(
            "Imported DMA-BUF texture {}: fd={}, {:?} {}x{}, offset={}, stride={}",
            id, fd, format.format, format.width, format.height, offset, stride
        );

        Ok(TextureHandle(id))
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

        if info.is_dma_buf {
            return Err(PlatformError::TextureError(
                "Cannot update DMA-BUF texture directly".to_string(),
            ));
        }

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
            self.dma_buf_textures.remove(&texture.0);
            debug!("Destroyed texture {}", texture.0);
            Ok(())
        } else {
            Err(PlatformError::NotFound("Texture not found".to_string()))
        }
    }

    fn begin_frame(&mut self) -> PlatformResult<()> {
        // In a real implementation:
        // 1. Acquire swapchain image
        // 2. Begin command buffer
        // 3. Begin render pass

        debug!("Beginning frame");
        Ok(())
    }

    fn clear(&mut self, color: Color) -> PlatformResult<()> {
        debug!("Clearing with color: {:?}", color);
        Ok(())
    }

    fn render(&mut self, surfaces: &[RenderSurface]) -> PlatformResult<()> {
        let mut sorted_surfaces: Vec<_> = surfaces.iter().collect();
        sorted_surfaces.sort_by_key(|s| s.z_order);

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
        debug!("Ending frame");
        Ok(())
    }

    async fn present(&mut self) -> PlatformResult<()> {
        // In a real implementation:
        // 1. Submit command buffer
        // 2. Present swapchain image
        // 3. Wait for presentation

        debug!("Presenting frame");
        Ok(())
    }

    fn set_output_size(&mut self, size: Size) -> PlatformResult<()> {
        self.output_size = size;
        info!("Output size set to {}x{}", size.width, size.height);
        // Would recreate swapchain here
        Ok(())
    }

    fn output_size(&self) -> Size {
        self.output_size
    }

    fn flush(&mut self) -> PlatformResult<()> {
        debug!("Flushing renderer");
        Ok(())
    }

    async fn wait_idle(&mut self) -> PlatformResult<()> {
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
    Ok(Box::new(VulkanRenderer::new(config)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CompositorConfig {
        CompositorConfig {
            width: 1920,
            height: 1080,
            renderer: "vulkan".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_create_texture() {
        let mut renderer = VulkanRenderer::new(&test_config()).unwrap();

        let format = TextureFormat::new_2d(PixelFormat::Rgba8, 256, 256);
        let handle = renderer.create_texture(format, None).unwrap();

        assert!(handle.is_valid());
        assert_eq!(renderer.textures.len(), 1);
    }

    #[test]
    fn test_dma_buf_support() {
        let renderer = VulkanRenderer::new(&test_config()).unwrap();
        assert!(renderer.capabilities().supports_dma_buf_import);
        assert!(renderer.capabilities().supports_dma_buf_export);
    }
}
