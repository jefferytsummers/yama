//! GPU memory allocator trait.
//!
//! This trait abstracts over GPU memory allocation and sharing:
//! - Metal buffers (Apple)
//! - Vulkan memory (cross-platform)
//! - CUDA memory (NVIDIA)
//! - DMA-BUF sharing (Linux)

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::PlatformResult;
use crate::types::Size;

/// Buffer usage flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BufferUsage(u32);

impl BufferUsage {
    /// No specific usage.
    pub const NONE: Self = Self(0);
    /// Buffer can be used as texture source.
    pub const TEXTURE_SRC: Self = Self(1 << 0);
    /// Buffer can be used as render target.
    pub const RENDER_TARGET: Self = Self(1 << 1);
    /// Buffer can be used for video decoding output.
    pub const VIDEO_DECODE: Self = Self(1 << 2);
    /// Buffer can be used for video encoding input.
    pub const VIDEO_ENCODE: Self = Self(1 << 3);
    /// Buffer can be mapped for CPU access.
    pub const CPU_READ: Self = Self(1 << 4);
    /// Buffer can be written by CPU.
    pub const CPU_WRITE: Self = Self(1 << 5);
    /// Buffer can be exported as DMA-BUF.
    pub const EXPORTABLE: Self = Self(1 << 6);
    /// Buffer was imported from external source.
    pub const IMPORTED: Self = Self(1 << 7);

    /// Combine usage flags.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check if usage contains a flag.
    #[must_use]
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }
}

impl std::ops::BitOr for BufferUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

/// GPU buffer handle.
///
/// This is a reference-counted handle to GPU memory.
/// The underlying memory is freed when all handles are dropped.
#[derive(Debug, Clone)]
pub struct GpuBuffer {
    /// Internal buffer data.
    inner: Arc<GpuBufferInner>,
}

#[derive(Debug)]
struct GpuBufferInner {
    /// Unique buffer ID.
    id: u64,
    /// Buffer size in bytes.
    size: usize,
    /// Buffer usage flags.
    usage: BufferUsage,
    /// Optional dimensions (for 2D buffers).
    dimensions: Option<Size>,
    /// Platform-specific handle (opaque).
    platform_handle: u64,
    /// DMA-BUF fd if exported (Linux only).
    #[cfg(unix)]
    dma_buf_fd: Option<std::os::unix::io::RawFd>,
}

impl GpuBuffer {
    /// Create a new buffer handle.
    #[must_use]
    pub fn new(id: u64, size: usize, usage: BufferUsage, platform_handle: u64) -> Self {
        Self {
            inner: Arc::new(GpuBufferInner {
                id,
                size,
                usage,
                dimensions: None,
                platform_handle,
                #[cfg(unix)]
                dma_buf_fd: None,
            }),
        }
    }

    /// Create a buffer handle with dimensions.
    #[must_use]
    pub fn new_2d(
        id: u64,
        size: usize,
        dimensions: Size,
        usage: BufferUsage,
        platform_handle: u64,
    ) -> Self {
        Self {
            inner: Arc::new(GpuBufferInner {
                id,
                size,
                usage,
                dimensions: Some(dimensions),
                platform_handle,
                #[cfg(unix)]
                dma_buf_fd: None,
            }),
        }
    }

    /// Get buffer ID.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.inner.id
    }

    /// Get buffer size in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.inner.size
    }

    /// Get buffer usage flags.
    #[must_use]
    pub fn usage(&self) -> BufferUsage {
        self.inner.usage
    }

    /// Get buffer dimensions (if 2D).
    #[must_use]
    pub fn dimensions(&self) -> Option<Size> {
        self.inner.dimensions
    }

    /// Get platform-specific handle.
    #[must_use]
    pub fn platform_handle(&self) -> u64 {
        self.inner.platform_handle
    }

    /// Check if buffer can be exported as DMA-BUF.
    #[must_use]
    pub fn is_exportable(&self) -> bool {
        self.inner.usage.contains(BufferUsage::EXPORTABLE)
    }

    /// Check if buffer was imported.
    #[must_use]
    pub fn is_imported(&self) -> bool {
        self.inner.usage.contains(BufferUsage::IMPORTED)
    }

    /// Get reference count.
    #[must_use]
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }
}

/// Allocator capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocatorCapabilities {
    /// Allocator name/identifier.
    pub name: String,
    /// Maximum allocation size.
    pub max_allocation_size: u64,
    /// Total GPU memory available.
    pub total_memory: u64,
    /// Whether DMA-BUF export is supported.
    pub supports_dma_buf_export: bool,
    /// Whether DMA-BUF import is supported.
    pub supports_dma_buf_import: bool,
    /// Whether CPU mapping is supported.
    pub supports_cpu_mapping: bool,
    /// Memory alignment requirement.
    pub alignment: u64,
}

