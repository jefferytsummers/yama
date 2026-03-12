//! Card component for the Yama design system.
//!
//! Cards are containers that group related content with consistent styling.
//! They match the web `.card` component with Slate background and Stone border.

use eframe::egui::{self, Response, Rounding, Stroke, Ui, Vec2};

use crate::{colors, radius, spacing};

/// A styled card container.
///
/// # Example
///
/// ```ignore
/// Card::new()
///     .with_title("Service Status")
///     .hoverable()
///     .show(ui, |ui| {
///         ui.label("Running");
///     });
/// ```
#[derive(Default)]
pub struct Card {
    title: Option<String>,
    hoverable: bool,
    padding: f32,
}

impl Card {
    /// Create a new card with default settings.
    pub fn new() -> Self {
        Self {
            title: None,
            hoverable: false,
            padding: spacing::S4,
        }
    }

    /// Add a title to the card header.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Enable hover effects (background color change).
    pub fn hoverable(mut self) -> Self {
        self.hoverable = true;
        self
    }

    /// Set custom padding.
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Display the card and its content.
    ///
    /// Returns the response from the card frame (for hover/click detection).
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> (Response, R) {
        let fill = if self.hoverable {
            colors::SURFACE
        } else {
            colors::SURFACE
        };

        let frame = egui::Frame::none()
            .fill(fill)
            .stroke(Stroke::new(1.0, colors::BORDER))
            .rounding(Rounding::same(radius::LG))
            .inner_margin(self.padding)
            .shadow(egui::epaint::Shadow {
                offset: Vec2::new(0.0, 1.0),
                blur: 3.0,
                spread: 0.0,
                color: egui::Color32::from_black_alpha(60),
            });

        let response = frame.show(ui, |ui| {
            // Handle hover state
            let rect = ui.min_rect();
            if self.hoverable && ui.rect_contains_pointer(rect) {
                ui.painter().rect_filled(
                    rect,
                    Rounding::same(radius::LG),
                    colors::SURFACE_HOVER,
                );
            }

            // Title if present
            if let Some(title) = &self.title {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(title)
                            .strong()
                            .color(colors::TEXT_PRIMARY),
                    );
                });
                ui.add_space(spacing::S2);
            }

            content(ui)
        });

        (response.response, response.inner)
    }
}
