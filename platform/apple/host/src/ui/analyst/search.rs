//! Search bar and results for the Content Analyst view.

use eframe::egui;
use yama_theme::{colors, font_size, radius, spacing};

use super::{AnalystState, MatchType, SearchMode, SearchResult};

/// Action returned from search components.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchAction {
    /// No action
    None,
    /// Execute search
    Search,
    /// Clear search
    Clear,
    /// Change search mode
    SetMode(SearchMode),
    /// Preview a search result
    PreviewResult { video_id: String, timestamp_ms: u64 },
    /// Toggle selection for a result
    ToggleSelectResult(String),
    /// Extract selected results as clips
    ExtractSelected,
}

/// Search bar component.
pub struct SearchBar<'a> {
    state: &'a mut AnalystState,
}

impl<'a> SearchBar<'a> {
    /// Create a new search bar.
    pub fn new(state: &'a mut AnalystState) -> Self {
        Self { state }
    }

    /// Show the search bar and return any actions.
    pub fn show(&mut self, ui: &mut egui::Ui) -> SearchAction {
        let mut action = SearchAction::None;

        ui.horizontal(|ui| {
            // Search input
            let text_edit = egui::TextEdit::singleline(&mut self.state.search_query)
                .desired_width(ui.available_width() - 200.0)
                .hint_text("Search videos by content...")
                .font(egui::FontId::proportional(font_size::BODY));

            let response = ui.add(text_edit);

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                action = SearchAction::Search;
            }

            // Search button
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("🔍 Search").color(colors::OBSIDIAN))
                        .fill(colors::AMBER)
                        .rounding(radius::MD),
                )
                .clicked()
            {
                action = SearchAction::Search;
            }

            // Clear button (if there's a query)
            if !self.state.search_query.is_empty() {
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("Clear").color(colors::SILVER))
                            .fill(colors::GRAPHITE)
                            .rounding(radius::SM),
                    )
                    .clicked()
                {
                    action = SearchAction::Clear;
                }
            }
        });

        // Mode tabs (only show when searching)
        if self.state.search_results.is_some() || !self.state.search_query.is_empty() {
            ui.add_space(spacing::S2);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Mode:").color(colors::SILVER).size(font_size::SMALL));

                for mode in SearchMode::all() {
                    let is_selected = self.state.search_mode == *mode;
                    let (bg, fg) = if is_selected {
                        (colors::AMBER, colors::OBSIDIAN)
                    } else {
                        (colors::GRAPHITE, colors::SILVER)
                    };

                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(mode.label()).color(fg))
                                .fill(bg)
                                .rounding(radius::SM),
                        )
                        .clicked()
                    {
                        action = SearchAction::SetMode(*mode);
                    }
                }
            });
        }

        action
    }
}

/// Search results list component.
pub struct SearchResults<'a> {
    state: &'a mut AnalystState,
}

impl<'a> SearchResults<'a> {
    /// Create a new search results component.
    pub fn new(state: &'a mut AnalystState) -> Self {
        Self { state }
    }

