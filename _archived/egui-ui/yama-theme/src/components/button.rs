//! Button helpers for the Yama design system.
//!
//! Provides pre-configured button styles that match the web button components.

use eframe::egui::{self, Button, Color32, Rounding, Stroke};

use crate::{colors, radius};

/// Create a primary button (amber background, dark text).
///
/// # Example
///
/// ```ignore
/// if ui.add(primary_button("Run Inference")).clicked() {
///     // Handle click
/// }
/// ```
pub fn primary_button(text: impl Into<String>) -> Button<'static> {
    let text: String = text.into();
    Button::new(egui::RichText::new(text).color(colors::OBSIDIAN).strong())
        .fill(colors::PRIMARY)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(radius::MD))
}

/// Create a secondary button (transparent with border).
///
/// # Example
///
/// ```ignore
/// if ui.add(secondary_button("Cancel")).clicked() {
///     // Handle click
/// }
/// ```
pub fn secondary_button(text: impl Into<String>) -> Button<'static> {
    let text: String = text.into();
    Button::new(egui::RichText::new(text).color(colors::TEXT_SECONDARY))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, colors::BORDER))
        .rounding(Rounding::same(radius::MD))
}

/// Create a danger button (ember/red styling).
///
/// # Example
///
/// ```ignore
/// if ui.add(danger_button("Delete")).clicked() {
///     // Handle delete
/// }
/// ```
pub fn danger_button(text: impl Into<String>) -> Button<'static> {
    let text: String = text.into();
    Button::new(egui::RichText::new(text).color(colors::CHALK).strong())
        .fill(colors::ERROR)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(radius::MD))
}
