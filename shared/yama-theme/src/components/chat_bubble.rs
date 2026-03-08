//! Chat bubble component for the Yama design system.
//!
//! Provides consistent chat message styling for user, assistant, and system messages.
//! User messages are right-aligned with amber accent, assistant messages are left-aligned
//! with slate background, and system messages are centered with subtle styling.

use eframe::egui::{self, Color32, Margin, Response, Rounding, Stroke, Ui};

use crate::{animation, colors, radius, spacing};

/// Message role determines the visual styling of the chat bubble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    /// User messages - right-aligned with amber accent
    User,
    /// Assistant messages - left-aligned with slate background
    Assistant,
    /// System messages - centered with subtle styling
    System,
}

/// Chat bubble styling configuration.
#[derive(Debug, Clone)]
pub struct ChatBubbleStyle {
    /// Background color
    pub fill: Color32,
    /// Border color
    pub stroke: Color32,
    /// Border radius (asymmetric for speech bubble tail effect)
    pub rounding: Rounding,
    /// Whether to show a glow effect (for inferring state)
    pub glow: bool,
    /// Glow color (used when glow is true)
    pub glow_color: Color32,
}

impl ChatBubbleStyle {
    /// Get the default style for a message role.
    pub fn for_role(role: MessageRole) -> Self {
        match role {
            MessageRole::User => Self {
                fill: colors::with_alpha(colors::AMBER, 51), // 20% alpha
                stroke: colors::with_alpha(colors::AMBER, 153), // 60% alpha
                // Tail on right: nw: LG, ne: SM, sw: LG, se: LG
                rounding: Rounding {
                    nw: radius::LG,
                    ne: radius::SM,
                    sw: radius::LG,
                    se: radius::LG,
                },
                glow: false,
                glow_color: colors::AMBER,
            },
            MessageRole::Assistant => Self {
                fill: colors::SLATE,
                stroke: colors::STONE,
                // Tail on left: nw: SM, ne: LG, sw: LG, se: LG
                rounding: Rounding {
                    nw: radius::SM,
                    ne: radius::LG,
                    sw: radius::LG,
                    se: radius::LG,
                },
                glow: false,
                glow_color: colors::VIOLET,
            },
            MessageRole::System => Self {
                fill: colors::with_alpha(colors::GRAPHITE, 128), // 50% alpha
                stroke: Color32::TRANSPARENT,
                rounding: Rounding::same(radius::MD),
                glow: false,
                glow_color: colors::AZURE,
            },
        }
    }

    /// Enable the glowing state (for assistant messages during inference).
    pub fn with_glow(mut self, enabled: bool) -> Self {
        self.glow = enabled;
        self
    }

    /// Set a custom glow color.
    pub fn with_glow_color(mut self, color: Color32) -> Self {
        self.glow_color = color;
        self
    }
}

/// A chat bubble component for displaying messages.
///
/// # Example
///
/// ```ignore
/// ChatBubble::new(MessageRole::User)
///     .show(ui, |ui| {
///         ui.label("Hello, can you analyze this video?");
///     });
///
/// ChatBubble::new(MessageRole::Assistant)
///     .inferring(true)
///     .show(ui, |ui| {
///         ui.label("Analyzing frames...");
///     });
/// ```
pub struct ChatBubble {
    role: MessageRole,
    style: ChatBubbleStyle,
    max_width_fraction: f32,
    padding: f32,
}

impl ChatBubble {
    /// Create a new chat bubble for the given role.
    pub fn new(role: MessageRole) -> Self {
        Self {
            role,
            style: ChatBubbleStyle::for_role(role),
            max_width_fraction: match role {
                MessageRole::User | MessageRole::Assistant => 0.75,
                MessageRole::System => 0.9,
            },
            padding: spacing::S3,
        }
    }

    /// Enable the inferring state (violet glow for assistant).
    pub fn inferring(mut self, is_inferring: bool) -> Self {
        if is_inferring && self.role == MessageRole::Assistant {
            self.style = self.style.with_glow(true).with_glow_color(colors::VIOLET);
        }
        self
    }

    /// Set custom padding.
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the maximum width as a fraction of available width.
    pub fn max_width(mut self, fraction: f32) -> Self {
        self.max_width_fraction = fraction.clamp(0.3, 1.0);
        self
    }

    /// Display the chat bubble with its content.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> (Response, R) {
        let available_width = ui.available_width();
        let max_bubble_width = available_width * self.max_width_fraction;

        // Layout based on role
        let layout = match self.role {
            MessageRole::User => egui::Layout::right_to_left(egui::Align::TOP),
            MessageRole::Assistant => egui::Layout::left_to_right(egui::Align::TOP),
            MessageRole::System => egui::Layout::centered_and_justified(egui::Direction::TopDown),
        };

        ui.with_layout(layout, |ui| {
            ui.set_max_width(max_bubble_width);

            // Draw glow effect if enabled
            let glow_rect = if self.style.glow {
                Some(ui.available_rect_before_wrap())
            } else {
                None
            };

            let frame = egui::Frame::none()
                .fill(self.style.fill)
                .stroke(Stroke::new(1.0, self.style.stroke))
                .rounding(self.style.rounding)
                .inner_margin(Margin::same(self.padding));

            let response = frame.show(ui, |ui| {
                // Draw glow behind content
                if let Some(rect) = glow_rect {
                    let pulse = animation::pulse_alpha(ui.ctx());
                    let glow_alpha = (pulse * 60.0) as u8;
                    animation::draw_rect_glow(
                        ui.painter(),
                        rect,
                        colors::with_alpha(self.style.glow_color, glow_alpha),
                        4.0,
                    );
                }

                content(ui)
            });

            (response.response, response.inner)
        })
        .inner
    }
}

/// Draw a chat bubble with simple text content.
pub fn chat_bubble(ui: &mut Ui, role: MessageRole, text: &str) -> Response {
    let text_color = match role {
        MessageRole::User => colors::CHALK,
        MessageRole::Assistant => colors::CHALK,
        MessageRole::System => colors::SILVER,
    };

    let font_size = match role {
        MessageRole::System => crate::font_size::SMALL,
        _ => crate::font_size::BODY,
    };

    ChatBubble::new(role)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(text_color).size(font_size));
        })
        .0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_styles() {
        let user_style = ChatBubbleStyle::for_role(MessageRole::User);
        assert!(!user_style.glow);

        let assistant_style = ChatBubbleStyle::for_role(MessageRole::Assistant);
        assert_eq!(assistant_style.fill, colors::SLATE);

        let system_style = ChatBubbleStyle::for_role(MessageRole::System);
        assert_eq!(system_style.stroke, Color32::TRANSPARENT);
    }

    #[test]
    fn test_glow_modifier() {
        let style = ChatBubbleStyle::for_role(MessageRole::Assistant)
            .with_glow(true)
            .with_glow_color(colors::VIOLET);
        assert!(style.glow);
        assert_eq!(style.glow_color, colors::VIOLET);
    }
}
