//! Detection overlay component for AI visualization.
//!
//! Renders bounding boxes, labels, and tracking information over video frames.
//! Follows NVIDIA DeepStream visual conventions.

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Ui, Vec2};

use crate::{colors, font_size, radius};

/// Detection class types for color mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetectionClass {
    /// Person/human detection
    Person,
    /// Vehicle detection (car, truck, bus, etc.)
    Vehicle,
    /// Face detection
    Face,
    /// Animal detection
    Animal,
    /// Generic/other detection
    #[default]
    Other,
}

impl DetectionClass {
    /// Get the color for this detection class.
    pub fn color(self) -> Color32 {
        match self {
            DetectionClass::Person => colors::AZURE,
            DetectionClass::Vehicle => colors::JADE,
            DetectionClass::Face => colors::VIOLET,
            DetectionClass::Animal => colors::EMBER,
            DetectionClass::Other => colors::AMBER,
        }
    }

    /// Parse a class name string into a detection class.
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "person" | "human" | "pedestrian" => DetectionClass::Person,
            "vehicle" | "car" | "truck" | "bus" | "motorcycle" | "bicycle" => DetectionClass::Vehicle,
            "face" | "head" => DetectionClass::Face,
            "animal" | "dog" | "cat" | "bird" => DetectionClass::Animal,
            _ => DetectionClass::Other,
        }
    }
}

/// A single detection result.
#[derive(Debug, Clone)]
pub struct Detection {
    /// Unique identifier
    pub id: String,
    /// Tracking ID (for multi-object tracking)
    pub track_id: Option<u32>,
    /// Detection class name
    pub class_name: String,
    /// Detection class type
    pub class_type: DetectionClass,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Bounding box in normalized coordinates (0.0 - 1.0)
    pub bbox: Rect,
    /// Optional attributes
    pub attributes: Vec<(String, String)>,
}

impl Detection {
    /// Create a new detection.
    pub fn new(
        id: impl Into<String>,
        class_name: impl Into<String>,
        confidence: f32,
        bbox: Rect,
    ) -> Self {
        let class_name = class_name.into();
        let class_type = DetectionClass::from_name(&class_name);
        Self {
            id: id.into(),
            track_id: None,
            class_name,
            class_type,
            confidence,
            bbox,
            attributes: Vec::new(),
        }
    }

    /// Set the tracking ID.
    pub fn with_track_id(mut self, track_id: u32) -> Self {
        self.track_id = Some(track_id);
        self
    }

    /// Add an attribute.
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    /// Get the color for this detection.
    pub fn color(&self) -> Color32 {
        self.class_type.color()
    }
}

/// Trail point for motion visualization.
#[derive(Debug, Clone, Copy)]
pub struct TrailPoint {
    /// Position in normalized coordinates
    pub pos: Pos2,
    /// Age of the point (0.0 = newest, 1.0 = oldest)
    pub age: f32,
}

/// Action returned from detection overlay interaction.
#[derive(Debug, Clone, PartialEq)]
pub enum DetectionOverlayAction {
    /// No action
    None,
    /// A detection was clicked
    Click(String),
    /// A detection was hovered
    Hover(String),
}

/// Detection overlay component.
pub struct DetectionOverlay<'a> {
    detections: &'a [Detection],
    frame_rect: Rect,
    show_labels: bool,
    show_confidence: bool,
    show_trails: bool,
    trails: Option<&'a std::collections::HashMap<u32, Vec<TrailPoint>>>,
    interactive: bool,
}

impl<'a> DetectionOverlay<'a> {
    /// Create a new detection overlay.
    pub fn new(detections: &'a [Detection], frame_rect: Rect) -> Self {
        Self {
            detections,
            frame_rect,
            show_labels: true,
            show_confidence: true,
            show_trails: false,
            trails: None,
            interactive: false,
        }
    }

    /// Set whether to show labels.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set whether to show confidence scores.
    pub fn show_confidence(mut self, show: bool) -> Self {
        self.show_confidence = show;
        self
    }

    /// Set motion trails data.
    pub fn with_trails(mut self, trails: &'a std::collections::HashMap<u32, Vec<TrailPoint>>) -> Self {
        self.show_trails = true;
        self.trails = Some(trails);
        self
    }

    /// Set whether detections are interactive (clickable).
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Show the detection overlay.
    pub fn show(self, ui: &mut Ui) -> DetectionOverlayAction {
        let mut action = DetectionOverlayAction::None;

        // Draw trails first (behind boxes)
        if self.show_trails {
            if let Some(trails) = self.trails {
                for detection in self.detections {
                    if let Some(track_id) = detection.track_id {
                        if let Some(trail) = trails.get(&track_id) {
                            self.draw_trail(ui, trail, detection.color());
                        }
                    }
                }
            }
        }

        // Draw detections
        for detection in self.detections {
            let det_action = self.draw_detection(ui, detection);
            if det_action != DetectionOverlayAction::None {
                action = det_action;
            }
        }

        action
    }

    fn draw_trail(&self, ui: &mut Ui, trail: &[TrailPoint], color: Color32) {
        if trail.len() < 2 {
            return;
        }

        let painter = ui.painter();

        for window in trail.windows(2) {
            let p1 = self.to_screen_pos(window[0].pos);
            let p2 = self.to_screen_pos(window[1].pos);

            // Fade based on age
            let alpha = ((1.0 - window[1].age) * 200.0) as u8;
            let trail_color = colors::with_alpha(color, alpha);

            painter.line_segment([p1, p2], Stroke::new(2.0, trail_color));
        }
    }

