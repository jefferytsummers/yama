//! Avatar component for the Yama design system.
//!
//! Renders user and assistant avatars with consistent styling.
//! User avatars show a person icon, assistant avatars show a sparkle/AI icon.

use eframe::egui::{self, Color32, Response, Ui, Vec2};

use crate::colors;

/// Avatar type determines the icon and color scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarType {
    /// User avatar - amber with person icon
    User,
    /// Assistant avatar - violet with AI/sparkle icon
    Assistant,
    /// System avatar - azure with info icon
    System,
}

impl AvatarType {
    /// Get the background color for this avatar type.
    pub fn background_color(self) -> Color32 {
        match self {
            AvatarType::User => colors::with_alpha(colors::AMBER, 51), // 20%
            AvatarType::Assistant => colors::with_alpha(colors::VIOLET, 51),
            AvatarType::System => colors::with_alpha(colors::AZURE, 51),
        }
    }

    /// Get the foreground (icon) color for this avatar type.
    pub fn foreground_color(self) -> Color32 {
        match self {
            AvatarType::User => colors::AMBER,
            AvatarType::Assistant => colors::VIOLET,
            AvatarType::System => colors::AZURE,
        }
    }

    /// Get the icon character for this avatar type.
    pub fn icon(self) -> &'static str {
        match self {
            AvatarType::User => "👤",
            AvatarType::Assistant => "✦",
            AvatarType::System => "ℹ",
        }
    }
}

/// An avatar component for chat messages.
///
/// # Example
///
/// ```ignore
/// Avatar::new(AvatarType::User)
///     .size(32.0)
///     .show(ui);
///
/// Avatar::new(AvatarType::Assistant)
///     .pulsing(true)
///     .show(ui);
/// ```
pub struct Avatar {
    avatar_type: AvatarType,
    size: f32,
    pulsing: bool,
}

impl Avatar {
    /// Create a new avatar of the given type.
    pub fn new(avatar_type: AvatarType) -> Self {
        Self {
            avatar_type,
            size: 28.0,
            pulsing: false,
        }
    }

    /// Set the avatar size in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Enable pulsing animation (for inferring state).
    pub fn pulsing(mut self, pulsing: bool) -> Self {
        self.pulsing = pulsing;
        self
    }

    /// Display the avatar.
    pub fn show(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(self.size), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center();
            let radius = self.size / 2.0;

            // Calculate alpha for pulsing effect
            let alpha = if self.pulsing {
                let pulse = crate::animation::pulse_alpha(ui.ctx());
                (pulse * 255.0) as u8
            } else {
                255
            };

            // Draw background circle
            let bg_color = if self.pulsing {
                colors::with_alpha(self.avatar_type.background_color(), alpha)
            } else {
                self.avatar_type.background_color()
            };
            painter.circle_filled(center, radius, bg_color);

            // Draw border
            let border_color = if self.pulsing {
                colors::with_alpha(self.avatar_type.foreground_color(), alpha / 2)
            } else {
                colors::with_alpha(self.avatar_type.foreground_color(), 102) // 40%
            };
            painter.circle_stroke(center, radius, egui::Stroke::new(1.0, border_color));

            // Draw icon
            let icon = self.avatar_type.icon();
            let fg_color = if self.pulsing {
                colors::with_alpha(self.avatar_type.foreground_color(), alpha)
            } else {
                self.avatar_type.foreground_color()
            };

            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(self.size * 0.5),
                fg_color,
            );
        }

        response
    }
}

/// Draw a simple avatar for a chat message.
pub fn avatar(ui: &mut Ui, avatar_type: AvatarType) -> Response {
    Avatar::new(avatar_type).show(ui)
}

/// Draw a pulsing avatar (for inferring state).
pub fn avatar_pulsing(ui: &mut Ui, avatar_type: AvatarType) -> Response {
    Avatar::new(avatar_type).pulsing(true).show(ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_colors() {
        assert_eq!(AvatarType::User.foreground_color(), colors::AMBER);
        assert_eq!(AvatarType::Assistant.foreground_color(), colors::VIOLET);
        assert_eq!(AvatarType::System.foreground_color(), colors::AZURE);
    }

    #[test]
    fn test_avatar_icons() {
        assert_eq!(AvatarType::User.icon(), "👤");
        assert_eq!(AvatarType::Assistant.icon(), "✦");
        assert_eq!(AvatarType::System.icon(), "ℹ");
    }
}
