//! Surface management for the compositor.

use std::collections::HashMap;

use anyhow::Result;
use tracing::debug;

use yama_platform_traits::{Rect, Transform};
use yama_platform_traits::renderer::{RenderSurface, TextureHandle};

/// A surface in the compositor.
#[derive(Debug)]
pub struct Surface {
    /// Surface identifier.
    pub id: SurfaceId,
    /// Surface type.
    pub kind: SurfaceKind,
    /// Position in compositor space.
    pub position: (i32, i32),
    /// Size in pixels.
    pub size: (u32, u32),
    /// Whether the surface is visible.
    pub visible: bool,
    /// Z-order (higher = on top).
    pub z_order: i32,
    /// Opacity (0.0 - 1.0).
    pub opacity: f32,
    /// Associated texture handle.
    pub texture: Option<TextureHandle>,
    /// DMA-BUF file descriptor (for zero-copy video).
    pub dma_buf_fd: Option<i32>,
}

/// Surface identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub u64);

/// Type of surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    /// Video playback surface.
    Video,
    /// UI overlay surface.
    Ui,
    /// Detection overlay surface.
    Overlay,
    /// External Wayland client surface.
    Wayland,
}

/// Manages all surfaces in the compositor.
pub struct SurfaceManager {
    surfaces: HashMap<SurfaceId, Surface>,
    next_id: u64,
    /// Primary video surface (fullscreen).
    primary_video: Option<SurfaceId>,
    /// Thumbnail video surfaces.
    thumbnails: Vec<SurfaceId>,
}

impl SurfaceManager {
    /// Create a new surface manager.
    pub fn new() -> Self {
        Self {
            surfaces: HashMap::new(),
            next_id: 1,
            primary_video: None,
            thumbnails: Vec::new(),
        }
    }

    /// Create a new surface.
    pub fn create(&mut self, kind: SurfaceKind, size: (u32, u32)) -> SurfaceId {
        let id = SurfaceId(self.next_id);
        self.next_id += 1;

        let surface = Surface {
            id,
            kind,
            position: (0, 0),
            size,
            visible: true,
            z_order: match kind {
                SurfaceKind::Video => 0,
                SurfaceKind::Overlay => 1,
                SurfaceKind::Ui => 2,
                SurfaceKind::Wayland => 3,
            },
            opacity: 1.0,
            texture: None,
            dma_buf_fd: None,
        };

        debug!("Created surface {:?}", surface);
        self.surfaces.insert(id, surface);
        id
    }

    /// Get a surface by ID.
    pub fn get(&self, id: SurfaceId) -> Option<&Surface> {
        self.surfaces.get(&id)
    }

    /// Get a mutable surface by ID.
    pub fn get_mut(&mut self, id: SurfaceId) -> Option<&mut Surface> {
        self.surfaces.get_mut(&id)
    }

    /// Remove a surface.
    pub fn remove(&mut self, id: SurfaceId) -> Option<Surface> {
        debug!("Removing surface {:?}", id);
        self.surfaces.remove(&id)
    }

    /// Set the primary video surface.
    pub fn set_primary_video(&mut self, id: SurfaceId) {
        self.primary_video = Some(id);
    }

    /// Get the primary video surface.
    pub fn primary_video(&self) -> Option<&Surface> {
        self.primary_video.and_then(|id| self.surfaces.get(&id))
    }

    /// Add a thumbnail video surface.
    pub fn add_thumbnail(&mut self, id: SurfaceId) {
        self.thumbnails.push(id);
    }

    /// Get all thumbnail surfaces.
    pub fn thumbnails(&self) -> impl Iterator<Item = &Surface> {
        self.thumbnails
            .iter()
            .filter_map(|id| self.surfaces.get(id))
    }

    /// Get the number of surfaces.
    pub fn count(&self) -> usize {
        self.surfaces.len()
    }

    /// Update all surfaces (called each frame).
    pub fn update(&mut self) -> Result<()> {
        // Update surface state, animations, etc.
        Ok(())
    }

    /// Get surfaces sorted by z-order for rendering.
    pub fn sorted_for_render(&self) -> Vec<&Surface> {
        let mut surfaces: Vec<_> = self.surfaces.values().filter(|s| s.visible).collect();
        surfaces.sort_by_key(|s| s.z_order);
        surfaces
    }

    /// Convert surfaces to render surfaces for the platform renderer.
    pub fn to_render_surfaces(&self) -> Vec<RenderSurface> {
        self.surfaces
            .values()
            .filter(|s| s.visible && s.texture.is_some())
            .map(|s| RenderSurface {
                texture: s.texture.unwrap(),
                src_rect: None,
                dst_rect: Rect::new(s.position.0, s.position.1, s.size.0, s.size.1),
                transform: Transform::IDENTITY,
                opacity: s.opacity,
                z_order: s.z_order,
            })
            .collect()
    }

    /// Attach a texture to a surface.
    pub fn set_texture(&mut self, id: SurfaceId, texture: TextureHandle) {
        if let Some(surface) = self.surfaces.get_mut(&id) {
            surface.texture = Some(texture);
        }
    }

    /// Attach a DMA-BUF to a surface (for zero-copy video on Jetson).
    pub fn set_dma_buf(&mut self, id: SurfaceId, fd: i32) {
        if let Some(surface) = self.surfaces.get_mut(&id) {
            surface.dma_buf_fd = Some(fd);
        }
    }
}

impl Default for SurfaceManager {
    fn default() -> Self {
        Self::new()
    }
}
