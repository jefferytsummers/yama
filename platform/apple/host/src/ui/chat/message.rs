//! Chat message rendering component.

use eframe::egui::{self, Response, Ui};
use yama_theme::{colors, components::*, font_size, radius, spacing};

use crate::ui::{ChatMessageData, FrameResult, InferenceProgress, MessageRole};

/// Renders a single chat message with appropriate styling.
pub struct ChatMessage<'a> {
    data: &'a ChatMessageData,
}

impl<'a> ChatMessage<'a> {
    /// Create a new chat message renderer.
    pub fn new(data: &'a ChatMessageData) -> Self {
        Self { data }
    }

    /// Format a timestamp in compact MM:SS format.
    fn format_timestamp_compact(ms: u64) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    /// Render the message.
    pub fn show(self, ui: &mut Ui) -> Response {
        match self.data.role {
            MessageRole::User => self.show_user_message(ui),
            MessageRole::Assistant => self.show_assistant_message(ui),
            MessageRole::System => self.show_system_message(ui),
        }
    }

    /// Render a user message (right-aligned with attachments).
    fn show_user_message(self, ui: &mut Ui) -> Response {
        let theme_role = yama_theme::components::MessageRole::User;

        ChatBubble::new(theme_role)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Show attachments first
                    if !self.data.attachments.is_empty() {
                        for attachment in &self.data.attachments {
                            ui.horizontal(|ui| {
                                // Video icon and filename
                                ui.label(
                                    egui::RichText::new("🎬")
                                        .size(font_size::BODY),
                                );
                                ui.label(
                                    egui::RichText::new(&attachment.filename)
                                        .color(colors::AMBER_LIGHT)
                                        .size(font_size::SMALL),
                                );

                                // Size
                                let size_str = format_file_size(attachment.size);
                                ui.label(
                                    egui::RichText::new(format!("({})", size_str))
                                        .color(colors::ASH)
                                        .size(font_size::TINY),
                                );
                            });
                        }

                        if !self.data.content.is_empty() {
                            ui.add_space(spacing::S2);
                        }
                    }

                    // Show message content
                    if !self.data.content.is_empty() {
                        ui.label(
                            egui::RichText::new(&self.data.content)
                                .color(colors::CHALK)
                                .size(font_size::BODY),
                        );
                    }
                });
            })
            .0
    }

    /// Render an assistant message (left-aligned with frame results).
    fn show_assistant_message(self, ui: &mut Ui) -> Response {
        let theme_role = yama_theme::components::MessageRole::Assistant;
        let is_inferring = self.data.is_inferring();

        ChatBubble::new(theme_role)
            .inferring(is_inferring)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Show error if present
                    if let Some(error) = &self.data.error {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("⚠")
                                    .color(colors::EMBER)
                                    .size(font_size::BODY),
                            );
                            ui.label(
                                egui::RichText::new(error)
                                    .color(colors::EMBER)
                                    .size(font_size::BODY),
                            );
                        });
                        return;
                    }

                    // Show progress with live frame result if inferring
                    if let Some(progress) = &self.data.progress {
                        self.show_live_analysis(ui, progress, &self.data.frame_results);
                    } else if !self.data.frame_results.is_empty() {
                        // Inference complete - show full results with summary
                        self.show_completed_results(ui, &self.data.frame_results);
                    } else if !is_inferring && self.data.content.is_empty() {
                        // No results yet but not inferring - shouldn't happen normally
                        ui.label(
                            egui::RichText::new("No results")
                                .color(colors::ASH)
                                .italics()
                                .size(font_size::BODY),
                        );
                    }
                });
            })
            .0
    }

    /// Show live analysis progress with single-line current frame status.
    fn show_live_analysis(&self, ui: &mut Ui, progress: &InferenceProgress, results: &[FrameResult]) {
        // Progress header with spinner
        ui.horizontal(|ui| {
            ui.spinner();
            ui.add_space(spacing::S2);

            if progress.total_frames > 0 {
                ui.label(
                    egui::RichText::new(format!(
                        "Analyzing frame {}/{} ({:.0}%)",
                        progress.current_frame, progress.total_frames, progress.percent
                    ))
                    .color(colors::VIOLET)
                    .size(font_size::BODY),
                );
            } else {
                ui.label(
                    egui::RichText::new("Preparing analysis...")
                        .color(colors::VIOLET)
                        .size(font_size::BODY),
                );
            }
        });

        // Progress bar
        if progress.total_frames > 0 {
            ui.add_space(spacing::S2);
            let bar_rect = ui.available_rect_before_wrap();
            let bar_width = (bar_rect.width() - spacing::S4).min(300.0);
            let bar_height = 4.0;

            let (bar_rect, _) = ui.allocate_exact_size(
                egui::vec2(bar_width, bar_height),
                egui::Sense::hover(),
            );

            // Track
            ui.painter().rect_filled(
                bar_rect,
                bar_height / 2.0,
                colors::GRAPHITE,
            );

            // Fill
            let fill_width = bar_rect.width() * (progress.percent / 100.0);
            if fill_width > 0.0 {
                let fill_rect = egui::Rect::from_min_size(
                    bar_rect.left_top(),
                    egui::vec2(fill_width, bar_height),
                );
                ui.painter().rect_filled(
                    fill_rect,
                    bar_height / 2.0,
                    colors::VIOLET,
                );
            }
        }

        // Show single-line live status of most recent frame analysis
        if let Some(latest) = results.last() {
            ui.add_space(spacing::S3);

            // Single-line with timestamp badge and truncated text
            ui.horizontal(|ui| {
                // Compact timestamp badge
                egui::Frame::none()
                    .fill(colors::with_alpha(colors::JADE, 40))
                    .rounding(egui::Rounding::same(radius::SM))
                    .inner_margin(egui::Margin::symmetric(spacing::S2, spacing::S1))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(Self::format_timestamp_compact(latest.timestamp_ms))
                                .color(colors::JADE)
                                .monospace()
                                .size(font_size::SMALL),
                        );
                    });

                ui.add_space(spacing::S2);

                // Truncate text to fit on one line
                let available_width = ui.available_width();
                let text = truncate_to_width(&latest.text, available_width, ui);

                ui.label(
                    egui::RichText::new(text)
                        .color(colors::SILVER)
                        .size(font_size::SMALL),
                );
            });
        }

        ui.add_space(spacing::S2);
    }

    /// Show completed analysis results with summary and expandable details.
    fn show_completed_results(&self, ui: &mut Ui, results: &[FrameResult]) {
        if results.is_empty() {
            ui.label(
                egui::RichText::new("Analysis complete - no results")
                    .color(colors::ASH)
                    .italics()
                    .size(font_size::BODY),
            );
            return;
        }

        // Generate and display summary
        let summary = generate_summary(results);

        // Summary section
        egui::Frame::none()
            .fill(colors::with_alpha(colors::VIOLET, 20))
            .rounding(egui::Rounding::same(radius::MD))
            .inner_margin(egui::Margin::same(spacing::S3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("📊")
                            .size(font_size::BODY),
                    );
                    ui.add_space(spacing::S1);
                    ui.label(
                        egui::RichText::new("Summary")
                            .color(colors::VIOLET)
                            .strong()
                            .size(font_size::BODY),
                    );
                });

                ui.add_space(spacing::S2);

                ui.label(
                    egui::RichText::new(&summary)
                        .color(colors::CHALK)
                        .size(font_size::BODY),
                );
            });

        ui.add_space(spacing::S4);

        // Frame-by-frame results header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Frame Analysis")
                    .color(colors::SILVER)
                    .size(font_size::SMALL),
            );
            ui.add_space(spacing::S2);
            ui.label(
                egui::RichText::new(format!("{} frames", results.len()))
                    .color(colors::ASH)
                    .size(font_size::TINY),
            );
        });

        ui.add_space(spacing::S2);

        // Show all frame results with timestamps
        for result in results {
            ui.horizontal_top(|ui| {
                // Timestamp badge
                egui::Frame::none()
                    .fill(colors::with_alpha(colors::JADE, 30))
                    .rounding(egui::Rounding::same(radius::SM))
                    .inner_margin(egui::Margin::symmetric(spacing::S2, spacing::S1))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(Self::format_timestamp_compact(result.timestamp_ms))
                                .color(colors::JADE)
                                .monospace()
                                .size(font_size::TINY),
                        );
                    });

                ui.add_space(spacing::S2);

                // Result text (wrapped)
                ui.label(
                    egui::RichText::new(&result.text)
                        .color(colors::SILVER)
                        .size(font_size::SMALL),
                );
            });

            ui.add_space(spacing::S1);
        }
    }

    /// Render a system message (centered, subtle).
    fn show_system_message(self, ui: &mut Ui) -> Response {
        let theme_role = yama_theme::components::MessageRole::System;

        ChatBubble::new(theme_role)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(&self.data.content)
                        .color(colors::SILVER)
                        .size(font_size::SMALL),
                );
            })
            .0
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

