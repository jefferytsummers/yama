//! Video grid component for multi-stream display.
//!
//! Displays multiple video sources in a configurable grid layout with
//! selection and focus capabilities.

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use crate::{colors, radius, spacing};

/// Grid layout configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridLayout {
    /// Single stream fullscreen
    Single,
    /// 2x2 grid (4 streams)
    Grid2x2,
    /// 3x3 grid (9 streams)
    Grid3x3,
    /// 4x4 grid (16 streams)
    Grid4x4,
    /// Auto-select based on stream count
    #[default]
    Auto,
}

impl GridLayout {
    /// Get the number of columns for this layout.
    pub fn columns(self, stream_count: usize) -> usize {
        match self {
            GridLayout::Single => 1,
            GridLayout::Grid2x2 => 2,
            GridLayout::Grid3x3 => 3,
            GridLayout::Grid4x4 => 4,
            GridLayout::Auto => {
                if stream_count <= 1 {
                    1
                } else if stream_count <= 4 {
                    2
                } else if stream_count <= 9 {
                    3
                } else {
                    4
                }
            }
        }
    }
}

/// Connection status of a video source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamStatus {
    /// Stream is connected and receiving frames
    Connected,
    /// Stream is attempting to connect
    Connecting,
    /// Stream is disconnected
    #[default]
    Disconnected,
    /// Stream encountered an error
    Error,
}

impl StreamStatus {
    /// Get the status color.
    pub fn color(self) -> Color32 {
        match self {
            StreamStatus::Connected => colors::SUCCESS,
            StreamStatus::Connecting => colors::WARNING,
            StreamStatus::Disconnected => colors::TEXT_MUTED,
            StreamStatus::Error => colors::ERROR,
        }
    }

    /// Get the status label.
    pub fn label(self) -> &'static str {
        match self {
            StreamStatus::Connected => "CONNECTED",
            StreamStatus::Connecting => "CONNECTING",
            StreamStatus::Disconnected => "DISCONNECTED",
            StreamStatus::Error => "ERROR",
        }
    }

    /// Whether this status should pulse.
    pub fn should_pulse(self) -> bool {
        matches!(self, StreamStatus::Connecting)
    }
}

/// Information about a video source.
#[derive(Debug, Clone)]
pub struct VideoSourceInfo {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Source URL
    pub url: String,
    /// Current connection status
    pub status: StreamStatus,
    /// Optional texture handle for the video frame
    pub texture: Option<egui::TextureId>,
}

impl VideoSourceInfo {
    /// Create a new video source info.
    pub fn new(id: impl Into<String>, name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            url: url.into(),
            status: StreamStatus::Disconnected,
            texture: None,
        }
    }

    /// Set the connection status.
    pub fn with_status(mut self, status: StreamStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the texture handle.
    pub fn with_texture(mut self, texture: egui::TextureId) -> Self {
        self.texture = Some(texture);
        self
    }
}

/// Action returned from video grid interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoGridAction {
    /// No action
    None,
    /// A cell was selected
    Select(String),
    /// A cell was double-clicked to focus
    Focus(String),
    /// Exit focus mode
    Unfocus,
}

/// Video grid component for displaying multiple streams.
pub struct VideoGrid<'a> {
    sources: &'a [VideoSourceInfo],
    layout: GridLayout,
    selected_id: Option<&'a str>,
    focused_id: Option<&'a str>,
    show_overlays: bool,
}

impl<'a> VideoGrid<'a> {
    /// Create a new video grid.
    pub fn new(sources: &'a [VideoSourceInfo]) -> Self {
        Self {
            sources,
            layout: GridLayout::Auto,
            selected_id: None,
            focused_id: None,
            show_overlays: true,
        }
    }

