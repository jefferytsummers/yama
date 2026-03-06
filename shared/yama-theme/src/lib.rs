//! Yama Design System Theme
//!
//! Unified theming for egui applications following the "Obsidian Lens" brand identity.
//! Provides consistent colors, typography, and styling across all Yama native UIs.

use eframe::egui::{self, Color32, FontFamily, FontId, Margin, Rounding, Stroke, Vec2};

/// Core palette colors - Obsidian to Stone spectrum
pub mod colors {
    use super::Color32;

    // Core backgrounds (dark to light)
    pub const OBSIDIAN: Color32 = Color32::from_rgb(13, 13, 15);
    pub const BASALT: Color32 = Color32::from_rgb(22, 22, 26);
    pub const SLATE: Color32 = Color32::from_rgb(30, 30, 36);
    pub const GRAPHITE: Color32 = Color32::from_rgb(42, 42, 50);
    pub const STONE: Color32 = Color32::from_rgb(61, 61, 71);

    // Accent colors
    pub const AMBER: Color32 = Color32::from_rgb(245, 158, 11);
    pub const AMBER_LIGHT: Color32 = Color32::from_rgb(251, 191, 36);
    pub const EMBER: Color32 = Color32::from_rgb(239, 68, 68);
    pub const EMBER_LIGHT: Color32 = Color32::from_rgb(248, 113, 113);
    pub const JADE: Color32 = Color32::from_rgb(16, 185, 129);
    pub const JADE_LIGHT: Color32 = Color32::from_rgb(52, 211, 153);
    pub const AZURE: Color32 = Color32::from_rgb(59, 130, 246);
    pub const AZURE_LIGHT: Color32 = Color32::from_rgb(96, 165, 250);
    pub const VIOLET: Color32 = Color32::from_rgb(139, 92, 246);
    pub const VIOLET_LIGHT: Color32 = Color32::from_rgb(167, 139, 250);

    // Text colors
    pub const CHALK: Color32 = Color32::from_rgb(250, 250, 250);
    pub const SILVER: Color32 = Color32::from_rgb(161, 161, 170);
    pub const ASH: Color32 = Color32::from_rgb(113, 113, 122);

    // Semantic aliases
    pub const BACKGROUND: Color32 = OBSIDIAN;
    pub const BACKGROUND_SECONDARY: Color32 = BASALT;
    pub const SURFACE: Color32 = SLATE;
    pub const SURFACE_HOVER: Color32 = GRAPHITE;
    pub const BORDER: Color32 = STONE;

    pub const PRIMARY: Color32 = AMBER;
    pub const SUCCESS: Color32 = JADE;
    pub const ERROR: Color32 = EMBER;
    pub const WARNING: Color32 = AMBER;
    pub const INFO: Color32 = AZURE;
    pub const INFERENCE: Color32 = VIOLET;

    pub const TEXT_PRIMARY: Color32 = CHALK;
    pub const TEXT_SECONDARY: Color32 = SILVER;
    pub const TEXT_MUTED: Color32 = ASH;

    /// Create a color with modified alpha
    pub const fn with_alpha(color: Color32, alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
    }
}

/// Border radius values
pub mod radius {
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
    pub const XL: f32 = 12.0;
}

/// Spacing values in pixels
pub mod spacing {
    pub const S1: f32 = 4.0;
    pub const S2: f32 = 8.0;
    pub const S3: f32 = 12.0;
    pub const S4: f32 = 16.0;
    pub const S5: f32 = 20.0;
    pub const S6: f32 = 24.0;
    pub const S8: f32 = 32.0;
    pub const S10: f32 = 40.0;
}

/// Font sizes
pub mod font_size {
    pub const DISPLAY: f32 = 32.0;
    pub const H1: f32 = 24.0;
    pub const H2: f32 = 20.0;
    pub const H3: f32 = 16.0;
    pub const BODY: f32 = 14.0;
    pub const SMALL: f32 = 12.0;
    pub const TINY: f32 = 10.0;
}

/// Yama theme configuration
#[derive(Debug, Clone)]
pub struct YamaTheme {
    /// Enable animations (glow effects, etc.)
    pub animations_enabled: bool,
    /// Scale factor for high-DPI displays
    pub scale_factor: f32,
}

impl Default for YamaTheme {
    fn default() -> Self {
        Self {
            animations_enabled: true,
            scale_factor: 1.0,
        }
    }
}

