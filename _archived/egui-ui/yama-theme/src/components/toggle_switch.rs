//! Toggle switch component for the Yama design system.
//!
//! Toggle switches provide a binary on/off control with smooth animation.
//! Uses JADE for enabled state and STONE for disabled state.

use eframe::egui::{self, Response, Sense, Ui, Vec2};

use crate::{animation, colors};

/// A styled toggle switch control.
///
/// # Example
///
/// ```ignore
/// let mut enabled = true;
/// if ToggleSwitch::new(&mut enabled).show(ui).changed() {
///     println!("Toggle changed to: {}", enabled);
/// }
/// ```
pub struct ToggleSwitch<'a> {
    value: &'a mut bool,
    size: ToggleSwitchSize,
    enabled: bool,
}

/// Size variants for the toggle switch.
#[derive(Debug, Clone, Copy, Default)]
pub enum ToggleSwitchSize {
    /// Small toggle (16x28px)
    Small,
    /// Medium toggle (20x36px) - default
    #[default]
    Medium,
    /// Large toggle (24x44px)
    Large,
}

impl ToggleSwitchSize {
    /// Get the height of the toggle track.
    pub fn height(self) -> f32 {
        match self {
            ToggleSwitchSize::Small => 16.0,
            ToggleSwitchSize::Medium => 20.0,
            ToggleSwitchSize::Large => 24.0,
        }
    }

    /// Get the width of the toggle track.
    pub fn width(self) -> f32 {
        match self {
            ToggleSwitchSize::Small => 28.0,
            ToggleSwitchSize::Medium => 36.0,
            ToggleSwitchSize::Large => 44.0,
        }
    }

    /// Get the radius of the toggle knob.
    pub fn knob_radius(self) -> f32 {
        (self.height() - 4.0) / 2.0
    }
}

impl<'a> ToggleSwitch<'a> {
    /// Create a new toggle switch bound to a boolean value.
    pub fn new(value: &'a mut bool) -> Self {
        Self {
            value,
            size: ToggleSwitchSize::default(),
            enabled: true,
        }
    }

    /// Set the size of the toggle switch.
    pub fn size(mut self, size: ToggleSwitchSize) -> Self {
        self.size = size;
        self
    }

    /// Set whether the toggle is enabled (interactive).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Display the toggle switch and handle interaction.
    pub fn show(self, ui: &mut Ui) -> Response {
        let width = self.size.width();
        let height = self.size.height();
        let knob_radius = self.size.knob_radius();

        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(width, height),
            if self.enabled { Sense::click() } else { Sense::hover() },
        );

        // Handle click
        if response.clicked() && self.enabled {
            *self.value = !*self.value;
        }

        // Animation progress (0.0 = off, 1.0 = on)
        let target = if *self.value { 1.0_f32 } else { 0.0_f32 };
        let anim_progress = ui.ctx().animate_value_with_time(
            response.id,
            target,
            animation::duration::NORMAL,
        );

        // Calculate colors
        let track_color = if self.enabled {
            if *self.value {
                colors::JADE
            } else {
                colors::STONE
            }
        } else {
            colors::with_alpha(colors::STONE, 128)
        };

        let knob_color = if self.enabled {
            colors::CHALK
        } else {
            colors::with_alpha(colors::CHALK, 180)
        };

        // Hover highlight
        let track_color = if response.hovered() && self.enabled {
            colors::with_alpha(track_color, 230)
        } else {
            track_color
        };

        // Draw track
        let track_rounding = height / 2.0;
        ui.painter().rect_filled(rect, track_rounding, track_color);

        // Draw knob
        let knob_padding = 2.0;
        let knob_travel = width - (knob_radius * 2.0) - (knob_padding * 2.0);
        let knob_x = rect.left() + knob_padding + knob_radius + (anim_progress * knob_travel);
        let knob_center = egui::pos2(knob_x, rect.center().y);

        // Knob shadow
        ui.painter().circle_filled(
            knob_center + Vec2::new(0.0, 1.0),
            knob_radius,
            egui::Color32::from_black_alpha(40),
        );

        // Knob
        ui.painter().circle_filled(knob_center, knob_radius, knob_color);

        response
    }
}

/// Convenience function to draw a simple toggle switch.
pub fn toggle_switch(ui: &mut Ui, value: &mut bool) -> Response {
    ToggleSwitch::new(value).show(ui)
}

/// Convenience function to draw a toggle switch with a label.
pub fn toggle_switch_with_label(ui: &mut Ui, value: &mut bool, label: &str) -> Response {
    ui.horizontal(|ui| {
        let response = ToggleSwitch::new(value).show(ui);
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(label)
                .color(colors::TEXT_SECONDARY)
                .size(14.0),
        );
        response
    })
    .inner
}