/// Mapped buffer for CPU access.
pub struct MappedBuffer<'a> {
    /// Pointer to mapped data.
    data: *mut u8,
    /// Size of mapped region.
    size: usize,
    /// Whether mapping is writable.
    writable: bool,
    /// Lifetime marker.
    _marker: std::marker::PhantomData<&'a mut [u8]>,
}

impl<'a> MappedBuffer<'a> {
    /// Create a new mapped buffer.
    ///
    /// # Safety
    /// The pointer must be valid for the lifetime and size specified.
    #[must_use]
    pub unsafe fn new(data: *mut u8, size: usize, writable: bool) -> Self {
        Self {
            data,
            size,
            writable,
            _marker: std::marker::PhantomData,
        }
    }

    /// Get a slice of the mapped data.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.data, self.size) }
    }

    /// Get a mutable slice of the mapped data.
    ///
    /// # Panics
    /// Panics if the mapping is not writable.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        assert!(self.writable, "Buffer is not mapped for writing");
        unsafe { std::slice::from_raw_parts_mut(self.data, self.size) }
    }
}

/// GPU memory allocator abstraction.
///
/// This trait defines the interface for platform-specific GPU memory allocators.
/// Implementations handle buffer allocation, mapping, and DMA-BUF sharing.
pub trait GpuAllocator: Send + Sync {
    /// Get allocator capabilities.
    fn capabilities(&self) -> &AllocatorCapabilities;

    /// Allocate a GPU buffer.
    fn allocate(&mut self, size: usize, usage: BufferUsage) -> PlatformResult<GpuBuffer>;

    /// Allocate a 2D GPU buffer (for images/textures).
    fn allocate_2d(
        &mut self,
        width: u32,
        height: u32,
        bytes_per_pixel: u32,
        usage: BufferUsage,
    ) -> PlatformResult<GpuBuffer> {
        let size = (width * height * bytes_per_pixel) as usize;
        let mut buffer = self.allocate(size, usage)?;
        // Note: In a real implementation, we'd set dimensions on the buffer
        Ok(buffer)
    }

    /// Free a GPU buffer.
    ///
    /// Note: Buffers are reference-counted, so this only frees if ref count is 1.
    fn free(&mut self, buffer: GpuBuffer) -> PlatformResult<()>;

    /// Import a DMA-BUF as a GPU buffer.
    #[cfg(unix)]
    fn import_dma_buf(
        &mut self,
        fd: std::os::unix::io::RawFd,
        size: usize,
        usage: BufferUsage,
    ) -> PlatformResult<GpuBuffer>;

    /// Export a GPU buffer as a DMA-BUF.
    #[cfg(unix)]
    fn export_dma_buf(&self, buffer: &GpuBuffer) -> PlatformResult<std::os::unix::io::RawFd>;

    /// Map a buffer for CPU access.
    fn map<'a>(&'a mut self, buffer: &'a GpuBuffer, writable: bool) -> PlatformResult<MappedBuffer<'a>>;

    /// Unmap a buffer.
    fn unmap(&mut self, buffer: &GpuBuffer) -> PlatformResult<()>;

    /// Copy data from CPU to GPU buffer.
    fn upload(&mut self, buffer: &GpuBuffer, offset: usize, data: &[u8]) -> PlatformResult<()>;

    /// Copy data from GPU buffer to CPU.
    fn download(&mut self, buffer: &GpuBuffer, offset: usize, size: usize) -> PlatformResult<Vec<u8>>;

    /// Copy between GPU buffers.
    fn copy(
        &mut self,
        src: &GpuBuffer,
        src_offset: usize,
        dst: &GpuBuffer,
        dst_offset: usize,
        size: usize,
    ) -> PlatformResult<()>;

    /// Get memory usage statistics.
    fn memory_stats(&self) -> MemoryStats;
}

/// Memory usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total memory available.
    pub total_bytes: u64,
    /// Memory currently allocated.
    pub allocated_bytes: u64,
    /// Number of active allocations.
    pub allocation_count: u32,
    /// Peak memory usage.
    pub peak_bytes: u64,
}

impl MemoryStats {
    /// Get free memory.
    #[must_use]
    pub const fn free_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.allocated_bytes)
    }

    /// Get usage percentage.
    #[must_use]
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.allocated_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
        }
    }
}
