//! Library view - video grid with thumbnails.

use eframe::egui;
use yama_theme::{colors, font_size, radius, spacing};

use super::{AnalystState, IndexingProgress, IndexingStage, LibraryVideo};

/// Action returned from the library view.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryAction {
    /// No action
    None,
    /// Open video for preview
    OpenVideo(String),
    /// Toggle video selection
    ToggleSelect(String),
    /// Start import from dropped files
    ImportFiles(Vec<std::path::PathBuf>),
    /// Open file picker
    OpenFilePicker,
    /// Toggle pause on indexing
    TogglePauseIndexing,
    /// Cancel indexing
    CancelIndexing,
}

/// Library view component.
pub struct LibraryView<'a> {
    state: &'a mut AnalystState,
}

impl<'a> LibraryView<'a> {
    /// Create a new library view.
    pub fn new(state: &'a mut AnalystState) -> Self {
        Self { state }
    }

    /// Show the library view and return any actions.
    pub fn show(&mut self, ui: &mut egui::Ui) -> LibraryAction {
        let mut action = LibraryAction::None;

        // Check for dropped files
        let dropped_files: Vec<std::path::PathBuf> = ui.ctx().input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        if !dropped_files.is_empty() {
            return LibraryAction::ImportFiles(dropped_files);
        }

        // Header
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new(format!("Library ({} videos)", self.state.videos.len()))
                    .color(colors::CHALK)
                    .size(font_size::H2),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Import button
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("⊕ Import").color(colors::OBSIDIAN),
                        )
                        .fill(colors::AMBER)
                        .rounding(radius::MD),
                    )
                    .clicked()
                {
                    action = LibraryAction::OpenFilePicker;
                }

                // Selection info
                if !self.state.selected_videos.is_empty() {
                    ui.add_space(spacing::S4);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} selected",
                            self.state.selected_videos.len()
                        ))
                        .color(colors::AMBER),
                    );

                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Clear").color(colors::SILVER))
                                .fill(colors::GRAPHITE)
                                .rounding(radius::SM),
                        )
                        .clicked()
                    {
                        self.state.clear_selection();
                    }
                }
            });
        });

        ui.add_space(spacing::S4);

        // Indexing progress overlay
        if let Some(progress) = &self.state.indexing_progress {
            action = self.show_indexing_progress(ui, progress);
            ui.add_space(spacing::S4);
        }

        // Video grid
        let grid_action = self.show_video_grid(ui);
        if grid_action != LibraryAction::None {
            action = grid_action;
        }

        // Drop zone hint when empty
        if self.state.videos.is_empty() {
            self.show_empty_state(ui);
        }

        action
    }

    /// Show the indexing progress overlay.
    fn show_indexing_progress(&self, ui: &mut egui::Ui, progress: &IndexingProgress) -> LibraryAction {
        let mut action = LibraryAction::None;

        egui::Frame::none()
            .fill(colors::SLATE)
            .rounding(radius::LG)
            .inner_margin(spacing::S6)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("◆ Indexing Videos")
                            .color(colors::AMBER)
                            .size(font_size::H3),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Cancel button
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("×").color(colors::SILVER))
                                    .fill(colors::GRAPHITE)
                                    .rounding(radius::SM),
                            )
                            .clicked()
                        {
                            action = LibraryAction::CancelIndexing;
                        }

                        // Pause button
                        let pause_text = if progress.paused { "Resume" } else { "Pause" };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(pause_text).color(colors::CHALK),
                                )
                                .fill(colors::GRAPHITE)
                                .rounding(radius::SM),
                            )
                            .clicked()
                        {
                            action = LibraryAction::TogglePauseIndexing;
                        }
                    });
                });

                ui.add_space(spacing::S3);

                // Stage indicator
                ui.label(
                    egui::RichText::new(format!(
                        "Stage {} of {}: {}",
                        progress.stage.index() + 1,
                        IndexingStage::total(),
                        progress.stage.label()
                    ))
                    .color(colors::SILVER),
                );

                ui.add_space(spacing::S2);

                // Progress bar
                let progress_rect = ui.available_rect_before_wrap();
                let bar_height = 8.0;
                let bar_rect = egui::Rect::from_min_size(
                    progress_rect.min,
                    egui::vec2(progress_rect.width(), bar_height),
                );

                // Track
                ui.painter()
                    .rect_filled(bar_rect, radius::XL, colors::STONE);

                // Fill
                let fill_width = bar_rect.width() * (progress.percent() / 100.0);
                let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(fill_width, bar_height));
                ui.painter().rect_filled(fill_rect, radius::XL, colors::AMBER);

                ui.add_space(bar_height + spacing::S2);

                // Progress text
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{}/{}",
                            progress.processed_videos, progress.total_videos
                        ))
                        .color(colors::CHALK),
                    );

                    if let Some(file) = &progress.current_file {
                        ui.label(egui::RichText::new(file).color(colors::SILVER));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("ETA: {}", progress.eta_string()))
                                .color(colors::SILVER),
                        );
                    });
                });

                // Stage progress list
                ui.add_space(spacing::S3);
                egui::Frame::none()
                    .fill(colors::GRAPHITE)
                    .rounding(radius::MD)
                    .inner_margin(spacing::S3)
                    .show(ui, |ui| {
                        let stages = [
                            IndexingStage::Scanning,
                            IndexingStage::ExtractingKeyframes,
                            IndexingStage::GeneratingEmbeddings,
                            IndexingStage::Transcribing,
                        ];

                        for stage in stages {
                            let (icon, color) = if stage.index() < progress.stage.index() {
                                ("✓", colors::JADE)
                            } else if stage.index() == progress.stage.index() {
                                ("→", colors::AMBER)
                            } else {
                                ("○", colors::ASH)
                            };

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(icon).color(color));
                                ui.label(egui::RichText::new(stage.label()).color(color));

                                if stage.index() < progress.stage.index() {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "({}/{})",
                                            progress.total_videos, progress.total_videos
                                        ))
                                        .color(colors::ASH)
                                        .size(font_size::SMALL),
                                    );
                                } else if stage.index() == progress.stage.index() {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "({}/{})",
                                            progress.processed_videos, progress.total_videos
                                        ))
                                        .color(colors::SILVER)
                                        .size(font_size::SMALL),
                                    );
                                }
                            });
                        }
                    });

                // Errors
                if !progress.errors.is_empty() {
                    ui.add_space(spacing::S2);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Errors: {} videos skipped", progress.errors.len()))
                                .color(colors::EMBER),
                        );
                        if ui
                            .small_button(egui::RichText::new("View Details").color(colors::AZURE))
                            .clicked()
                        {
                            // TODO: Show error details
                        }
                    });
                }
            });

        action
    }

    /// Show the video grid.
    fn show_video_grid(&mut self, ui: &mut egui::Ui) -> LibraryAction {
        let mut action = LibraryAction::None;

        let available_width = ui.available_width();
        let card_width = 180.0;
        let card_height = 160.0;
        let card_spacing = spacing::S3;
        let columns = ((available_width + card_spacing) / (card_width + card_spacing)).floor() as usize;
        let columns = columns.max(2);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("video_grid")
                    .spacing([card_spacing, card_spacing])
                    .show(ui, |ui| {
                        for (i, video) in self.state.videos.iter().enumerate() {
                            let card_action = self.show_video_card(ui, video, card_width, card_height);
                            if card_action != LibraryAction::None {
                                action = card_action;
                            }

                            if (i + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });

        action
    }

    /// Show a single video card.
    fn show_video_card(
        &self,
        ui: &mut egui::Ui,
        video: &LibraryVideo,
        width: f32,
        height: f32,
    ) -> LibraryAction {
        let mut action = LibraryAction::None;
        let is_selected = self.state.selected_videos.contains(&video.id);

        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

        // Background
        let bg_color = if is_selected {
            colors::GRAPHITE
        } else if response.hovered() {
            colors::SLATE
        } else {
            colors::BASALT
        };

        let border_color = if is_selected {
            colors::AMBER
        } else {
            colors::STONE
        };

        ui.painter().rect(
            rect,
            radius::LG,
            bg_color,
            egui::Stroke::new(1.0, border_color),
        );

        // Thumbnail area
        let thumb_height = height - 50.0;
        let thumb_rect = egui::Rect::from_min_size(
            rect.min + egui::vec2(1.0, 1.0),
            egui::vec2(width - 2.0, thumb_height),
        );

        // Placeholder thumbnail
        ui.painter().rect_filled(
            thumb_rect,
            egui::Rounding {
                nw: radius::LG,
                ne: radius::LG,
                sw: 0.0,
                se: 0.0,
            },
            colors::STONE,
        );

        // Play icon in center
        let play_center = thumb_rect.center();
        ui.painter().circle_filled(play_center, 20.0, colors::with_alpha(colors::OBSIDIAN, 180));
        ui.painter().text(
            play_center,
            egui::Align2::CENTER_CENTER,
            "▶",
            egui::FontId::proportional(16.0),
            colors::CHALK,
        );

        // Duration badge
        let duration_text = video.duration_string();
        let duration_rect = egui::Rect::from_min_size(
            thumb_rect.right_bottom() - egui::vec2(50.0, 20.0),
            egui::vec2(46.0, 18.0),
        );
        ui.painter()
            .rect_filled(duration_rect, radius::SM, colors::with_alpha(colors::OBSIDIAN, 200));
        ui.painter().text(
            duration_rect.center(),
            egui::Align2::CENTER_CENTER,
            &duration_text,
            egui::FontId::proportional(font_size::TINY),
            colors::CHALK,
        );

        // Indexed indicator
        if video.indexed {
            let badge_rect = egui::Rect::from_min_size(
                thumb_rect.left_top() + egui::vec2(4.0, 4.0),
                egui::vec2(20.0, 16.0),
            );
            ui.painter()
                .rect_filled(badge_rect, radius::SM, colors::JADE);
            ui.painter().text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✓",
                egui::FontId::proportional(10.0),
                colors::OBSIDIAN,
            );
        }

        // Selection checkbox (top-right)
        if is_selected || response.hovered() {
            let checkbox_center = thumb_rect.right_top() + egui::vec2(-14.0, 14.0);
            let checkbox_rect = egui::Rect::from_center_size(checkbox_center, egui::vec2(20.0, 20.0));

            if is_selected {
                ui.painter().rect_filled(checkbox_rect, radius::SM, colors::AMBER);
                ui.painter().text(
                    checkbox_center,
                    egui::Align2::CENTER_CENTER,
                    "✓",
                    egui::FontId::proportional(12.0),
                    colors::OBSIDIAN,
                );
            } else {
                ui.painter().rect_stroke(
                    checkbox_rect,
                    radius::SM,
                    egui::Stroke::new(1.5, colors::CHALK),
                );
            }
        }

        // Info area
        let info_rect = egui::Rect::from_min_max(
            rect.left_bottom() - egui::vec2(0.0, 50.0),
            rect.right_bottom(),
        );

        // Filename
        let filename_rect = info_rect.shrink2(egui::vec2(spacing::S2, spacing::S1));
        let mut truncated_name = video.filename.clone();
        if truncated_name.len() > 22 {
            truncated_name.truncate(19);
            truncated_name.push_str("...");
        }
        ui.painter().text(
            filename_rect.left_top(),
            egui::Align2::LEFT_TOP,
            &truncated_name,
            egui::FontId::proportional(font_size::SMALL),
            colors::CHALK,
        );

        // Size
        ui.painter().text(
            filename_rect.left_bottom() - egui::vec2(0.0, 4.0),
            egui::Align2::LEFT_BOTTOM,
            &video.size_string(),
            egui::FontId::proportional(font_size::TINY),
            colors::ASH,
        );

        // Handle clicks
        if response.clicked() {
            if ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
                action = LibraryAction::ToggleSelect(video.id.clone());
            } else {
                action = LibraryAction::OpenVideo(video.id.clone());
            }
        }

        if response.secondary_clicked() {
            action = LibraryAction::ToggleSelect(video.id.clone());
        }

        action
    }

    /// Show empty state with drop zone.
    fn show_empty_state(&self, ui: &mut egui::Ui) {
        let available = ui.available_rect_before_wrap();
        let center = available.center();

        // Dashed border drop zone
        let zone_size = egui::vec2(400.0, 200.0);
        let zone_rect = egui::Rect::from_center_size(center, zone_size);

        // Draw dashed border (approximation with dots)
        let painter = ui.painter();
        let dash_len = 8.0;
        let gap_len = 6.0;
        let stroke = egui::Stroke::new(2.0, colors::STONE);

        // Top edge
        let mut x = zone_rect.left();
        while x < zone_rect.right() {
            let end_x = (x + dash_len).min(zone_rect.right());
            painter.line_segment(
                [egui::pos2(x, zone_rect.top()), egui::pos2(end_x, zone_rect.top())],
                stroke,
            );
            x += dash_len + gap_len;
        }

        // Bottom edge
        let mut x = zone_rect.left();
        while x < zone_rect.right() {
            let end_x = (x + dash_len).min(zone_rect.right());
            painter.line_segment(
                [egui::pos2(x, zone_rect.bottom()), egui::pos2(end_x, zone_rect.bottom())],
                stroke,
            );
            x += dash_len + gap_len;
        }

        // Left edge
        let mut y = zone_rect.top();
        while y < zone_rect.bottom() {
            let end_y = (y + dash_len).min(zone_rect.bottom());
            painter.line_segment(
                [egui::pos2(zone_rect.left(), y), egui::pos2(zone_rect.left(), end_y)],
                stroke,
            );
            y += dash_len + gap_len;
        }

        // Right edge
        let mut y = zone_rect.top();
        while y < zone_rect.bottom() {
            let end_y = (y + dash_len).min(zone_rect.bottom());
            painter.line_segment(
                [egui::pos2(zone_rect.right(), y), egui::pos2(zone_rect.right(), end_y)],
                stroke,
            );
            y += dash_len + gap_len;
        }

        // Icon
        painter.text(
            center - egui::vec2(0.0, 40.0),
            egui::Align2::CENTER_CENTER,
            "⬇",
            egui::FontId::proportional(48.0),
            colors::STONE,
        );

        // Text
        painter.text(
            center + egui::vec2(0.0, 20.0),
            egui::Align2::CENTER_CENTER,
            "Drop video files or folders here",
            egui::FontId::proportional(font_size::BODY),
            colors::SILVER,
        );

        painter.text(
            center + egui::vec2(0.0, 45.0),
            egui::Align2::CENTER_CENTER,
            "or click Import to browse",
            egui::FontId::proportional(font_size::SMALL),
            colors::ASH,
        );
    }
}