    /// Show the search results and return any actions.
    pub fn show(&mut self, ui: &mut egui::Ui) -> SearchAction {
        let mut action = SearchAction::None;

        let results = match &self.state.search_results {
            Some(r) => r.clone(),
            None => return action,
        };

        if results.is_empty() {
            // No results message
            egui::Frame::none()
                .fill(colors::SLATE)
                .rounding(radius::MD)
                .inner_margin(spacing::S6)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("No results found")
                                .color(colors::SILVER)
                                .size(font_size::H3),
                        );
                        ui.add_space(spacing::S2);
                        ui.label(
                            egui::RichText::new("Try different search terms or adjust the search mode")
                                .color(colors::ASH),
                        );
                    });
                });
            return action;
        }

        // Results header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} results", results.len()))
                    .color(colors::CHALK)
                    .size(font_size::BODY),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let selected_count = self.state.selected_videos.len();
                if selected_count > 0 {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(format!("Extract Selected ({})", selected_count))
                                    .color(colors::OBSIDIAN),
                            )
                            .fill(colors::JADE)
                            .rounding(radius::MD),
                        )
                        .clicked()
                    {
                        action = SearchAction::ExtractSelected;
                    }
                }
            });
        });

        ui.add_space(spacing::S3);

        // Results list
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for result in &results {
                    let result_action = self.show_result_row(ui, result);
                    if result_action != SearchAction::None {
                        action = result_action;
                    }
                    ui.add_space(spacing::S2);
                }
            });

        action
    }

    /// Show a single result row.
    fn show_result_row(&self, ui: &mut egui::Ui, result: &SearchResult) -> SearchAction {
        let mut action = SearchAction::None;
        let is_selected = self.state.selected_videos.contains(&result.video_id);

        let row_height = 80.0;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_height),
            egui::Sense::click(),
        );

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
            radius::MD,
            bg_color,
            egui::Stroke::new(1.0, border_color),
        );

        // Layout: [checkbox] [thumbnail] [info] [score]
        let checkbox_width = 30.0;
        let thumb_width = 100.0;
        let score_width = 60.0;

        // Checkbox
        let checkbox_center = rect.left_center() + egui::vec2(checkbox_width / 2.0, 0.0);
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
                egui::Stroke::new(1.5, colors::STONE),
            );
        }

        // Thumbnail placeholder
        let thumb_rect = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(checkbox_width, spacing::S2),
            egui::vec2(thumb_width, row_height - spacing::S2 * 2.0),
        );
        ui.painter().rect_filled(thumb_rect, radius::SM, colors::STONE);

        // Play icon
        ui.painter().text(
            thumb_rect.center(),
            egui::Align2::CENTER_CENTER,
            "▶",
            egui::FontId::proportional(20.0),
            colors::with_alpha(colors::CHALK, 150),
        );

        // Timestamp badge
        let ts_text = result.timestamp_string();
        let ts_rect = egui::Rect::from_min_size(
            thumb_rect.left_bottom() - egui::vec2(0.0, 18.0),
            egui::vec2(thumb_width, 16.0),
        );
        ui.painter()
            .rect_filled(ts_rect, radius::SM, colors::with_alpha(colors::OBSIDIAN, 200));
        ui.painter().text(
            ts_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("@ {}", ts_text),
            egui::FontId::proportional(font_size::TINY),
            colors::CHALK,
        );

        // Info area
        let info_left = checkbox_width + thumb_width + spacing::S3;
        let info_right = rect.right() - score_width - spacing::S3;
        let info_rect = egui::Rect::from_x_y_ranges(
            info_left..=info_right,
            rect.top() + spacing::S2..=rect.bottom() - spacing::S2,
        );

        // Filename
        ui.painter().text(
            info_rect.left_top(),
            egui::Align2::LEFT_TOP,
            &result.video_filename,
            egui::FontId::proportional(font_size::BODY),
            colors::CHALK,
        );

        // Match type badge
        let match_color = match result.match_type {
            MatchType::Transcript => colors::AZURE,
            MatchType::Visual => colors::VIOLET,
            MatchType::Both => colors::JADE,
        };
        let badge_text = result.match_type.label();
        let badge_rect = egui::Rect::from_min_size(
            info_rect.left_top() + egui::vec2(0.0, font_size::BODY + 4.0),
            egui::vec2(65.0, 16.0),
        );
        ui.painter().rect_filled(badge_rect, radius::SM, match_color);
        ui.painter().text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            badge_text,
            egui::FontId::proportional(font_size::TINY),
            colors::OBSIDIAN,
        );

        // Snippet
        let snippet_top = badge_rect.bottom() + spacing::S1;
        let max_snippet_len = 80;
        let snippet = if result.snippet.len() > max_snippet_len {
            format!("{}...", &result.snippet[..max_snippet_len])
        } else {
            result.snippet.clone()
        };
        ui.painter().text(
            egui::pos2(info_rect.left(), snippet_top),
            egui::Align2::LEFT_TOP,
            &snippet,
            egui::FontId::proportional(font_size::SMALL),
            colors::SILVER,
        );

        // Score
        let score_center = egui::pos2(rect.right() - score_width / 2.0, rect.center().y);
        let score_text = format!("{:.0}%", result.score * 100.0);
        let score_color = if result.score > 0.8 {
            colors::JADE
        } else if result.score > 0.5 {
            colors::AMBER
        } else {
            colors::SILVER
        };
        ui.painter().text(
            score_center,
            egui::Align2::CENTER_CENTER,
            &score_text,
            egui::FontId::proportional(font_size::H3),
            score_color,
        );

        // Handle clicks
        if response.clicked() {
            // Check if clicking on checkbox area
            let click_pos = response.interact_pointer_pos().unwrap_or(rect.center());
            if click_pos.x < rect.left() + checkbox_width + 10.0 {
                action = SearchAction::ToggleSelectResult(result.video_id.clone());
            } else {
                action = SearchAction::PreviewResult {
                    video_id: result.video_id.clone(),
                    timestamp_ms: result.timestamp_ms,
                };
            }
        }

        action
    }
}
