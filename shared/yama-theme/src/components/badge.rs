//! Badge component for the Yama design system.
//!
//! Badges are small labels used to display status, counts, or categories.
//! They match the web `.badge` component styling.

use eframe::egui::{self, Color32, Rounding, Ui};

use crate::{colors, radius, spacing};

/// Badge color variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    /// Default/neutral badge (silver text on graphite).
    #[default]
    Default,
    /// Primary badge (amber).
    Primary,
    /// Success badge (jade).
    Success,
    /// Error badge (ember).
    Error,
    /// Warning badge (amber).
    Warning,
    /// Info badge (azure).
    Info,
    /// Inference badge (violet).
    Inference,
}

impl BadgeVariant {
    /// Get the background color for this variant.
    pub fn bg_color(self) -> Color32 {
        match self {
            BadgeVariant::Default => colors::GRAPHITE,
            BadgeVariant::Primary => colors::with_alpha(colors::AMBER, 30),
            BadgeVariant::Success => colors::with_alpha(colors::JADE, 30),
            BadgeVariant::Error => colors::with_alpha(colors::EMBER, 30),
            BadgeVariant::Warning => colors::with_alpha(colors::AMBER, 30),
            BadgeVariant::Info => colors::with_alpha(colors::AZURE, 30),
            BadgeVariant::Inference => colors::with_alpha(colors::VIOLET, 30),
        }
    }

    /// Get the text color for this variant.
    pub fn text_color(self) -> Color32 {
        match self {
            BadgeVariant::Default => colors::TEXT_SECONDARY,
            BadgeVariant::Primary => colors::AMBER,
            BadgeVariant::Success => colors::JADE,
            BadgeVariant::Error => colors::EMBER,
            BadgeVariant::Warning => colors::AMBER,
            BadgeVariant::Info => colors::AZURE,
            BadgeVariant::Inference => colors::VIOLET,
        }
    }
}

/// A styled badge label.
///
/// # Example
///
/// ```ignore
/// Badge::new("Active")
///     .variant(BadgeVariant::Success)
///     .show(ui);
/// ```
pub struct Badge {
    text: String,
    variant: BadgeVariant,
}

impl Badge {
    /// Create a new badge with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: BadgeVariant::Default,
        }
    }

    /// Set the badge variant.
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Display the badge.
    pub fn show(self, ui: &mut Ui) {
        let bg_color = self.variant.bg_color();
        let text_color = self.variant.text_color();

        egui::Frame::none()
            .fill(bg_color)
            .rounding(Rounding::same(radius::SM))
            .inner_margin(egui::Margin::symmetric(spacing::S2, spacing::S1))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(&self.text)
                        .color(text_color)
                        .size(10.0)
                        .strong(),
                );
            });
    }
}

/// Convenience function to create a simple badge.
pub fn badge(ui: &mut Ui, text: &str, variant: BadgeVariant) {
    Badge::new(text).variant(variant).show(ui);
}
