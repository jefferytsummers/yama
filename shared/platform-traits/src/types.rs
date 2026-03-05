//! Common types used across platform traits.

use serde::{Deserialize, Serialize};

/// Pixel format for buffers and textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PixelFormat {
    /// 8-bit RGBA (32 bits per pixel)
    Rgba8,
    /// 8-bit BGRA (32 bits per pixel)
    Bgra8,
    /// NV12 (YUV 4:2:0, 12 bits per pixel) - common for video
    Nv12,
    /// I420/YV12 (YUV 4:2:0 planar)
    I420,
    /// P010 (10-bit YUV 4:2:0) - HDR video
    P010,
    /// 16-bit RGBA float (64 bits per pixel)
    Rgba16Float,
    /// 32-bit RGBA float (128 bits per pixel)
    Rgba32Float,
}

impl PixelFormat {
    /// Get bytes per pixel (for packed formats) or 0 for planar.
    #[must_use]
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Rgba8 | Self::Bgra8 => 4,
            Self::Nv12 | Self::I420 => 0, // Planar
            Self::P010 => 0,              // Planar
            Self::Rgba16Float => 8,
            Self::Rgba32Float => 16,
        }
    }

    /// Check if format is planar (YUV).
    #[must_use]
    pub const fn is_planar(&self) -> bool {
        matches!(self, Self::Nv12 | Self::I420 | Self::P010)
    }

    /// Calculate buffer size for given dimensions.
    #[must_use]
    pub const fn buffer_size(&self, width: u32, height: u32) -> usize {
        let w = width as usize;
        let h = height as usize;
        match self {
            Self::Rgba8 | Self::Bgra8 => w * h * 4,
            Self::Nv12 => w * h + (w * h / 2), // Y + UV interleaved
            Self::I420 => w * h + (w * h / 2), // Y + U + V planes
            Self::P010 => (w * h + w * h / 2) * 2, // 16-bit per sample
            Self::Rgba16Float => w * h * 8,
            Self::Rgba32Float => w * h * 16,
        }
    }
}

/// Rectangle definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rect {
    /// X coordinate of top-left corner.
    pub x: i32,
    /// Y coordinate of top-left corner.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl Rect {
    /// Create a new rectangle.
    #[must_use]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Create a rectangle at origin.
    #[must_use]
    pub const fn from_size(width: u32, height: u32) -> Self {
        Self { x: 0, y: 0, width, height }
    }

    /// Check if rectangle contains a point.
    #[must_use]
    pub const fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width as i32
            && py >= self.y
            && py < self.y + self.height as i32
    }

    /// Calculate area.
    #[must_use]
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// 2D size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Size {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl Size {
    /// Create a new size.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// 2D point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Point {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

impl Point {
    /// Create a new point.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Transform matrix (3x3 for 2D transforms).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    /// Matrix elements in row-major order.
    pub matrix: [[f32; 3]; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        matrix: [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
    };

    /// Create a translation transform.
    #[must_use]
    pub const fn translate(x: f32, y: f32) -> Self {
        Self {
            matrix: [
                [1.0, 0.0, x],
                [0.0, 1.0, y],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    /// Create a scale transform.
    #[must_use]
    pub const fn scale(sx: f32, sy: f32) -> Self {
        Self {
            matrix: [
                [sx, 0.0, 0.0],
                [0.0, sy, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }
}

/// Color in linear RGBA.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    /// Red component (0.0 - 1.0).
    pub r: f32,
    /// Green component (0.0 - 1.0).
    pub g: f32,
    /// Blue component (0.0 - 1.0).
    pub b: f32,
    /// Alpha component (0.0 - 1.0).
    pub a: f32,
}

impl Color {
    /// Transparent black.
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    /// Opaque black.
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    /// Opaque white.
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    /// Create a new color.
    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create from 8-bit RGBA values.
    #[must_use]
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: f32::from(r) / 255.0,
            g: f32::from(g) / 255.0,
            b: f32::from(b) / 255.0,
            a: f32::from(a) / 255.0,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

/// File descriptor wrapper for DMA-BUF handles.
#[cfg(unix)]
#[derive(Debug)]
pub struct DmaBufFd {
    fd: std::os::unix::io::RawFd,
    owned: bool,
}

#[cfg(unix)]
impl DmaBufFd {
    /// Create from a raw file descriptor (takes ownership).
    ///
    /// # Safety
    /// The fd must be a valid DMA-BUF file descriptor.
    #[must_use]
    pub const unsafe fn from_raw(fd: std::os::unix::io::RawFd) -> Self {
        Self { fd, owned: true }
    }

    /// Create from a raw file descriptor (does not take ownership).
    ///
    /// # Safety
    /// The fd must be valid for the lifetime of this wrapper.
    #[must_use]
    pub const unsafe fn from_raw_borrowed(fd: std::os::unix::io::RawFd) -> Self {
        Self { fd, owned: false }
    }

    /// Get the raw file descriptor.
    #[must_use]
    pub const fn as_raw(&self) -> std::os::unix::io::RawFd {
        self.fd
    }

    /// Duplicate the file descriptor.
    pub fn try_clone(&self) -> std::io::Result<Self> {
        let new_fd = unsafe { libc::dup(self.fd) };
        if new_fd < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Self { fd: new_fd, owned: true })
        }
    }
}

#[cfg(unix)]
impl Drop for DmaBufFd {
    fn drop(&mut self) {
        if self.owned && self.fd >= 0 {
            unsafe { libc::close(self.fd) };
        }
    }
}

#[cfg(unix)]
impl std::os::unix::io::AsRawFd for DmaBufFd {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.fd
    }
}
