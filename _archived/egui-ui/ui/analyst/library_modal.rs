//! Library modal - searchable video library with browse functionality.

use eframe::egui;
use yama_theme::{colors, font_size, radius, spacing};

use super::{AnalystState, LibraryVideo};

/// Action from the library modal.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryModalAction {
    None,
    /// Close the modal
    Close,
    /// Browse for files
    BrowseFiles,
    /// Select a video to reference in query
    SelectVideo(String),
    /// Select multiple videos
    SelectVideos(Vec<String>),
    /// Import dropped files
    ImportFiles(Vec<std::path::PathBuf>),
}

/// Library modal component.
pub struct LibraryModal<'a> {
    state: &'a mut AnalystState,
    /// Local search filter
    filter: &'a mut String,
    /// Selected videos in modal
    modal_selection: &'a mut std::collections::HashSet<String>,
    /// Whether to allow multi-select
    multi_select: bool,
}

impl<'a> LibraryModal<'a> {
    pub fn new(
        state: &'a mut AnalystState,
        filter: &'a mut String,
        modal_selection: &'a mut std::collections::HashSet<String>,
    ) -> Self {
        Self {
            state,
            filter,
            modal_selection,
            multi_select: true,
        }
    }

    pub fn multi_select(mut self, enabled: bool) -> Self {
        self.multi_select = enabled;
        self
    }

    /// Show the modal and return action.
    pub fn show(&mut self, ctx: &egui::Context) -> LibraryModalAction {
        let mut action = LibraryModalAction::None;
        let mut open = true;

        // Check for dropped files
        let dropped_files: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        if !dropped_files.is_empty() {
            return LibraryModalAction::ImportFiles(dropped_files);
        }

        egui::Window::new("Video Library")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([700.0, 500.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(
                egui::Frame::none()
                    .fill(colors::SLATE)
                    .rounding(radius::LG)
                    .inner_margin(spacing::S4)
                    .stroke(egui::Stroke::new(1.0, colors::STONE)),
            )
            .show(ctx, |ui| {
                // Header with search and browse
                ui.horizontal(|ui| {
                    // Search input
                    let search_response = ui.add(
                        egui::TextEdit::singleline(self.filter)
                            .desired_width(300.0)
                            .hint_text("Filter videos...")
                            .font(egui::FontId::proportional(font_size::BODY)),
                    );

                    if search_response.changed() {
                        // Filter updates automatically
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Browse button
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("📁 Browse").color(colors::OBSIDIAN),
                                )
                                .fill(colors::AMBER)
                                .rounding(radius::MD),
                            )
                            .clicked()
                        {
                            action = LibraryModalAction::BrowseFiles;
                        }

                        // Selection count
                        if !self.modal_selection.is_empty() {
                            ui.add_space(spacing::S3);
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(format!(
                                            "Use {} selected",
                                            self.modal_selection.len()
                                        ))
                                        .color(colors::OBSIDIAN),
                                    )
                                    .fill(colors::JADE)
                                    .rounding(radius::MD),
                                )
                                .clicked()
                            {
                                action = LibraryModalAction::SelectVideos(
                                    self.modal_selection.iter().cloned().collect(),
                                );
                            }
                        }
                    });
                });

                ui.add_space(spacing::S3);

