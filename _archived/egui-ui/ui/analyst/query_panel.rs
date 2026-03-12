//! Query panel - natural language queries and tool results.

use eframe::egui;
use yama_theme::{colors, font_size, radius, spacing};

use super::{AnalystState, MatchType, SearchResult};

/// A query/response pair in the conversation.
#[derive(Clone)]
pub struct QueryExchange {
    /// Unique ID
    pub id: String,
    /// User's query
    pub query: String,
    /// Results from the query
    pub results: Vec<SearchResult>,
    /// Tool calls made
    pub tool_calls: Vec<ToolCall>,
    /// Summary text (if generated)
    pub summary: Option<String>,
    /// Whether this exchange is still processing
    pub processing: bool,
}

impl std::fmt::Debug for QueryExchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryExchange")
            .field("id", &self.id)
            .field("query", &self.query)
            .field("results_count", &self.results.len())
            .finish()
    }
}

/// A tool call made during query processing.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Tool name
    pub name: String,
    /// Tool arguments (display string)
    pub args: String,
    /// Tool result (if complete)
    pub result: Option<ToolResult>,
    /// Whether this tool is still running
    pub running: bool,
}

/// Result from a tool call.
#[derive(Debug, Clone)]
pub enum ToolResult {
    /// Search results
    SearchResults(usize),
    /// Clip extracted
    ClipExtracted { path: String, duration_ms: u64 },
    /// Summary generated
    Summary(String),
    /// Frame analyzed
    FrameAnalysis { timestamp_ms: u64, description: String },
    /// Error
    Error(String),
}

/// Action from the query panel.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryAction {
    None,
    /// Submit a new query
    SubmitQuery(String),
    /// Preview a result
    PreviewResult { video_id: String, timestamp_ms: u64 },
    /// Extract clip from result
    ExtractClip { video_id: String, start_ms: u64, end_ms: u64 },
    /// Analyze a specific frame
    AnalyzeFrame { video_id: String, timestamp_ms: u64 },
    /// Generate summary from results
    GenerateSummary,
}

/// Query panel state.
#[derive(Debug, Default)]
pub struct QueryPanelState {
    /// Current query input
    pub query_input: String,
    /// Query history
    pub exchanges: Vec<QueryExchange>,
    /// Selected results for batch operations
    pub selected_result_indices: std::collections::HashSet<usize>,
}

impl QueryPanelState {
    /// Add a new exchange.
    pub fn add_exchange(&mut self, query: String) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.exchanges.push(QueryExchange {
            id: id.clone(),
            query,
            results: Vec::new(),
            tool_calls: Vec::new(),
            summary: None,
            processing: true,
        });
        id
    }

    /// Get the latest exchange.
    pub fn latest_exchange_mut(&mut self) -> Option<&mut QueryExchange> {
        self.exchanges.last_mut()
    }

    /// Mock: simulate completing a query with results.
    pub fn mock_complete_query(&mut self, results: Vec<SearchResult>) {
        if let Some(exchange) = self.exchanges.last_mut() {
            exchange.results = results;
            exchange.tool_calls = vec![
                ToolCall {
                    name: "search_videos".to_string(),
                    args: format!("query=\"{}\"", exchange.query),
                    result: Some(ToolResult::SearchResults(exchange.results.len())),
                    running: false,
                },
            ];
            exchange.processing = false;
        }
    }

    /// Add a video reference to the query input.
    pub fn add_video_reference(&mut self, filename: &str) {
        if !self.query_input.is_empty() {
            self.query_input.push_str(" ");
        }
        self.query_input.push_str(&format!("@[{}]", filename));
    }
}

/// Query panel component.
pub struct QueryPanel<'a> {
    state: &'a mut QueryPanelState,
    analyst_state: &'a AnalystState,
}