/// Truncate text to approximately fit within a given width.
fn truncate_to_width(text: &str, max_width: f32, ui: &egui::Ui) -> String {
    // Estimate character width (monospace assumption for simplicity)
    let char_width = ui.fonts(|f| {
        f.glyph_width(&egui::FontId::proportional(font_size::SMALL), 'M')
    });

    // Calculate approximate max chars
    let max_chars = (max_width / char_width).floor() as usize;

    if max_chars < 10 {
        return "...".to_string();
    }

    // Get first line only
    let first_line = text.lines().next().unwrap_or(text);

    if first_line.len() <= max_chars {
        first_line.to_string()
    } else {
        format!("{}...", &first_line[..max_chars.saturating_sub(3)])
    }
}

/// Generate a synthesized summary from frame analysis results.
///
/// This combines all frame observations into a coherent narrative response.
fn generate_summary(results: &[FrameResult]) -> String {
    if results.is_empty() {
        return "No frames analyzed.".to_string();
    }

    let total_frames = results.len();
    let duration_ms = results.last().map(|r| r.timestamp_ms).unwrap_or(0);
    let duration_secs = duration_ms / 1000;

    // Collect all non-empty observations
    let observations: Vec<&str> = results
        .iter()
        .filter_map(|r| {
            let text = r.text.trim();
            if text.is_empty() || is_generic_placeholder(text) {
                None
            } else {
                Some(text)
            }
        })
        .collect();

    // If we have real observations, synthesize them
    if !observations.is_empty() {
        // Combine unique observations into a narrative
        let mut seen = std::collections::HashSet::new();
        let unique_observations: Vec<&str> = observations
            .into_iter()
            .filter(|obs| {
                let key = obs.to_lowercase();
                if seen.contains(&key) {
                    false
                } else {
                    seen.insert(key);
                    true
                }
            })
            .collect();

        // Build narrative summary
        let narrative = unique_observations.join(" ");

        // Add context about duration
        let duration_context = if duration_secs > 60 {
            format!("Over {}:{:02} of video: ", duration_secs / 60, duration_secs % 60)
        } else if duration_secs > 0 {
            format!("Over {} seconds: ", duration_secs)
        } else {
            String::new()
        };

        format!("{}{}", duration_context, truncate_text(&narrative, 500))
    } else {
        // Fallback for mock/placeholder data
        let duration_info = if duration_secs > 0 {
            let mins = duration_secs / 60;
            let secs = duration_secs % 60;
            if mins > 0 {
                format!("{}:{:02}", mins, secs)
            } else {
                format!("{} seconds", secs)
            }
        } else {
            format!("{} frames", total_frames)
        };

        format!(
            "Analyzed {} of video. The VLM processed {} frames but returned placeholder responses. \
             Ensure the VLM container is connected for real analysis.",
            duration_info, total_frames
        )
    }
}

/// Check if text is a generic placeholder that doesn't contain real analysis.
fn is_generic_placeholder(text: &str) -> bool {
    let lower = text.to_lowercase();
    let placeholders = [
        "video analysis initiated",
        "processing first segment",
        "scene establishes",
        "activity detected within",
        "continuing analysis",
        "notable elements identified",
        "temporal progression",
        "visual patterns consistent",
        "analyzing contextual",
        "processing final segments",
        "analysis complete",
    ];
    placeholders.iter().any(|p| lower.contains(p))
}

/// Truncate text to a maximum length with ellipsis.
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}
