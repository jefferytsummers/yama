//! Configuration card component for the Yama design system.
//!
//! Config cards display configurable items (tools, models, workflows)
//! with an enable/disable toggle and optional description.

use eframe::egui::{self, Response, Rounding, Sense, Stroke, Ui, Vec2};

use crate::{colors, font_size, radius, spacing};
use super::toggle_switch::ToggleSwitch;

/// A configuration card with toggle, icon, and description.
///
/// # Example
///
/// ```ignore
/// let mut enabled = true;
/// ConfigCard::new("search_videos", &mut enabled)
///     .icon("🔍")
///     .description("Search through indexed video content")
///     .show(ui);
/// ```
pub struct ConfigCard<'a> {
    id: String,
    enabled: &'a mut bool,
    icon: Option<String>,
    description: Option<String>,
    secondary_text: Option<String>,
    interactive: bool,
}

impl<'a> ConfigCard<'a> {
    /// Create a new config card.
    pub fn new(id: impl Into<String>, enabled: &'a mut bool) -> Self {
        Self {
            id: id.into(),
            enabled,
            icon: None,
            description: None,
            secondary_text: None,
            interactive: true,
        }
    }

    /// Set an icon to display before the name.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set a description to display below the name.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set secondary text (e.g., version, status) to display on the right.
    pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
        self.secondary_text = Some(text.into());
        self
    }

    /// Set whether the card is interactive.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Display the config card.
    pub fn show(self, ui: &mut Ui) -> ConfigCardResponse {
        let mut changed = false;

        let min_height = if self.description.is_some() { 56.0 } else { 40.0 };
        let available_width = ui.available_width();

        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(available_width, min_height),
            if self.interactive { Sense::click() } else { Sense::hover() },
        );

        // Background color with hover state
        let bg_color = if response.hovered() && self.interactive {
            colors::GRAPHITE
        } else {
            colors::SLATE
        };

        // Draw background
        ui.painter().rect(
            rect,
            Rounding::same(radius::MD),
            bg_color,
            Stroke::new(1.0, colors::STONE),
        );

        // Layout content
        let content_rect = rect.shrink(spacing::S3);

        // Toggle on the right side
        let toggle_width = 36.0;
        let toggle_height = 20.0;
        let toggle_rect = egui::Rect::from_min_size(
            egui::pos2(
                content_rect.right() - toggle_width,
                content_rect.center().y - toggle_height / 2.0,
            ),
            Vec2::new(toggle_width, toggle_height),
        );

        // Content area (left of toggle)
        let text_right = toggle_rect.left() - spacing::S3;

        // Icon and name
        let mut text_left = content_rect.left();

        if let Some(icon) = &self.icon {
            let icon_size = 16.0;
            ui.painter().text(
                egui::pos2(text_left + icon_size / 2.0, content_rect.center().y - 2.0),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(icon_size),
                colors::SILVER,
            );
            text_left += icon_size + spacing::S2;
        }

        // Name and description
        let name_y = if self.description.is_some() {
            content_rect.top() + spacing::S2
        } else {
            content_rect.center().y - font_size::BODY / 2.0
        };

        ui.painter().text(
            egui::pos2(text_left, name_y),
            egui::Align2::LEFT_TOP,
            &self.id,
            egui::FontId::proportional(font_size::BODY),
            colors::CHALK,
        );

        if let Some(description) = &self.description {
            let desc_y = name_y + font_size::BODY + 2.0;
            let max_width = text_right - text_left;

            // Truncate description if too long
            let truncated = truncate_text(description, max_width, font_size::SMALL);

            ui.painter().text(
                egui::pos2(text_left, desc_y),
                egui::Align2::LEFT_TOP,
                &truncated,
                egui::FontId::proportional(font_size::SMALL),
                colors::SILVER,
            );
        }

        // Secondary text (version, etc.)
        if let Some(secondary) = &self.secondary_text {
            let secondary_x = toggle_rect.left() - spacing::S3;
            ui.painter().text(
                egui::pos2(secondary_x, content_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                secondary,
                egui::FontId::proportional(font_size::TINY),
                colors::ASH,
            );
        }

        // Draw toggle using a child UI
        let mut toggle_ui = ui.new_child(egui::UiBuilder::new()
            .max_rect(toggle_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)));
        let toggle_response = ToggleSwitch::new(self.enabled)
            .enabled(self.interactive)
            .show(&mut toggle_ui);

        if toggle_response.changed() {
            changed = true;
        }

        // Handle click on the entire card (excluding toggle area)
        if response.clicked() && self.interactive {
            let click_pos = response.interact_pointer_pos().unwrap_or_default();
            if !toggle_rect.contains(click_pos) {
                *self.enabled = !*self.enabled;
                changed = true;
            }
        }

        ConfigCardResponse {
            response,
            changed,
            enabled: *self.enabled,
        }
    }
}

/// Response from displaying a config card.
pub struct ConfigCardResponse {
    /// The egui response for the card area.
    pub response: Response,
    /// Whether the enabled state changed.
    pub changed: bool,
    /// The current enabled state.
    pub enabled: bool,
}

/// Truncate text to fit within a given width (rough estimate).
fn truncate_text(text: &str, max_width: f32, font_size: f32) -> String {
    // Rough estimate: average character width is ~0.5 * font_size
    let avg_char_width = font_size * 0.5;
    let max_chars = (max_width / avg_char_width) as usize;

    if text.len() <= max_chars {
        text.to_string()
    } else if max_chars > 3 {
        format!("{}...", &text[..max_chars - 3])
    } else {
        text[..max_chars.min(text.len())].to_string()
    }
}

/// Convenience function to draw a config card.
pub fn config_card(
    ui: &mut Ui,
    id: &str,
    enabled: &mut bool,
    icon: Option<&str>,
    description: Option<&str>,
) -> ConfigCardResponse {
    let mut card = ConfigCard::new(id, enabled);
    if let Some(icon) = icon {
        card = card.icon(icon);
    }
    if let Some(description) = description {
        card = card.description(description);
    }
    card.show(ui)
}