    /// Set the grid layout.
    pub fn layout(mut self, layout: GridLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Set the selected source ID.
    pub fn selected(mut self, id: Option<&'a str>) -> Self {
        self.selected_id = id;
        self
    }

    /// Set the focused source ID (fullscreen mode).
    pub fn focused(mut self, id: Option<&'a str>) -> Self {
        self.focused_id = id;
        self
    }

    /// Set whether to show overlays (name, status).
    pub fn show_overlays(mut self, show: bool) -> Self {
        self.show_overlays = show;
        self
    }

    /// Show the video grid and return any action.
    pub fn show(self, ui: &mut Ui) -> VideoGridAction {
        let mut action = VideoGridAction::None;

        // Determine which sources to display
        let display_sources: Vec<&VideoSourceInfo> = if let Some(focused_id) = self.focused_id {
            self.sources
                .iter()
                .filter(|s| s.id == focused_id)
                .collect()
        } else {
            self.sources.iter().collect()
        };

        if display_sources.is_empty() {
            // Empty state
            self.show_empty_state(ui);
            return action;
        }

        // Calculate grid dimensions
        let columns = if self.focused_id.is_some() {
            1
        } else {
            self.layout.columns(display_sources.len())
        };
        let rows = (display_sources.len() + columns - 1) / columns;

        // Calculate cell size
        let available = ui.available_size();
        let gap = spacing::S2;
        let cell_width = (available.x - gap * (columns as f32 - 1.0)) / columns as f32;
        let cell_height = (available.y - gap * (rows as f32 - 1.0)) / rows as f32;

        // Maintain 16:9 aspect ratio if not focused
        let cell_height = if self.focused_id.is_some() {
            cell_height
        } else {
            cell_height.min(cell_width * 9.0 / 16.0)
        };

        let start_pos = ui.cursor().min;

        for (i, source) in display_sources.iter().enumerate() {
            let col = i % columns;
            let row = i / columns;

            let x = start_pos.x + (cell_width + gap) * col as f32;
            let y = start_pos.y + (cell_height + gap) * row as f32;
            let cell_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(cell_width, cell_height));

            // Draw cell
            let cell_action = self.draw_cell(ui, source, cell_rect);
            if cell_action != VideoGridAction::None {
                action = cell_action;
            }
        }

        // Allocate the full grid space
        let total_height = (cell_height + gap) * rows as f32 - gap;
        ui.allocate_space(Vec2::new(available.x, total_height));

        action
    }

    fn draw_cell(&self, ui: &mut Ui, source: &VideoSourceInfo, rect: Rect) -> VideoGridAction {
        let mut action = VideoGridAction::None;

        let is_selected = self.selected_id == Some(source.id.as_str());
        let is_focused = self.focused_id == Some(source.id.as_str());

        // Interaction
        let response = ui.allocate_rect(rect, Sense::click());

        if response.clicked() {
            if is_focused {
                action = VideoGridAction::Unfocus;
            } else if is_selected {
                action = VideoGridAction::Focus(source.id.clone());
            } else {
                action = VideoGridAction::Select(source.id.clone());
            }
        }

        if response.double_clicked() {
            action = VideoGridAction::Focus(source.id.clone());
        }

        // Border color based on state
        let border_color = if is_focused {
            colors::AMBER
        } else if is_selected {
            colors::AMBER
        } else if response.hovered() {
            colors::TEXT_MUTED
        } else {
            colors::BORDER
        };

        let border_width = if is_focused || is_selected { 2.0 } else { 1.0 };

        // Draw background
        ui.painter()
            .rect_filled(rect, Rounding::same(radius::LG), colors::BASALT);

        // Draw video frame or placeholder
        if let Some(texture_id) = source.texture {
            ui.painter().image(
                texture_id,
                rect.shrink(border_width),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            // Placeholder icon
            self.draw_placeholder(ui, rect);
        }

        // Draw overlay
        if self.show_overlays {
            self.draw_overlay(ui, source, rect, is_focused);
        }

        // Draw border
        ui.painter()
            .rect_stroke(rect, Rounding::same(radius::LG), Stroke::new(border_width, border_color));

        // Draw glow for focused
        if is_focused {
            let glow_rect = rect.expand(4.0);
            ui.painter().rect_stroke(
                glow_rect,
                Rounding::same(radius::LG + 4.0),
                Stroke::new(2.0, colors::with_alpha(colors::AMBER, 60)),
            );
        }

        action
    }

    fn draw_placeholder(&self, ui: &mut Ui, rect: Rect) {
        let center = rect.center();
        let icon_size = 32.0;

        // Camera icon placeholder
        let icon_rect = Rect::from_center_size(center, Vec2::splat(icon_size));
        ui.painter().rect_stroke(
            icon_rect,
            Rounding::same(radius::SM),
            Stroke::new(1.5, colors::with_alpha(colors::TEXT_MUTED, 100)),
        );

        // Lens circle
        ui.painter().circle_stroke(
            center,
            icon_size * 0.25,
            Stroke::new(1.5, colors::with_alpha(colors::TEXT_MUTED, 100)),
        );
    }

    fn draw_overlay(&self, ui: &mut Ui, source: &VideoSourceInfo, rect: Rect, is_focused: bool) {
        // Gradient overlay at bottom
        let overlay_height = 40.0;
        let overlay_rect = Rect::from_min_max(
            Pos2::new(rect.min.x, rect.max.y - overlay_height),
            rect.max,
        );

        // Semi-transparent gradient
        ui.painter().rect_filled(
            overlay_rect,
            Rounding {
                nw: 0.0,
                ne: 0.0,
                sw: radius::LG,
                se: radius::LG,
            },
            Color32::from_black_alpha(180),
        );

        // Source name
        let text_pos = Pos2::new(rect.min.x + spacing::S2, rect.max.y - overlay_height + spacing::S2);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_TOP,
            &source.name,
            egui::FontId::proportional(12.0),
            colors::CHALK,
        );

        // Status indicator
        let status_pos = Pos2::new(rect.max.x - spacing::S2, rect.max.y - overlay_height + spacing::S2);
        let status_color = source.status.color();

        // Status dot
        let dot_center = Pos2::new(status_pos.x - 30.0, status_pos.y + 6.0);
        if source.status.should_pulse() {
            // Pulsing glow
            ui.painter()
                .circle_filled(dot_center, 5.0, colors::with_alpha(status_color, 60));
        }
        ui.painter().circle_filled(dot_center, 3.0, status_color);

        // Status label
        ui.painter().text(
            Pos2::new(status_pos.x - 36.0, status_pos.y),
            egui::Align2::RIGHT_TOP,
            source.status.label(),
            egui::FontId::proportional(9.0),
            status_color,
        );

        // Exit focus button
        if is_focused {
            let btn_rect = Rect::from_min_size(
                Pos2::new(rect.max.x - 32.0 - spacing::S2, rect.min.y + spacing::S2),
                Vec2::new(32.0, 32.0),
            );

            ui.painter().rect_filled(
                btn_rect,
                Rounding::same(radius::MD),
                Color32::from_black_alpha(150),
            );

            // Minimize icon (four corners pointing inward)
            let icon_center = btn_rect.center();
            let icon_offset = 6.0;
            let icon_color = colors::CHALK;

            // Top-left corner
            ui.painter().line_segment(
                [
                    icon_center + Vec2::new(-icon_offset, -icon_offset),
                    icon_center + Vec2::new(-icon_offset + 4.0, -icon_offset),
                ],
                Stroke::new(1.5, icon_color),
            );
            ui.painter().line_segment(
                [
                    icon_center + Vec2::new(-icon_offset, -icon_offset),
                    icon_center + Vec2::new(-icon_offset, -icon_offset + 4.0),
                ],
                Stroke::new(1.5, icon_color),
            );

            // Top-right corner
            ui.painter().line_segment(
                [
                    icon_center + Vec2::new(icon_offset, -icon_offset),
                    icon_center + Vec2::new(icon_offset - 4.0, -icon_offset),
                ],
                Stroke::new(1.5, icon_color),
            );
            ui.painter().line_segment(
                [
                    icon_center + Vec2::new(icon_offset, -icon_offset),
                    icon_center + Vec2::new(icon_offset, -icon_offset + 4.0),
                ],
                Stroke::new(1.5, icon_color),
            );

            // Bottom-left corner
            ui.painter().line_segment(
                [
                    icon_center + Vec2::new(-icon_offset, icon_offset),
                    icon_center + Vec2::new(-icon_offset + 4.0, icon_offset),
                ],
                Stroke::new(1.5, icon_color),
            );
            ui.painter().line_segment(
                [
                    icon_center + Vec2::new(-icon_offset, icon_offset),
                    icon_center + Vec2::new(-icon_offset, icon_offset - 4.0),
                ],
                Stroke::new(1.5, icon_color),
            );

            // Bottom-right corner
            ui.painter().line_segment(
                [
                    icon_center + Vec2::new(icon_offset, icon_offset),
                    icon_center + Vec2::new(icon_offset - 4.0, icon_offset),
                ],
                Stroke::new(1.5, icon_color),
            );
            ui.painter().line_segment(
                [
                    icon_center + Vec2::new(icon_offset, icon_offset),
                    icon_center + Vec2::new(icon_offset, icon_offset - 4.0),
                ],
                Stroke::new(1.5, icon_color),
            );
        }
    }

