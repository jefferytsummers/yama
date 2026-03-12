//! Status indicator component for the Yama design system.
//!
//! Status indicators show service/process states with consistent colors and animations.
//! They match the web `.status-dot` component styling.

use eframe::egui::{self, Color32, Ui, Vec2};

use crate::{animation, colors};

/// Status states for services and processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Service is running normally.
    Running,
    /// Service is stopped/inactive.
    Stopped,
    /// Service is starting up.
    Starting,
    /// Service is shutting down.
    Stopping,
    /// Service has encountered an error.
    Failed,
    /// Service is performing inference.
    Inferring,
}

impl Status {
    /// Get the color for this status.
    pub fn color(self) -> Color32 {
        match self {
            Status::Running => colors::SUCCESS,
            Status::Stopped => colors::TEXT_MUTED,
            Status::Starting | Status::Stopping => colors::WARNING,
            Status::Failed => colors::ERROR,
            Status::Inferring => colors::INFERENCE,
        }
    }

    /// Whether this status should have a pulsing animation.
    pub fn should_pulse(self) -> bool {
        matches!(self, Status::Starting | Status::Stopping | Status::Inferring)
    }

    /// Whether this status should have a glow effect.
    pub fn should_glow(self) -> bool {
        matches!(self, Status::Running | Status::Inferring | Status::Failed)
    }

    /// Get a human-readable label for this status.
    pub fn label(self) -> &'static str {
        match self {
            Status::Running => "Running",
            Status::Stopped => "Stopped",
            Status::Starting => "Starting",
            Status::Stopping => "Stopping",
            Status::Failed => "Failed",
            Status::Inferring => "Inferring",
        }
    }
}

/// A status indicator dot with optional glow and pulse effects.
pub struct StatusIndicator {
    status: Status,
    size: f32,
    show_label: bool,
}

impl StatusIndicator {
    /// Create a new status indicator.
    pub fn new(status: Status) -> Self {
        Self {
            status,
            size: 8.0,
            show_label: false,
        }
    }

    /// Set the size of the indicator dot.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Show a text label next to the dot.
    pub fn with_label(mut self) -> Self {
        self.show_label = true;
        self
    }

    /// Display the status indicator.
    pub fn show(self, ui: &mut Ui) {
        let color = self.status.color();

        // Calculate alpha for pulse effect
        let alpha = if self.status.should_pulse() {
            animation::pulse_alpha(ui.ctx())
        } else {
            1.0
        };

        let display_color = colors::with_alpha(color, (alpha * 255.0) as u8);

        ui.horizontal(|ui| {
            let (rect, _response) =
                ui.allocate_exact_size(Vec2::splat(self.size), egui::Sense::hover());

            // Draw glow effect for certain statuses
            if self.status.should_glow() {
                animation::draw_glow(ui.painter(), rect.center(), self.size * 0.6, color, 3);
            }

            // Draw the dot
            ui.painter()
                .circle_filled(rect.center(), self.size * 0.4, display_color);

            // Optional label
            if self.show_label {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(self.status.label())
                        .color(colors::TEXT_SECONDARY)
                        .size(12.0),
                );
            }
        });
    }
}

/// Convenience function to draw a simple status indicator.
pub fn status_indicator(ui: &mut Ui, status: Status, size: f32) {
    StatusIndicator::new(status).size(size).show(ui);
}
