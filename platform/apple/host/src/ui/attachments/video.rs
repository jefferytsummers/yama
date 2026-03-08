//! Video attachment component with thumbnail and metadata.

use eframe::egui::{self, Ui};
use yama_theme::{colors, font_size, radius, spacing};

use crate::ui::VideoAttachmentData;

/// Renders a video attachment with thumbnail and metadata.
pub struct VideoAttachment<'a> {
    data: &'a VideoAttachmentData,
    compact: bool,
}

impl<'a> VideoAttachment<'a> {
    /// Create a new video attachment renderer.
    pub fn new(data: &'a VideoAttachmentData) -> Self {
        Self {
            data,
            compact: false,
        }
    }

    /// Use compact display mode (for chat bubbles).
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Render the video attachment.
    pub fn show(self, ui: &mut Ui) {
        if self.compact {
            self.show_compact(ui);
        } else {
            self.show_full(ui);
        }
    }

    /// Compact display for chat bubbles.
    fn show_compact(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Thumbnail or placeholder
            let thumb_size = egui::vec2(48.0, 36.0);

            if let Some(texture) = &self.data.thumbnail {
                ui.image((texture.id(), thumb_size));
            } else {
                // Placeholder
                let (rect, _) = ui.allocate_exact_size(thumb_size, egui::Sense::hover());
                ui.painter().rect_filled(rect, radius::SM, colors::GRAPHITE);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "🎬",
                    egui::FontId::proportional(font_size::H3),
                    colors::STONE,
                );
            }

            ui.add_space(spacing::S2);

            // Metadata
            ui.vertical(|ui| {
                // Filename
                ui.label(
                    egui::RichText::new(&self.data.filename)
                        .color(colors::CHALK)
                        .size(font_size::SMALL),
                );

                // Size and duration
                let mut meta_parts = vec![format_file_size(self.data.size)];

                if let Some(duration) = self.data.duration_ms {
                    meta_parts.push(format_duration(duration));
                }

                if let Some((w, h)) = self.data.dimensions {
                    meta_parts.push(format!("{}×{}", w, h));
                }

                ui.label(
                    egui::RichText::new(meta_parts.join(" • "))
                        .color(colors::ASH)
                        .size(font_size::TINY),
                );
            });
        });
    }

    /// Full display for expanded view.
    fn show_full(&self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(colors::SLATE)
            .stroke(egui::Stroke::new(1.0, colors::STONE))
            .rounding(egui::Rounding::same(radius::LG))
            .inner_margin(spacing::S3)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Thumbnail or placeholder
                    let thumb_size = egui::vec2(200.0, 120.0);

                    if let Some(texture) = &self.data.thumbnail {
                        ui.image((texture.id(), thumb_size));
                    } else {
                        let (rect, _) = ui.allocate_exact_size(thumb_size, egui::Sense::hover());
                        ui.painter().rect_filled(rect, radius::MD, colors::OBSIDIAN);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "🎬",
                            egui::FontId::proportional(48.0),
                            colors::STONE,
                        );
                    }

                    ui.add_space(spacing::S2);

                    // Filename
                    ui.label(
                        egui::RichText::new(&self.data.filename)
                            .color(colors::CHALK)
                            .size(font_size::BODY)
                            .strong(),
                    );

                    // Metadata
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format_file_size(self.data.size))
                                .color(colors::SILVER)
                                .size(font_size::SMALL),
                        );

                        if let Some(duration) = self.data.duration_ms {
                            ui.label(
                                egui::RichText::new("•")
                                    .color(colors::ASH)
                                    .size(font_size::SMALL),
                            );
                            ui.label(
                                egui::RichText::new(format_duration(duration))
                                    .color(colors::SILVER)
                                    .size(font_size::SMALL),
                            );
                        }

                        if let Some((w, h)) = self.data.dimensions {
                            ui.label(
                                egui::RichText::new("•")
                                    .color(colors::ASH)
                                    .size(font_size::SMALL),
                            );
                            ui.label(
                                egui::RichText::new(format!("{}×{}", w, h))
                                    .color(colors::SILVER)
                                    .size(font_size::SMALL),
                            );
                        }
                    });
                });
            });
    }
}

/// Format file size for display.
fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration in MM:SS format.
fn format_duration(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{}:{:02}", minutes, seconds)
}
