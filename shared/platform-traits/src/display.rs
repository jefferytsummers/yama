//! Display backend trait for output management.
//!
//! This trait abstracts over display systems:
//! - DRM/KMS (Linux native)
//! - CoreGraphics (macOS)
//! - Wayland (as a client)
//! - Winit (development/windowed mode)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::allocator::GpuBuffer;
use crate::error::PlatformResult;
use crate::types::{Rect, Size};

/// Output identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutputId(pub u32);

impl OutputId {
    /// Invalid output ID.
    pub const INVALID: Self = Self(u32::MAX);

    /// Primary output (usually first connected).
    pub const PRIMARY: Self = Self(0);

    /// Check if valid.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

/// Display mode (resolution and refresh rate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayMode {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Refresh rate in millihertz (e.g., 60000 = 60Hz).
    pub refresh_mhz: u32,
    /// Whether this is the preferred/native mode.
    pub preferred: bool,
}

impl DisplayMode {
    /// Create a new display mode.
    #[must_use]
    pub const fn new(width: u32, height: u32, refresh_mhz: u32) -> Self {
        Self {
            width,
            height,
            refresh_mhz,
            preferred: false,
        }
    }

    /// Get refresh rate in Hz.
    #[must_use]
    pub fn refresh_hz(&self) -> f32 {
        self.refresh_mhz as f32 / 1000.0
    }

    /// Get size.
    #[must_use]
    pub const fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Common modes.
    pub const MODE_1080P_60: Self = Self::new(1920, 1080, 60000);
    pub const MODE_4K_60: Self = Self::new(3840, 2160, 60000);
    pub const MODE_720P_60: Self = Self::new(1280, 720, 60000);
}

/// Output/connector type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ConnectorType {
    /// Unknown connector type.
    Unknown,
    /// VGA.
    Vga,
    /// DVI.
    Dvi,
    /// HDMI.
    Hdmi,
    /// DisplayPort.
    DisplayPort,
    /// Embedded display (laptop, tablet).
    Embedded,
    /// Virtual/headless display.
    Virtual,
    /// USB-C with DisplayPort.
    UsbC,
}

/// Information about a display output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputInfo {
    /// Output identifier.
    pub id: OutputId,
    /// Human-readable name.
    pub name: String,
    /// Connector type.
    pub connector: ConnectorType,
    /// Whether the output is currently connected.
    pub connected: bool,
    /// Whether the output is enabled/active.
    pub enabled: bool,
    /// Current mode (if enabled).
    pub current_mode: Option<DisplayMode>,
    /// Available modes.
    pub available_modes: Vec<DisplayMode>,
    /// Physical size in millimeters (0 if unknown).
    pub physical_width_mm: u32,
    /// Physical height in millimeters (0 if unknown).
    pub physical_height_mm: u32,
    /// Current position in multi-monitor setup.
    pub position: (i32, i32),
    /// Manufacturer name (if available).
    pub manufacturer: Option<String>,
    /// Model name (if available).
    pub model: Option<String>,
    /// Serial number (if available).
    pub serial: Option<String>,
}

impl OutputInfo {
    /// Get DPI based on physical size and current mode.
    #[must_use]
    pub fn dpi(&self) -> Option<f32> {
        let mode = self.current_mode.as_ref()?;
        if self.physical_width_mm == 0 {
            return None;
        }
        let width_inches = self.physical_width_mm as f32 / 25.4;
        Some(mode.width as f32 / width_inches)
    }

    /// Get aspect ratio.
    #[must_use]
    pub fn aspect_ratio(&self) -> Option<f32> {
        let mode = self.current_mode.as_ref()?;
        Some(mode.width as f32 / mode.height as f32)
    }
}

/// VSync mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VsyncMode {
    /// VSync disabled (may tear).
    Off,
    /// VSync enabled (wait for vblank).
    #[default]
    On,
    /// Adaptive VSync (on if below refresh rate).
    Adaptive,
    /// Mailbox (triple buffering, no tearing, low latency).
    Mailbox,
}

/// Display backend capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayCapabilities {
    /// Backend name.
    pub name: String,
    /// Whether multi-monitor is supported.
    pub supports_multi_monitor: bool,
    /// Whether hotplug detection is supported.
    pub supports_hotplug: bool,
    /// Whether mode setting is supported.
    pub supports_mode_setting: bool,
    /// Supported VSync modes.
    pub vsync_modes: Vec<VsyncMode>,
    /// Maximum number of outputs.
    pub max_outputs: u32,
}

/// Display backend event.
#[derive(Debug, Clone)]
pub enum DisplayEvent {
    /// Output connected.
    OutputConnected(OutputId),
    /// Output disconnected.
    OutputDisconnected(OutputId),
    /// Output mode changed.
    ModeChanged(OutputId, DisplayMode),
    /// VBlank occurred.
    VBlank(OutputId),
    /// Page flip completed.
    PageFlipComplete(OutputId),
}

/// Display backend abstraction.
///
/// This trait defines the interface for platform-specific display backends.
/// Implementations handle output enumeration, mode setting, and page flipping.
#[async_trait]
pub trait DisplayBackend: Send + Sync {
    /// Get backend capabilities.
    fn capabilities(&self) -> &DisplayCapabilities;

    /// Get all outputs.
    fn outputs(&self) -> Vec<OutputInfo>;

    /// Get a specific output.
    fn output(&self, id: OutputId) -> Option<OutputInfo>;

    /// Get the primary output.
    fn primary_output(&self) -> Option<OutputInfo> {
        self.outputs().into_iter().find(|o| o.connected && o.enabled)
    }

    /// Enable an output with a specific mode.
    fn enable_output(&mut self, id: OutputId, mode: &DisplayMode) -> PlatformResult<()>;

    /// Disable an output.
    fn disable_output(&mut self, id: OutputId) -> PlatformResult<()>;

    /// Set the mode for an output.
    fn set_mode(&mut self, id: OutputId, mode: &DisplayMode) -> PlatformResult<()>;

    /// Set output position (for multi-monitor).
    fn set_position(&mut self, id: OutputId, x: i32, y: i32) -> PlatformResult<()>;

    /// Set VSync mode.
    fn set_vsync(&mut self, mode: VsyncMode) -> PlatformResult<()>;

    /// Get current VSync mode.
    fn vsync(&self) -> VsyncMode;

    /// Queue a buffer for display (page flip).
    ///
    /// The buffer will be displayed at the next vblank.
    fn queue_buffer(&mut self, output: OutputId, buffer: &GpuBuffer) -> PlatformResult<()>;

    /// Wait for page flip to complete.
    async fn wait_for_flip(&mut self, output: OutputId) -> PlatformResult<()>;

    /// Poll for display events.
    fn poll_events(&mut self) -> Vec<DisplayEvent>;

    /// Wait for the next event.
    async fn wait_event(&mut self) -> PlatformResult<DisplayEvent>;

    /// Get total display area (bounding box of all outputs).
    fn total_area(&self) -> Rect {
        let outputs = self.outputs();
        if outputs.is_empty() {
            return Rect::default();
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for output in outputs {
            if let Some(mode) = &output.current_mode {
                min_x = min_x.min(output.position.0);
                min_y = min_y.min(output.position.1);
                max_x = max_x.max(output.position.0 + mode.width as i32);
                max_y = max_y.max(output.position.1 + mode.height as i32);
            }
        }

        Rect::new(
            min_x,
            min_y,
            (max_x - min_x) as u32,
            (max_y - min_y) as u32,
        )
    }
}
