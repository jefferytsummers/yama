//! Alert timeline component for temporal event visualization.
//!
//! Displays events on a horizontal timeline with severity-based markers,
//! click-to-seek functionality, and zoom controls.

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use crate::{colors, font_size, radius, spacing};

/// Event severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventSeverity {
    /// Informational event
    #[default]
    Info,
    /// Warning event
    Warning,
    /// Critical event
    Critical,
}

impl EventSeverity {
    /// Get the color for this severity.
    pub fn color(self) -> Color32 {
        match self {
            EventSeverity::Info => colors::AZURE,
            EventSeverity::Warning => colors::AMBER,
            EventSeverity::Critical => colors::EMBER,
        }
    }

    /// Get the label for this severity.
    pub fn label(self) -> &'static str {
        match self {
            EventSeverity::Info => "Info",
            EventSeverity::Warning => "Warning",
            EventSeverity::Critical => "Critical",
        }
    }

    /// Whether this severity should glow.
    pub fn should_glow(self) -> bool {
        matches!(self, EventSeverity::Critical)
    }
}

/// A timeline event.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    /// Unique identifier
    pub id: String,
    /// Event timestamp (milliseconds since epoch)
    pub timestamp_ms: u64,
    /// Event severity
    pub severity: EventSeverity,
    /// Event type/category
    pub event_type: String,
    /// Short title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional source ID (e.g., camera ID)
    pub source_id: Option<String>,
}

impl TimelineEvent {
    /// Create a new timeline event.
    pub fn new(
        id: impl Into<String>,
        timestamp_ms: u64,
        severity: EventSeverity,
        event_type: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            timestamp_ms,
            severity,
            event_type: event_type.into(),
            title: title.into(),
            description: None,
            source_id: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the source ID.
    pub fn with_source(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self
    }
}

/// Action returned from timeline interaction.
#[derive(Debug, Clone, PartialEq)]
pub enum TimelineAction {
    /// No action
    None,
    /// An event was clicked
    EventClick(String),
    /// User seeked to a time
    Seek(u64),
    /// Severity filter changed
    FilterChanged(Vec<EventSeverity>),
}

/// Alert timeline component.
pub struct AlertTimeline<'a> {
    events: &'a [TimelineEvent],
    start_time_ms: u64,
    end_time_ms: u64,
    current_time_ms: Option<u64>,
    zoom: f32,
    severity_filter: Vec<EventSeverity>,
    show_legend: bool,
}

impl<'a> AlertTimeline<'a> {
    /// Create a new alert timeline.
    pub fn new(events: &'a [TimelineEvent], start_time_ms: u64, end_time_ms: u64) -> Self {
        Self {
            events,
            start_time_ms,
            end_time_ms,
            current_time_ms: None,
            zoom: 1.0,
            severity_filter: vec![EventSeverity::Info, EventSeverity::Warning, EventSeverity::Critical],
            show_legend: true,
        }
    }

    /// Set the current playback time.
    pub fn current_time(mut self, time_ms: u64) -> Self {
        self.current_time_ms = Some(time_ms);
        self
    }

