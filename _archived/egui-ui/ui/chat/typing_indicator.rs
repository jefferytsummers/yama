//! Typing indicator component for assistant responses.

use eframe::egui::{self, Ui};
use yama_theme::{animation, colors, spacing};

/// A pulsing typing indicator to show the assistant is "thinking".
pub struct TypingIndicator {
    dot_count: usize,
}

impl Default for TypingIndicator {
    fn default() -> Self {
        Self { dot_count: 3 }
    }
}

impl TypingIndicator {
    /// Create a new typing indicator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of dots.
    pub fn dots(mut self, count: usize) -> Self {
        self.dot_count = count.clamp(1, 5);
        self
    }

    /// Render the typing indicator.
    pub fn show(self, ui: &mut Ui) {
        let time = ui.ctx().input(|i| i.time);
        let dot_size = 6.0;
        let dot_spacing = spacing::S2;
        let total_width = (dot_size + dot_spacing) * self.dot_count as f32;

        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(total_width, dot_size * 2.0),
            egui::Sense::hover(),
        );

        if !ui.is_rect_visible(rect) {
            return;
        }

        // Request repaint for animation
        ui.ctx().request_repaint();

        let painter = ui.painter();
        let base_y = rect.center().y;

        for i in 0..self.dot_count {
            let x = rect.left() + (dot_size + dot_spacing) * i as f32 + dot_size / 2.0;

            // Phase offset for each dot
            let phase = time * 3.0 + (i as f64 * 0.4);
            let bounce = (phase.sin() as f32 * 0.5 + 0.5) * dot_size;

            let center = egui::pos2(x, base_y - bounce);

            // Draw dot with varying alpha
            let alpha = animation::pulse_alpha(ui.ctx());
            let color = colors::with_alpha(colors::VIOLET, (alpha * 200.0) as u8);

            painter.circle_filled(center, dot_size / 2.0, color);
        }
    }
}

/// Draw a simple typing indicator.
pub fn typing_indicator(ui: &mut Ui) {
    TypingIndicator::new().show(ui);
}