    fn draw_detection(&self, ui: &mut Ui, detection: &Detection) -> DetectionOverlayAction {
        let mut action = DetectionOverlayAction::None;

        let screen_rect = self.to_screen_rect(detection.bbox);
        let color = detection.color();

        // Handle interaction if enabled (must be done before borrowing painter)
        if self.interactive {
            let response = ui.allocate_rect(screen_rect, egui::Sense::click_and_drag());
            if response.clicked() {
                action = DetectionOverlayAction::Click(detection.id.clone());
            } else if response.hovered() {
                action = DetectionOverlayAction::Hover(detection.id.clone());
            }
        }

        let painter = ui.painter();

        // Draw bounding box
        painter.rect_stroke(screen_rect, Rounding::ZERO, Stroke::new(2.0, color));

        // Draw corner accents (DeepStream style)
        let corner_len = 8.0_f32.min(screen_rect.width() * 0.2).min(screen_rect.height() * 0.2);
        self.draw_corner_accents(painter, screen_rect, corner_len, color);

        // Draw label
        if self.show_labels {
            self.draw_label(painter, detection, screen_rect, color);
        }

        // Draw attributes
        if !detection.attributes.is_empty() {
            self.draw_attributes(painter, detection, screen_rect, color);
        }

        action
    }

    fn draw_corner_accents(&self, painter: &egui::Painter, rect: Rect, len: f32, color: Color32) {
        let stroke = Stroke::new(3.0, color);

        // Top-left
        painter.line_segment(
            [rect.left_top(), rect.left_top() + Vec2::new(len, 0.0)],
            stroke,
        );
        painter.line_segment(
            [rect.left_top(), rect.left_top() + Vec2::new(0.0, len)],
            stroke,
        );

        // Top-right
        painter.line_segment(
            [rect.right_top(), rect.right_top() + Vec2::new(-len, 0.0)],
            stroke,
        );
        painter.line_segment(
            [rect.right_top(), rect.right_top() + Vec2::new(0.0, len)],
            stroke,
        );

        // Bottom-left
        painter.line_segment(
            [rect.left_bottom(), rect.left_bottom() + Vec2::new(len, 0.0)],
            stroke,
        );
        painter.line_segment(
            [rect.left_bottom(), rect.left_bottom() + Vec2::new(0.0, -len)],
            stroke,
        );

        // Bottom-right
        painter.line_segment(
            [rect.right_bottom(), rect.right_bottom() + Vec2::new(-len, 0.0)],
            stroke,
        );
        painter.line_segment(
            [rect.right_bottom(), rect.right_bottom() + Vec2::new(0.0, -len)],
            stroke,
        );
    }

    fn draw_label(&self, painter: &egui::Painter, detection: &Detection, rect: Rect, color: Color32) {
        // Build label text
        let mut label = String::new();

        if let Some(track_id) = detection.track_id {
            label.push_str(&format!("#{} ", track_id));
        }

        label.push_str(&detection.class_name.to_uppercase());

        if self.show_confidence {
            label.push_str(&format!(" {}%", (detection.confidence * 100.0) as u8));
        }

        // Measure text
        let font = egui::FontId::proportional(font_size::TINY);
        let galley = painter.layout_no_wrap(label.clone(), font.clone(), colors::OBSIDIAN);

        let padding = Vec2::new(6.0, 2.0);
        let label_size = galley.size() + padding * 2.0;

        // Position label above the box
        let label_pos = Pos2::new(rect.min.x, rect.min.y - label_size.y - 2.0);
        let label_rect = Rect::from_min_size(label_pos, label_size);

        // Draw label background
        painter.rect_filled(label_rect, Rounding::same(radius::SM), color);

        // Draw label text
        painter.galley(label_rect.min + padding, galley, colors::OBSIDIAN);
    }

    fn draw_attributes(&self, painter: &egui::Painter, detection: &Detection, rect: Rect, color: Color32) {
        let font = egui::FontId::monospace(font_size::TINY);
        let mut y_offset = 4.0;

        for (key, value) in detection.attributes.iter().take(2) {
            let text = format!("{}: {}", key, value);
            painter.text(
                Pos2::new(rect.min.x + 4.0, rect.max.y + y_offset),
                egui::Align2::LEFT_TOP,
                &text,
                font.clone(),
                colors::with_alpha(color, 200),
            );
            y_offset += 12.0;
        }
    }

    fn to_screen_pos(&self, normalized: Pos2) -> Pos2 {
        Pos2::new(
            self.frame_rect.min.x + normalized.x * self.frame_rect.width(),
            self.frame_rect.min.y + normalized.y * self.frame_rect.height(),
        )
    }

    fn to_screen_rect(&self, normalized: Rect) -> Rect {
        Rect::from_min_max(
            self.to_screen_pos(normalized.min),
            self.to_screen_pos(normalized.max),
        )
    }
}

/// Convenience function to draw a detection overlay.
pub fn detection_overlay(ui: &mut Ui, detections: &[Detection], frame_rect: Rect) -> DetectionOverlayAction {
    DetectionOverlay::new(detections, frame_rect).show(ui)
}