    /// Set the zoom level (1.0 = fit to width).
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom.max(0.5).min(10.0);
        self
    }

    /// Set the severity filter.
    pub fn filter(mut self, severities: Vec<EventSeverity>) -> Self {
        self.severity_filter = severities;
        self
    }

    /// Set whether to show the legend.
    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Show the timeline and return any action.
    pub fn show(self, ui: &mut Ui) -> TimelineAction {
        let mut action = TimelineAction::None;

        // Filter events
        let filtered_events: Vec<&TimelineEvent> = self
            .events
            .iter()
            .filter(|e| self.severity_filter.contains(&e.severity))
            .collect();

        // Calculate dimensions
        let available_width = ui.available_width();
        let header_height = 32.0;
        let track_height = 60.0;
        let legend_height = if self.show_legend { 24.0 } else { 0.0 };
        let total_height = header_height + track_height + legend_height + spacing::S4;

        let (response, painter) = ui.allocate_painter(Vec2::new(available_width, total_height), Sense::hover());
        let rect = response.rect;

        // Draw background
        painter.rect_filled(rect, Rounding::same(radius::LG), colors::SURFACE);
        painter.rect_stroke(rect, Rounding::same(radius::LG), Stroke::new(1.0, colors::BORDER));

        // Header
        let header_rect = Rect::from_min_size(rect.min, Vec2::new(available_width, header_height));
        action = self.draw_header(ui, &painter, header_rect, filtered_events.len(), action);

        // Track
        let track_rect = Rect::from_min_size(
            rect.min + Vec2::new(0.0, header_height),
            Vec2::new(available_width, track_height),
        );
        action = self.draw_track(ui, &painter, track_rect, &filtered_events, action);

        // Legend
        if self.show_legend {
            let legend_rect = Rect::from_min_size(
                rect.min + Vec2::new(0.0, header_height + track_height + spacing::S2),
                Vec2::new(available_width, legend_height),
            );
            self.draw_legend(&painter, legend_rect);
        }

        action
    }

    fn draw_header(
        &self,
        _ui: &mut Ui,
        painter: &egui::Painter,
        rect: Rect,
        event_count: usize,
        action: TimelineAction,
    ) -> TimelineAction {
        // Title
        let title_pos = rect.min + Vec2::new(spacing::S4, spacing::S2);
        painter.text(
            title_pos,
            egui::Align2::LEFT_TOP,
            "Event Timeline",
            egui::FontId::proportional(font_size::BODY),
            colors::TEXT_PRIMARY,
        );

        // Event count
        let count_text = format!("{} events", event_count);
        let count_pos = Pos2::new(rect.max.x - spacing::S4, rect.min.y + spacing::S2);
        painter.text(
            count_pos,
            egui::Align2::RIGHT_TOP,
            &count_text,
            egui::FontId::proportional(font_size::SMALL),
            colors::TEXT_MUTED,
        );

        // Severity filter buttons
        let btn_size = Vec2::new(28.0, 20.0);
        let btn_y = rect.min.y + spacing::S1;
        let mut btn_x = rect.max.x - spacing::S4 - 80.0 - (btn_size.x + spacing::S1) * 3.0;

        for severity in [EventSeverity::Info, EventSeverity::Warning, EventSeverity::Critical] {
            let btn_rect = Rect::from_min_size(Pos2::new(btn_x, btn_y), btn_size);
            let is_active = self.severity_filter.contains(&severity);

            // Draw button background
            let bg_color = if is_active {
                colors::SURFACE_HOVER
            } else {
                colors::GRAPHITE
            };
            painter.rect_filled(btn_rect, Rounding::same(radius::SM), bg_color);

            if is_active {
                painter.rect_stroke(btn_rect, Rounding::same(radius::SM), Stroke::new(1.0, colors::AMBER));
            }

            // Draw severity dot
            painter.circle_filled(btn_rect.center(), 5.0, severity.color());

            btn_x += btn_size.x + spacing::S1;
        }

        action
    }

    fn draw_track(
        &self,
        ui: &mut Ui,
        painter: &egui::Painter,
        rect: Rect,
        events: &[&TimelineEvent],
        mut action: TimelineAction,
    ) -> TimelineAction {
        let inner_rect = rect.shrink2(Vec2::new(spacing::S4, spacing::S2));
        let duration_ms = self.end_time_ms.saturating_sub(self.start_time_ms);

        if duration_ms == 0 {
            return action;
        }

        // Draw track background
        let track_bg_rect = Rect::from_min_size(
            inner_rect.min + Vec2::new(0.0, 16.0),
            Vec2::new(inner_rect.width(), inner_rect.height() - 20.0),
        );
        painter.rect_filled(track_bg_rect, Rounding::same(radius::MD), colors::BASALT);
        painter.rect_stroke(track_bg_rect, Rounding::same(radius::MD), Stroke::new(1.0, colors::BORDER));

        // Draw time axis labels
        let num_labels = 6;
        for i in 0..=num_labels {
            let t = i as f32 / num_labels as f32;
            let x = inner_rect.min.x + t * inner_rect.width();
            let time_ms = self.start_time_ms + (t * duration_ms as f32) as u64;
            let time_str = format_time_ms(time_ms);

            painter.text(
                Pos2::new(x, inner_rect.min.y),
                egui::Align2::CENTER_TOP,
                &time_str,
                egui::FontId::monospace(font_size::TINY),
                colors::TEXT_MUTED,
            );

            // Tick mark
            painter.line_segment(
                [
                    Pos2::new(x, track_bg_rect.min.y),
                    Pos2::new(x, track_bg_rect.min.y + 4.0),
                ],
                Stroke::new(1.0, colors::BORDER),
            );
        }

        // Make track interactive for seeking
        let track_response = ui.allocate_rect(track_bg_rect, Sense::click_and_drag());
        if track_response.clicked() || track_response.dragged() {
            if let Some(pos) = track_response.interact_pointer_pos() {
                let t = (pos.x - inner_rect.min.x) / inner_rect.width();
                let t = t.clamp(0.0, 1.0);
                let seek_time = self.start_time_ms + (t * duration_ms as f32) as u64;
                action = TimelineAction::Seek(seek_time);
            }
        }

        // Draw event markers
        let marker_y = track_bg_rect.center().y;

        for event in events {
            let t = (event.timestamp_ms.saturating_sub(self.start_time_ms)) as f32 / duration_ms as f32;
            if t < 0.0 || t > 1.0 {
                continue;
            }

            let x = inner_rect.min.x + t * inner_rect.width();
            let marker_pos = Pos2::new(x, marker_y);
            let color = event.severity.color();

            // Glow for critical events
            if event.severity.should_glow() {
                painter.circle_filled(marker_pos, 8.0, colors::with_alpha(color, 60));
            }

            // Marker dot
            painter.circle_filled(marker_pos, 5.0, color);
            painter.circle_stroke(marker_pos, 5.0, Stroke::new(1.5, colors::BASALT));

            // Tooltip on hover
            let marker_rect = Rect::from_center_size(marker_pos, Vec2::splat(16.0));
            let marker_response = ui.allocate_rect(marker_rect, Sense::click());

            if marker_response.clicked() {
                action = TimelineAction::EventClick(event.id.clone());
            }

            if marker_response.hovered() {
                self.draw_tooltip(ui, event, marker_pos);
            }
        }

        // Draw current time indicator
        if let Some(current_ms) = self.current_time_ms {
            let t = (current_ms.saturating_sub(self.start_time_ms)) as f32 / duration_ms as f32;
            if (0.0..=1.0).contains(&t) {
                let x = inner_rect.min.x + t * inner_rect.width();

                // Vertical line
                painter.line_segment(
                    [
                        Pos2::new(x, track_bg_rect.min.y),
                        Pos2::new(x, track_bg_rect.max.y),
                    ],
                    Stroke::new(2.0, colors::AMBER),
                );

                // Time label
                let time_str = format_time_ms(current_ms);
                let label_rect = Rect::from_center_size(
                    Pos2::new(x, track_bg_rect.min.y - 10.0),
                    Vec2::new(60.0, 16.0),
                );
                painter.rect_filled(label_rect, Rounding::same(radius::SM), colors::BASALT);
                painter.text(
                    label_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &time_str,
                    egui::FontId::monospace(font_size::TINY),
                    colors::AMBER,
                );
            }
        }

        action
    }

    fn draw_tooltip(&self, ui: &mut Ui, event: &TimelineEvent, anchor: Pos2) {
        egui::show_tooltip_at(ui.ctx(), ui.layer_id(), egui::Id::new(&event.id), anchor + Vec2::new(0.0, 16.0), |ui| {
            ui.set_max_width(200.0);

            ui.horizontal(|ui| {
                // Severity dot
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 4.0, event.severity.color());

                ui.label(egui::RichText::new(&event.title).strong().color(colors::TEXT_PRIMARY));
            });

            ui.label(
                egui::RichText::new(format_time_ms(event.timestamp_ms))
                    .monospace()
                    .size(font_size::TINY)
                    .color(colors::TEXT_SECONDARY),
            );

            if let Some(desc) = &event.description {
                ui.label(
                    egui::RichText::new(desc)
                        .size(font_size::SMALL)
                        .color(colors::TEXT_MUTED),
                );
            }
        });
    }

    fn draw_legend(&self, painter: &egui::Painter, rect: Rect) {
        let center_y = rect.center().y;
        let mut x = rect.center().x - 100.0;

        for severity in [EventSeverity::Info, EventSeverity::Warning, EventSeverity::Critical] {
            // Dot
            painter.circle_filled(Pos2::new(x, center_y), 4.0, severity.color());

            // Label
            painter.text(
                Pos2::new(x + 10.0, center_y),
                egui::Align2::LEFT_CENTER,
                severity.label(),
                egui::FontId::proportional(font_size::TINY),
                colors::TEXT_MUTED,
            );

            x += 70.0;
        }
    }
}

/// Format milliseconds timestamp as HH:MM:SS.
fn format_time_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

/// Convenience function to show an alert timeline.
pub fn alert_timeline<'a>(
    ui: &mut Ui,
    events: &'a [TimelineEvent],
    start_ms: u64,
    end_ms: u64,
    current_ms: Option<u64>,
) -> TimelineAction {
    let mut timeline = AlertTimeline::new(events, start_ms, end_ms);
    if let Some(t) = current_ms {
        timeline = timeline.current_time(t);
    }
    timeline.show(ui)
}