impl YamaTheme {
    /// Create a new Yama theme with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the scale factor for high-DPI displays
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale_factor = scale;
        self
    }

    /// Enable or disable animations
    pub fn with_animations(mut self, enabled: bool) -> Self {
        self.animations_enabled = enabled;
        self
    }

    /// Apply the Yama theme to an egui context
    pub fn apply(&self, ctx: &egui::Context) {
        // Configure fonts
        self.configure_fonts(ctx);

        // Configure visuals (colors, spacing, etc.)
        self.configure_visuals(ctx);

        // Configure style
        self.configure_style(ctx);
    }

    fn configure_fonts(&self, _ctx: &egui::Context) {
        // Use default system fonts.
        // In production, you could load custom fonts here:
        //
        // let mut fonts = egui::FontDefinitions::default();
        // fonts.font_data.insert(
        //     "geist".to_owned(),
        //     egui::FontData::from_static(include_bytes!("../fonts/Geist-Regular.otf")),
        // );
        // ctx.set_fonts(fonts);
        //
        // For now, we rely on the system fonts which work well on macOS.
    }

    fn configure_visuals(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();

        // Background colors
        visuals.panel_fill = colors::BACKGROUND;
        visuals.window_fill = colors::SURFACE;
        visuals.extreme_bg_color = colors::OBSIDIAN;
        visuals.faint_bg_color = colors::BASALT;
        visuals.code_bg_color = colors::SURFACE;

        // Widget colors
        visuals.widgets.noninteractive.bg_fill = colors::SURFACE;
        visuals.widgets.noninteractive.weak_bg_fill = colors::BASALT;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, colors::BORDER);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors::TEXT_SECONDARY);
        visuals.widgets.noninteractive.rounding = Rounding::same(radius::MD);

        visuals.widgets.inactive.bg_fill = colors::SURFACE;
        visuals.widgets.inactive.weak_bg_fill = colors::GRAPHITE;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors::BORDER);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors::TEXT_SECONDARY);
        visuals.widgets.inactive.rounding = Rounding::same(radius::MD);

        visuals.widgets.hovered.bg_fill = colors::GRAPHITE;
        visuals.widgets.hovered.weak_bg_fill = colors::STONE;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, colors::TEXT_MUTED);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);
        visuals.widgets.hovered.rounding = Rounding::same(radius::MD);

        visuals.widgets.active.bg_fill = colors::AMBER;
        visuals.widgets.active.weak_bg_fill = colors::with_alpha(colors::AMBER, 50);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, colors::AMBER_LIGHT);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, colors::OBSIDIAN);
        visuals.widgets.active.rounding = Rounding::same(radius::MD);

        visuals.widgets.open.bg_fill = colors::GRAPHITE;
        visuals.widgets.open.weak_bg_fill = colors::STONE;
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, colors::AMBER);
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);
        visuals.widgets.open.rounding = Rounding::same(radius::MD);

        // Selection colors
        visuals.selection.bg_fill = colors::with_alpha(colors::AMBER, 80);
        visuals.selection.stroke = Stroke::new(1.0, colors::AMBER);

        // Hyperlinks
        visuals.hyperlink_color = colors::AZURE;

        // Error/warning colors
        visuals.error_fg_color = colors::ERROR;
        visuals.warn_fg_color = colors::WARNING;

        // Window appearance
        visuals.window_rounding = Rounding::same(radius::LG);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: Vec2::new(0.0, 4.0),
            blur: 12.0,
            spread: 0.0,
            color: Color32::from_black_alpha(100),
        };
        visuals.window_stroke = Stroke::new(1.0, colors::BORDER);

        // Popup appearance
        visuals.popup_shadow = egui::epaint::Shadow {
            offset: Vec2::new(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: Color32::from_black_alpha(80),
        };

        // Resize handle
        visuals.resize_corner_size = 12.0;

        // Clip rect margin
        visuals.clip_rect_margin = 3.0;

        // Button frames
        visuals.button_frame = true;

        // Collapsing header frame
        visuals.collapsing_header_frame = true;

        // Striped tables
        visuals.striped = true;

        ctx.set_visuals(visuals);
    }

    fn configure_style(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();

        // Spacing
        style.spacing.item_spacing = Vec2::new(spacing::S2, spacing::S2);
        style.spacing.window_margin = Margin::same(spacing::S4);
        style.spacing.button_padding = Vec2::new(spacing::S3, spacing::S2);
        style.spacing.menu_margin = Margin::same(spacing::S2);
        style.spacing.indent = spacing::S4;
        style.spacing.interact_size = Vec2::new(spacing::S10, spacing::S5);
        style.spacing.slider_width = 100.0;
        style.spacing.combo_width = 100.0;
        style.spacing.text_edit_width = 200.0;
        style.spacing.icon_width = spacing::S4;
        style.spacing.icon_width_inner = spacing::S3;
        style.spacing.icon_spacing = spacing::S2;
        style.spacing.tooltip_width = 400.0;
        style.spacing.menu_width = 200.0;
        style.spacing.combo_height = 200.0;
        style.spacing.scroll = egui::style::ScrollStyle::solid();

        // Animation
        style.animation_time = if self.animations_enabled { 0.15 } else { 0.0 };

        // Interaction
        style.interaction.selectable_labels = true;
        style.interaction.multi_widget_text_select = true;
        style.interaction.tooltip_delay = 0.5;

        // Visuals already configured separately

        ctx.set_style(style);
    }
}