impl<'a> QueryPanel<'a> {
    pub fn new(state: &'a mut QueryPanelState, analyst_state: &'a AnalystState) -> Self {
        Self { state, analyst_state }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> QueryAction {
        let mut action = QueryAction::None;

        // Header
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("Ask about your videos")
                    .color(colors::CHALK)
                    .size(font_size::H3),
            );
        });

        ui.add_space(spacing::S3);

        // Library status
        let indexed_count = self.analyst_state.videos.iter().filter(|v| v.indexed).count();
        let total_count = self.analyst_state.videos.len();

        if indexed_count == 0 {
            // No videos indexed yet
            egui::Frame::none()
                .fill(colors::SLATE)
                .rounding(radius::MD)
                .inner_margin(spacing::S4)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("No videos indexed yet")
                                .color(colors::SILVER)
                                .size(font_size::BODY),
                        );
                        ui.add_space(spacing::S2);
                        ui.label(
                            egui::RichText::new("Import videos to start searching")
                                .color(colors::ASH)
                                .size(font_size::SMALL),
                        );
                    });
                });
            return action;
        }

        // Indexed status badge
        egui::Frame::none()
            .fill(colors::GRAPHITE)
            .rounding(radius::SM)
            .inner_margin(egui::Margin::symmetric(spacing::S3, spacing::S2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("●").color(colors::JADE).size(10.0));
                    ui.label(
                        egui::RichText::new(format!("{} videos indexed", indexed_count))
                            .color(colors::SILVER)
                            .size(font_size::SMALL),
                    );
                });
            });

        ui.add_space(spacing::S4);

        // Query input
        let input_action = self.show_query_input(ui);
        if input_action != QueryAction::None {
            action = input_action;
        }

        ui.add_space(spacing::S4);

        // Conversation history
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for exchange in &self.state.exchanges {
                    let exchange_action = self.show_exchange(ui, exchange);
                    if exchange_action != QueryAction::None {
                        action = exchange_action;
                    }
                    ui.add_space(spacing::S4);
                }
            });

        action
    }

    fn show_query_input(&mut self, ui: &mut egui::Ui) -> QueryAction {
        let mut action = QueryAction::None;

        egui::Frame::none()
            .fill(colors::SLATE)
            .rounding(radius::MD)
            .inner_margin(spacing::S3)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let text_edit = egui::TextEdit::singleline(&mut self.state.query_input)
                        .desired_width(ui.available_width() - 80.0)
                        .hint_text("Ask a question about your videos...")
                        .font(egui::FontId::proportional(font_size::BODY));

                    let response = ui.add(text_edit);

                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.state.query_input.trim().is_empty() {
                            action = QueryAction::SubmitQuery(self.state.query_input.clone());
                        }
                    }

                    let can_submit = !self.state.query_input.trim().is_empty();
                    let button_color = if can_submit { colors::AMBER } else { colors::STONE };

                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Ask").color(colors::OBSIDIAN))
                                .fill(button_color)
                                .rounding(radius::SM),
                        )
                        .clicked()
                        && can_submit
                    {
                        action = QueryAction::SubmitQuery(self.state.query_input.clone());
                    }
                });

                // Example queries
                ui.add_space(spacing::S2);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Try:")
                            .color(colors::ASH)
                            .size(font_size::TINY),
                    );

                    let examples = [
                        "Find mentions of 'quarterly results'",
                        "Show frames with charts",
                        "Summarize the meeting topics",
                    ];

                    for example in examples {
                        if ui
                            .small_button(egui::RichText::new(example).color(colors::AZURE).size(font_size::TINY))
                            .clicked()
                        {
                            self.state.query_input = example.to_string();
                        }
                    }
                });
            });

        action
    }

    fn show_exchange(&self, ui: &mut egui::Ui, exchange: &QueryExchange) -> QueryAction {
        let mut action = QueryAction::None;

        // User query bubble
        egui::Frame::none()
            .fill(colors::GRAPHITE)
            .rounding(radius::MD)
            .inner_margin(spacing::S3)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("You").color(colors::AMBER).strong());
                });
                ui.label(egui::RichText::new(&exchange.query).color(colors::CHALK));
            });

        ui.add_space(spacing::S2);

        // Tool calls
        for tool in &exchange.tool_calls {
            self.show_tool_call(ui, tool);
            ui.add_space(spacing::S1);
        }

        // Results
        if !exchange.results.is_empty() {
            ui.add_space(spacing::S2);

            egui::Frame::none()
                .fill(colors::SLATE)
                .rounding(radius::MD)
                .inner_margin(spacing::S3)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Found {} results", exchange.results.len()))
                                .color(colors::JADE)
                                .strong(),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button(egui::RichText::new("Summarize").color(colors::AZURE))
                                .clicked()
                            {
                                action = QueryAction::GenerateSummary;
                            }
                        });
                    });

                    ui.add_space(spacing::S2);

                    // Result cards
                    for result in &exchange.results {
                        let result_action = self.show_result_card(ui, result);
                        if result_action != QueryAction::None {
                            action = result_action;
                        }
                        ui.add_space(spacing::S2);
                    }
                });
        }

        // Summary (if generated)
        if let Some(summary) = &exchange.summary {
            ui.add_space(spacing::S2);
            egui::Frame::none()
                .fill(colors::VIOLET.linear_multiply(0.2))
                .rounding(radius::MD)
                .inner_margin(spacing::S3)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Summary").color(colors::VIOLET).strong());
                    ui.add_space(spacing::S1);
                    ui.label(egui::RichText::new(summary).color(colors::CHALK));
                });
        }

        // Processing indicator
        if exchange.processing {
            ui.add_space(spacing::S2);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(egui::RichText::new("Searching...").color(colors::SILVER));
            });
        }

        action
    }

    fn show_tool_call(&self, ui: &mut egui::Ui, tool: &ToolCall) {
        egui::Frame::none()
            .fill(colors::BASALT)
            .rounding(radius::SM)
            .inner_margin(egui::Margin::symmetric(spacing::S2, spacing::S1))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Tool icon
                    let icon = match tool.name.as_str() {
                        "search_videos" => "🔍",
                        "extract_clip" => "✂",
                        "analyze_frame" => "🔬",
                        "generate_summary" => "📝",
                        _ => "⚙",
                    };
                    ui.label(egui::RichText::new(icon).size(font_size::SMALL));

                    // Tool name
                    ui.label(
                        egui::RichText::new(&tool.name)
                            .color(colors::AZURE)
                            .size(font_size::SMALL),
                    );

                    // Args
                    ui.label(
                        egui::RichText::new(&tool.args)
                            .color(colors::ASH)
                            .size(font_size::TINY),
                    );

                    // Status
                    if tool.running {
                        ui.spinner();
                    } else if let Some(result) = &tool.result {
                        let result_text = match result {
                            ToolResult::SearchResults(n) => format!("→ {} results", n),
                            ToolResult::ClipExtracted { path, .. } => format!("→ {}", path),
                            ToolResult::Summary(_) => "→ Done".to_string(),
                            ToolResult::FrameAnalysis { .. } => "→ Done".to_string(),
                            ToolResult::Error(e) => format!("✗ {}", e),
                        };
                        let color = match result {
                            ToolResult::Error(_) => colors::EMBER,
                            _ => colors::JADE,
                        };
                        ui.label(egui::RichText::new(result_text).color(color).size(font_size::TINY));
                    }
                });
            });
    }

    fn show_result_card(&self, ui: &mut egui::Ui, result: &SearchResult) -> QueryAction {
        let mut action = QueryAction::None;

        let card_height = 70.0;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), card_height),
            egui::Sense::click(),
        );

        // Background
        let bg = if response.hovered() { colors::GRAPHITE } else { colors::BASALT };
        ui.painter().rect_filled(rect, radius::SM, bg);

        // Layout
        let thumb_width = 90.0;
        let padding = spacing::S2;

        // Thumbnail placeholder
        let thumb_rect = egui::Rect::from_min_size(
            rect.min + egui::vec2(padding, padding),
            egui::vec2(thumb_width, card_height - padding * 2.0),
        );
        ui.painter().rect_filled(thumb_rect, radius::SM, colors::STONE);
        ui.painter().text(
            thumb_rect.center(),
            egui::Align2::CENTER_CENTER,
            "▶",
            egui::FontId::proportional(16.0),
            colors::with_alpha(colors::CHALK, 150),
        );

        // Timestamp on thumbnail
        let ts_rect = egui::Rect::from_min_size(
            thumb_rect.left_bottom() - egui::vec2(0.0, 16.0),
            egui::vec2(thumb_width, 14.0),
        );
        ui.painter().rect_filled(ts_rect, 2.0, colors::with_alpha(colors::OBSIDIAN, 200));
        ui.painter().text(
            ts_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("@ {}", result.timestamp_string()),
            egui::FontId::proportional(font_size::TINY),
            colors::CHALK,
        );

        // Info
        let info_left = rect.min.x + thumb_width + padding * 2.0;
        let info_top = rect.min.y + padding;

        // Filename
        ui.painter().text(
            egui::pos2(info_left, info_top),
            egui::Align2::LEFT_TOP,
            &result.video_filename,
            egui::FontId::proportional(font_size::SMALL),
            colors::CHALK,
        );

        // Match type badge
        let match_color = match result.match_type {
            MatchType::Transcript => colors::AZURE,
            MatchType::Visual => colors::VIOLET,
            MatchType::Both => colors::JADE,
        };
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(info_left, info_top + font_size::SMALL + 2.0),
            egui::vec2(55.0, 14.0),
        );
        ui.painter().rect_filled(badge_rect, 2.0, match_color);
        ui.painter().text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            result.match_type.label(),
            egui::FontId::proportional(9.0),
            colors::OBSIDIAN,
        );

        // Snippet
        let snippet_top = badge_rect.bottom() + 4.0;
        let max_len = 60;
        let snippet = if result.snippet.len() > max_len {
            format!("{}...", &result.snippet[..max_len])
        } else {
            result.snippet.clone()
        };
        ui.painter().text(
            egui::pos2(info_left, snippet_top),
            egui::Align2::LEFT_TOP,
            &snippet,
            egui::FontId::proportional(font_size::TINY),
            colors::SILVER,
        );

        // Action buttons (right side)
        let button_width = 60.0;
        let button_height = 24.0;
        let buttons_right = rect.right() - padding;

        // Extract clip button
        let extract_rect = egui::Rect::from_min_size(
            egui::pos2(buttons_right - button_width, rect.center().y - button_height / 2.0 - 14.0),
            egui::vec2(button_width, button_height),
        );

        let extract_hovered = response.hovered() && extract_rect.contains(response.hover_pos().unwrap_or_default());
        let extract_bg = if extract_hovered { colors::JADE } else { colors::GRAPHITE };
        let extract_fg = if extract_hovered { colors::OBSIDIAN } else { colors::JADE };

        ui.painter().rect_filled(extract_rect, radius::SM, extract_bg);
        ui.painter().text(
            extract_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✂ Clip",
            egui::FontId::proportional(font_size::TINY),
            extract_fg,
        );

        // Analyze button
        let analyze_rect = egui::Rect::from_min_size(
            egui::pos2(buttons_right - button_width, rect.center().y - button_height / 2.0 + 14.0),
            egui::vec2(button_width, button_height),
        );

        let analyze_hovered = response.hovered() && analyze_rect.contains(response.hover_pos().unwrap_or_default());
        let analyze_bg = if analyze_hovered { colors::VIOLET } else { colors::GRAPHITE };
        let analyze_fg = if analyze_hovered { colors::OBSIDIAN } else { colors::VIOLET };

        ui.painter().rect_filled(analyze_rect, radius::SM, analyze_bg);
        ui.painter().text(
            analyze_rect.center(),
            egui::Align2::CENTER_CENTER,
            "🔬 Analyze",
            egui::FontId::proportional(font_size::TINY),
            analyze_fg,
        );

        // Handle clicks
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if extract_rect.contains(pos) {
                    action = QueryAction::ExtractClip {
                        video_id: result.video_id.clone(),
                        start_ms: result.timestamp_ms.saturating_sub(5000),
                        end_ms: result.timestamp_ms + 10000,
                    };
                } else if analyze_rect.contains(pos) {
                    action = QueryAction::AnalyzeFrame {
                        video_id: result.video_id.clone(),
                        timestamp_ms: result.timestamp_ms,
                    };
                } else {
                    action = QueryAction::PreviewResult {
                        video_id: result.video_id.clone(),
                        timestamp_ms: result.timestamp_ms,
                    };
                }
            }
        }

        action
    }
}
