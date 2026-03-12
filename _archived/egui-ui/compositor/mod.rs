//! Wayland compositor implementation using Smithay.
//!
//! This module provides the core compositor functionality including:
//! - DRM/KMS backend for display management
//! - wgpu-based rendering with Metal backend
//! - Surface management for video and UI
//! - Input handling via libinput

use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{debug, error, info, warn};

use yama_platform_traits::renderer::Renderer;

use crate::AppState;

mod render;
mod surface;
mod input;

pub use render::{create_renderer, MetalRenderer};
pub use surface::SurfaceManager;
pub use input::InputHandler;

/// Compositor configuration.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CompositorConfig {
    /// Backend type: "drm" or "winit"
    #[serde(default = "default_backend")]
    pub backend: String,
    /// Renderer type: "wgpu" or "vulkan"
    #[serde(default = "default_renderer")]
    pub renderer: String,
    /// Target frame rate
    #[serde(default = "default_fps")]
    pub fps: u32,
    /// Enable VSync
    #[serde(default = "default_true")]
    pub vsync: bool,
    /// Display width
    #[serde(default = "default_width")]
    pub width: u32,
    /// Display height
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_backend() -> String {
    "drm".to_string()
}

fn default_renderer() -> String {
    "wgpu".to_string()
}

fn default_fps() -> u32 {
    60
}

fn default_true() -> bool {
    true
}

fn default_width() -> u32 {
    1920
}

fn default_height() -> u32 {
    1080
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            renderer: default_renderer(),
            fps: default_fps(),
            vsync: default_true(),
            width: default_width(),
            height: default_height(),
        }
    }
}

/// The main compositor.
pub struct Compositor {
    config: CompositorConfig,
    state: Arc<AppState>,
    renderer: Box<dyn Renderer>,
    surfaces: SurfaceManager,
    input: InputHandler,
    running: bool,
}

impl Compositor {
    /// Create a new compositor.
    pub fn new(config: CompositorConfig, state: Arc<AppState>) -> Result<Self> {
        info!("Initializing compositor with {:?} backend", config.backend);

        // Initialize renderer using the platform trait
        let renderer = create_renderer(&config).context("Failed to initialize renderer")?;

        // Initialize surface manager
        let surfaces = SurfaceManager::new();

        // Initialize input handler
        let input = InputHandler::new();

        Ok(Self {
            config,
            state,
            renderer,
            surfaces,
            input,
            running: false,
        })
    }

    /// Run the compositor event loop.
    pub async fn run(mut self) -> Result<()> {
        info!("Starting compositor event loop");
        self.running = true;

        let frame_time = std::time::Duration::from_secs_f64(1.0 / f64::from(self.config.fps));

        while self.running {
            let frame_start = std::time::Instant::now();

            // Process input events
            self.input.process_events(&mut self.surfaces)?;

            // Update surfaces
            self.surfaces.update()?;

            // Begin frame
            self.renderer.begin_frame()?;

            // Clear background
            self.renderer.clear(yama_platform_traits::Color::BLACK)?;

            // Convert surfaces to render surfaces
            let render_surfaces = self.surfaces.to_render_surfaces();

            // Render frame using the trait
            self.renderer.render(&render_surfaces)?;

            // End frame
            self.renderer.end_frame()?;

            // Present
            self.renderer.present().await?;

            // Handle event bus messages
            self.process_events().await?;

            // Frame timing
            let elapsed = frame_start.elapsed();
            if elapsed < frame_time {
                tokio::time::sleep(frame_time - elapsed).await;
            } else {
                debug!("Frame took {:?} (target: {:?})", elapsed, frame_time);
            }
        }

        Ok(())
    }

    /// Process events from the event bus.
    async fn process_events(&mut self) -> Result<()> {
        // Poll for events without blocking
        // TODO: Integrate with event bus receiver
        Ok(())
    }

    /// Request compositor shutdown.
    pub fn shutdown(&mut self) {
        info!("Compositor shutdown requested");
        self.running = false;
    }

    /// Get memory usage from the renderer.
    pub fn memory_usage(&self) -> yama_platform_traits::renderer::MemoryUsage {
        self.renderer.memory_usage()
    }
}
