//! Progress bar component for the Yama design system.
//!
//! Progress bars show completion state with the amber-to-ember gradient.
//! They match the web `.progress-bar` component styling.

use eframe::egui::{self, Color32, Rounding, Ui};

use crate::colors;

/// A styled progress bar.
///
/// # Example
///
/// ```ignore
/// ProgressBar::new(0.65)
///     .height(8.0)
///     .show_percentage()
///     .ui(ui);
/// ```
pub struct ProgressBar {
    progress: f32,
    height: f32,
    show_percentage: bool,
    use_gradient: bool,
}

impl ProgressBar {
    /// Create a new progress bar with the given progress (0.0 to 1.0).
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            height: 4.0,
            show_percentage: false,
            use_gradient: true,
        }
    }

    /// Set the height of the progress bar.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Show a percentage label.
    pub fn show_percentage(mut self) -> Self {
        self.show_percentage = true;
        self
    }

    /// Use a solid color instead of gradient.
    pub fn solid_color(mut self) -> Self {
        self.use_gradient = false;
        self
    }

    /// Display the progress bar.
    pub fn ui(self, ui: &mut Ui) {
        let available_width = ui.available_width();

        // Optional percentage label
        if self.show_percentage {
            ui.horizontal(|ui| {
                ui.add_space(available_width - 40.0);
                ui.label(
                    egui::RichText::new(format!("{:.0}%", self.progress * 100.0))
                        .color(colors::TEXT_MUTED)
                        .size(10.0),
                );
            });
        }

        // Allocate space for the bar
        let (rect, _response) =
            ui.allocate_exact_size(egui::vec2(available_width, self.height), egui::Sense::hover());

        // Draw track (background)
        ui.painter()
            .rect_filled(rect, Rounding::same(9999.0), colors::STONE);

        // Draw fill
        if self.progress > 0.0 {
            let fill_width = rect.width() * self.progress;
            let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_width, rect.height()));

            // Use gradient or solid color
            let fill_color = if self.use_gradient {
                // Simple gradient approximation: interpolate between amber and ember
                let r = colors::AMBER.r() as f32
                    + (colors::EMBER.r() as f32 - colors::AMBER.r() as f32) * self.progress;
                let g = colors::AMBER.g() as f32
                    + (colors::EMBER.g() as f32 - colors::AMBER.g() as f32) * self.progress;
                let b = colors::AMBER.b() as f32
                    + (colors::EMBER.b() as f32 - colors::AMBER.b() as f32) * self.progress;
                Color32::from_rgb(r as u8, g as u8, b as u8)
            } else {
                colors::AMBER
            };

            ui.painter()
                .rect_filled(fill_rect, Rounding::same(9999.0), fill_color);
        }
    }
}

/// Convenience function to create a simple progress bar.
pub fn progress_bar(ui: &mut Ui, progress: f32) {
    ProgressBar::new(progress).ui(ui);
}