/// Helper function to create a styled frame for panels
pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::BACKGROUND_SECONDARY)
        .inner_margin(spacing::S4)
        .outer_margin(0.0)
}

/// Helper function to create a styled frame for cards
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::SURFACE)
        .stroke(Stroke::new(1.0, colors::BORDER))
        .rounding(Rounding::same(radius::LG))
        .inner_margin(spacing::S4)
        .shadow(egui::epaint::Shadow {
            offset: Vec2::new(0.0, 1.0),
            blur: 3.0,
            spread: 0.0,
            color: Color32::from_black_alpha(60),
        })
}

/// Helper function to create a styled frame for the top bar
pub fn topbar_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::BACKGROUND_SECONDARY)
        .stroke(Stroke::new(1.0, colors::BORDER))
        .inner_margin(Margin::symmetric(spacing::S4, spacing::S2))
}

/// Helper to create primary button styling
pub fn primary_button() -> egui::Button<'static> {
    egui::Button::new("")
        .fill(colors::PRIMARY)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(radius::MD))
}

/// Helper to create secondary button styling
pub fn secondary_button() -> egui::Button<'static> {
    egui::Button::new("")
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, colors::BORDER))
        .rounding(Rounding::same(radius::MD))
}

/// Helper to create a styled progress bar
pub fn progress_bar(progress: f32) -> egui::ProgressBar {
    egui::ProgressBar::new(progress)
        .fill(colors::AMBER)
}

/// Status indicator colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Running,
    Stopped,
    Starting,
    Stopping,
    Failed,
    Inferring,
}

impl Status {
    /// Get the color for this status
    pub fn color(self) -> Color32 {
        match self {
            Status::Running => colors::SUCCESS,
            Status::Stopped => colors::TEXT_MUTED,
            Status::Starting | Status::Stopping => colors::WARNING,
            Status::Failed => colors::ERROR,
            Status::Inferring => colors::INFERENCE,
        }
    }

    /// Whether this status should have a pulsing animation
    pub fn should_pulse(self) -> bool {
        matches!(self, Status::Starting | Status::Stopping | Status::Inferring)
    }

    /// Whether this status should have a glow effect
    pub fn should_glow(self) -> bool {
        matches!(self, Status::Running | Status::Inferring)
    }
}

/// Draw a status indicator dot
pub fn status_dot(ui: &mut egui::Ui, status: Status, size: f32) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let color = status.color();

    // Draw glow effect for certain statuses
    if status.should_glow() {
        ui.painter().circle_filled(
            rect.center(),
            size * 0.8,
            colors::with_alpha(color, 40),
        );
    }

    // Draw the dot
    ui.painter().circle_filled(rect.center(), size * 0.4, color);
}

/// Rich text helper for consistent typography
pub trait RichTextExt {
    fn heading1(self) -> Self;
    fn heading2(self) -> Self;
    fn heading3(self) -> Self;
    fn body(self) -> Self;
    fn small(self) -> Self;
    fn muted(self) -> Self;
    fn primary_color(self) -> Self;
    fn success_color(self) -> Self;
    fn error_color(self) -> Self;
    fn mono(self) -> Self;
}

impl RichTextExt for egui::RichText {
    fn heading1(self) -> Self {
        self.size(font_size::H1).strong()
    }

    fn heading2(self) -> Self {
        self.size(font_size::H2).strong()
    }

    fn heading3(self) -> Self {
        self.size(font_size::H3).strong()
    }

    fn body(self) -> Self {
        self.size(font_size::BODY)
    }

    fn small(self) -> Self {
        self.size(font_size::SMALL)
    }

    fn muted(self) -> Self {
        self.color(colors::TEXT_MUTED)
    }

    fn primary_color(self) -> Self {
        self.color(colors::PRIMARY)
    }

    fn success_color(self) -> Self {
        self.color(colors::SUCCESS)
    }

    fn error_color(self) -> Self {
        self.color(colors::ERROR)
    }

    fn mono(self) -> Self {
        self.family(FontFamily::Monospace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_with_alpha() {
        let color = colors::with_alpha(colors::AMBER, 128);
        assert_eq!(color.r(), 245);
        assert_eq!(color.g(), 158);
        assert_eq!(color.b(), 11);
        assert_eq!(color.a(), 128);
    }

    #[test]
    fn test_status_colors() {
        assert_eq!(Status::Running.color(), colors::SUCCESS);
        assert_eq!(Status::Failed.color(), colors::ERROR);
        assert!(Status::Starting.should_pulse());
        assert!(!Status::Stopped.should_pulse());
    }
}
