//! Animation utilities for the Yama design system.
//!
//! Provides consistent animation timing, pulse effects, and glow rendering
//! that match across web and native platforms.

use eframe::egui::{self, Color32, Painter, Rect};

use crate::colors;

/// Animation duration constants (in seconds).
pub mod duration {
    /// Fast animations (hover, micro-interactions).
    pub const FAST: f32 = 0.1;
    /// Normal animations (state changes, transitions).
    pub const NORMAL: f32 = 0.2;
    /// Slow animations (page transitions, loading states).
    pub const SLOW: f32 = 0.3;
    /// Pulse cycle duration (status indicators).
    pub const PULSE_CYCLE: f64 = 2.0;
}

/// Calculate a pulsing alpha value for status indicators.
///
/// Returns a value between 0.4 and 1.0 that oscillates smoothly over time.
/// Uses a 2-second cycle to match the web CSS animation.
///
/// # Example
///
/// ```ignore
/// let alpha = animation::pulse_alpha(ctx);
/// let pulsing_color = colors::with_alpha(colors::JADE, (alpha * 255.0) as u8);
/// ```
pub fn pulse_alpha(ctx: &egui::Context) -> f32 {
    // Request continuous repaints for animation
    ctx.request_repaint();

    // Get time since app start
    let time = ctx.input(|i| i.time);

    // 2-second cycle using sine wave, oscillating between 0.4 and 1.0
    let phase = (time / duration::PULSE_CYCLE) * std::f64::consts::TAU;
    let sine = phase.sin() as f32;

    // Map from [-1, 1] to [0.4, 1.0]
    0.7 + (sine * 0.3)
}

/// Draw a glow effect around a shape.
///
/// Creates a soft, diffused glow by drawing multiple circles with decreasing opacity.
/// The glow color is automatically derived from the provided color with reduced alpha.
///
/// # Arguments
///
/// * `painter` - The egui painter to draw with
/// * `center` - Center point of the glow
/// * `radius` - Radius of the innermost glow layer
/// * `color` - Base color for the glow
/// * `layers` - Number of glow layers (more = softer glow)
///
/// # Example
///
/// ```ignore
/// animation::draw_glow(&painter, rect.center(), 8.0, colors::JADE, 3);
/// ```
pub fn draw_glow(painter: &Painter, center: egui::Pos2, radius: f32, color: Color32, layers: u8) {
    let layers = layers.max(1);

    for i in (0..layers).rev() {
        let layer_radius = radius * (1.0 + (i as f32 * 0.5));
        let alpha = ((layers - i) as f32 / layers as f32) * 0.4;
        let glow_color = colors::with_alpha(color, (alpha * 255.0) as u8);
        painter.circle_filled(center, layer_radius, glow_color);
    }
}

/// Draw a glow effect within a rectangle.
///
/// Similar to `draw_glow` but expands outward from a rectangle's bounds.
/// Useful for glowing cards or panels.
pub fn draw_rect_glow(painter: &Painter, rect: Rect, color: Color32, spread: f32) {
    let expanded = rect.expand(spread);
    let glow_color = colors::with_alpha(color, 40);
    painter.rect_filled(expanded, spread, glow_color);
}

/// Easing functions for smooth animations.
pub mod easing {
    /// Ease-out cubic: decelerating to zero velocity.
    pub fn ease_out_cubic(t: f32) -> f32 {
        let t = t - 1.0;
        t * t * t + 1.0
    }

    /// Ease-in-out cubic: accelerating then decelerating.
    pub fn ease_in_out_cubic(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let t = -2.0 * t + 2.0;
            1.0 - t * t * t / 2.0
        }
    }

    /// Linear interpolation (no easing).
    pub fn linear(t: f32) -> f32 {
        t.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_bounds() {
        assert!((easing::ease_out_cubic(0.0) - 0.0).abs() < 0.001);
        assert!((easing::ease_out_cubic(1.0) - 1.0).abs() < 0.001);

        assert!((easing::ease_in_out_cubic(0.0) - 0.0).abs() < 0.001);
        assert!((easing::ease_in_out_cubic(1.0) - 1.0).abs() < 0.001);

        assert!((easing::linear(0.0) - 0.0).abs() < 0.001);
        assert!((easing::linear(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_linear_clamp() {
        assert_eq!(easing::linear(-0.5), 0.0);
        assert_eq!(easing::linear(1.5), 1.0);
    }
}