                // Stats bar
                let indexed = self.state.videos.iter().filter(|v| v.indexed).count();
                let total = self.state.videos.len();
                let filtered = self.filtered_videos().len();

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{} videos", total))
                            .color(colors::SILVER)
                            .size(font_size::SMALL),
                    );
                    ui.label(egui::RichText::new("•").color(colors::STONE));
                    ui.label(
                        egui::RichText::new(format!("{} indexed", indexed))
                            .color(colors::JADE)
                            .size(font_size::SMALL),
                    );

                    if !self.filter.is_empty() {
                        ui.label(egui::RichText::new("•").color(colors::STONE));
                        ui.label(
                            egui::RichText::new(format!("{} shown", filtered))
                                .color(colors::AZURE)
                                .size(font_size::SMALL),
                        );
                    }
                });

                ui.add_space(spacing::S3);
                ui.separator();
                ui.add_space(spacing::S3);

                // Video grid - clone video data to avoid borrow conflicts
                let videos_empty = self.state.videos.is_empty();
                let filtered_video_data: Vec<_> = self.filtered_videos()
                    .into_iter()
                    .map(|v| (v.id.clone(), v.filename.clone(), v.duration_ms, v.size_bytes, v.indexed))
                    .collect();

                if filtered_video_data.is_empty() {
                    // Empty state
                    ui.vertical_centered(|ui| {
                        ui.add_space(spacing::S8);

                        if videos_empty {
                            ui.label(
                                egui::RichText::new("No videos yet")
                                    .color(colors::SILVER)
                                    .size(font_size::H3),
                            );
                            ui.add_space(spacing::S2);
                            ui.label(
                                egui::RichText::new("Drop video files here or click Browse")
                                    .color(colors::ASH),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("No videos match filter")
                                    .color(colors::SILVER)
                                    .size(font_size::H3),
                            );
                        }

                        ui.add_space(spacing::S8);
                    });
                } else {
                    // Collect clicks to process after rendering
                    let mut clicked_id: Option<String> = None;
                    let mut double_clicked_id: Option<String> = None;

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let available_width = ui.available_width();
                            let card_width = 140.0;
                            let card_spacing = spacing::S2;
                            let columns = ((available_width + card_spacing) / (card_width + card_spacing))
                                .floor() as usize;
                            let columns = columns.max(2);

                            egui::Grid::new("library_modal_grid")
                                .spacing([card_spacing, card_spacing])
                                .show(ui, |ui| {
                                    for (i, (id, filename, duration_ms, size_bytes, indexed)) in filtered_video_data.iter().enumerate() {
                                        let is_selected = self.modal_selection.contains(id);
                                        let (click, double_click) = self.show_video_card_data(
                                            ui, id, filename, *duration_ms, *size_bytes, *indexed, is_selected, card_width
                                        );

                                        if click {
                                            clicked_id = Some(id.clone());
                                        }
                                        if double_click {
                                            double_clicked_id = Some(id.clone());
                                        }

                                        if (i + 1) % columns == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });

                    // Process clicks after rendering
                    if let Some(id) = double_clicked_id {
                        action = LibraryModalAction::SelectVideo(id);
                    } else if let Some(id) = clicked_id {
                        if self.multi_select {
                            if self.modal_selection.contains(&id) {
                                self.modal_selection.remove(&id);
                            } else {
                                self.modal_selection.insert(id);
                            }
                        } else {
                            action = LibraryModalAction::SelectVideo(id);
                        }
                    }
                }
            });

        if !open {
            return LibraryModalAction::Close;
        }

        action
    }

    fn filtered_videos(&self) -> Vec<&LibraryVideo> {
        let filter_lower = self.filter.to_lowercase();
        self.state
            .videos
            .iter()
            .filter(|v| {
                filter_lower.is_empty() || v.filename.to_lowercase().contains(&filter_lower)
            })
            .collect()
    }

    /// Show a video card using direct data (avoids borrow conflicts).
    /// Returns (clicked, double_clicked).
    fn show_video_card_data(
        &self,
        ui: &mut egui::Ui,
        id: &str,
        filename: &str,
        duration_ms: u64,
        size_bytes: u64,
        indexed: bool,
        is_selected: bool,
        width: f32,
    ) -> (bool, bool) {
        let height = 120.0;

        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

        // Background
        let bg_color = if is_selected {
            colors::GRAPHITE
        } else if response.hovered() {
            colors::BASALT
        } else {
            colors::OBSIDIAN
        };

        let border_color = if is_selected {
            colors::AMBER
        } else {
            colors::STONE
        };

        ui.painter().rect(
            rect,
            radius::MD,
            bg_color,
            egui::Stroke::new(if is_selected { 2.0 } else { 1.0 }, border_color),
        );

        // Thumbnail area
        let thumb_height = height - 35.0;
        let thumb_rect = egui::Rect::from_min_size(
            rect.min + egui::vec2(1.0, 1.0),
            egui::vec2(width - 2.0, thumb_height),
        );

        ui.painter().rect_filled(
            thumb_rect,
            egui::Rounding {
                nw: radius::MD,
                ne: radius::MD,
                sw: 0.0,
                se: 0.0,
            },
            colors::STONE,
        );

        // Play icon
        ui.painter().text(
            thumb_rect.center(),
            egui::Align2::CENTER_CENTER,
            "▶",
            egui::FontId::proportional(14.0),
            colors::with_alpha(colors::CHALK, 150),
        );

        // Duration badge
        let duration_text = format_duration(duration_ms);
        let dur_rect = egui::Rect::from_min_size(
            thumb_rect.right_bottom() - egui::vec2(42.0, 16.0),
            egui::vec2(40.0, 14.0),
        );
        ui.painter()
            .rect_filled(dur_rect, 2.0, colors::with_alpha(colors::OBSIDIAN, 200));
        ui.painter().text(
            dur_rect.center(),
            egui::Align2::CENTER_CENTER,
            &duration_text,
            egui::FontId::proportional(9.0),
            colors::CHALK,
        );

        // Indexed indicator
        if indexed {
            let badge_pos = thumb_rect.left_top() + egui::vec2(4.0, 4.0);
            ui.painter()
                .circle_filled(badge_pos + egui::vec2(6.0, 6.0), 6.0, colors::JADE);
            ui.painter().text(
                badge_pos + egui::vec2(6.0, 6.0),
                egui::Align2::CENTER_CENTER,
                "✓",
                egui::FontId::proportional(8.0),
                colors::OBSIDIAN,
            );
        }

        // Selection checkbox
        if is_selected || response.hovered() {
            let checkbox_center = thumb_rect.right_top() + egui::vec2(-12.0, 12.0);
            let checkbox_rect = egui::Rect::from_center_size(checkbox_center, egui::vec2(18.0, 18.0));

            if is_selected {
                ui.painter().rect_filled(checkbox_rect, 2.0, colors::AMBER);
                ui.painter().text(
                    checkbox_center,
                    egui::Align2::CENTER_CENTER,
                    "✓",
                    egui::FontId::proportional(10.0),
                    colors::OBSIDIAN,
                );
            } else {
                ui.painter().rect_stroke(
                    checkbox_rect,
                    2.0,
                    egui::Stroke::new(1.5, colors::CHALK),
                );
            }
        }

        // Filename
        let info_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + spacing::S1, thumb_rect.bottom() + 2.0),
            rect.right_bottom() - egui::vec2(spacing::S1, 2.0),
        );

        let mut display_filename = filename.to_string();
        if display_filename.len() > 18 {
            display_filename.truncate(15);
            display_filename.push_str("...");
        }

        ui.painter().text(
            info_rect.left_top(),
            egui::Align2::LEFT_TOP,
            &display_filename,
            egui::FontId::proportional(font_size::TINY),
            colors::CHALK,
        );

        ui.painter().text(
            info_rect.left_bottom() - egui::vec2(0.0, 2.0),
            egui::Align2::LEFT_BOTTOM,
            &format_size(size_bytes),
            egui::FontId::proportional(8.0),
            colors::ASH,
        );

        // Return click states
        (response.clicked(), response.double_clicked())
    }
}

/// Format duration as MM:SS or HH:MM:SS.
fn format_duration(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Format file size as human-readable string.
fn format_size(size_bytes: u64) -> String {
    let kb = size_bytes as f64 / 1024.0;
    let mb = kb / 1024.0;
    let gb = mb / 1024.0;

    if gb >= 1.0 {
        format!("{:.1} GB", gb)
    } else if mb >= 1.0 {
        format!("{:.1} MB", mb)
    } else {
        format!("{:.0} KB", kb)
    }
}