    fn show_empty_state(&self, ui: &mut Ui) {
        let available = ui.available_size();
        let center = ui.cursor().min + available * 0.5;

        // Empty state box
        let box_size = Vec2::new(300.0, 150.0);
        let box_rect = Rect::from_center_size(center, box_size);

        ui.painter().rect_stroke(
            box_rect,
            Rounding::same(radius::LG),
            Stroke::new(1.0, colors::BORDER),
        );

        // Icon
        let icon_center = center - Vec2::new(0.0, 20.0);
        ui.painter()
            .circle_stroke(icon_center, 24.0, Stroke::new(1.5, colors::TEXT_MUTED));
        ui.painter().rect_stroke(
            Rect::from_center_size(icon_center, Vec2::new(36.0, 24.0)),
            Rounding::same(radius::SM),
            Stroke::new(1.5, colors::TEXT_MUTED),
        );

        // Text
        ui.painter().text(
            center + Vec2::new(0.0, 30.0),
            egui::Align2::CENTER_CENTER,
            "No video sources configured",
            egui::FontId::proportional(14.0),
            colors::TEXT_MUTED,
        );

        ui.allocate_space(available);
    }
}

/// Convenience function to show a video grid.
pub fn video_grid<'a>(
    ui: &mut Ui,
    sources: &'a [VideoSourceInfo],
    selected: Option<&'a str>,
    focused: Option<&'a str>,
) -> VideoGridAction {
    VideoGrid::new(sources)
        .selected(selected)
        .focused(focused)
        .show(ui)
}
