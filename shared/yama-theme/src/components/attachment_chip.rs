//! Attachment chip component for the Yama design system.
//!
//! Compact indicators for file attachments in chat messages.
//! Shows an icon, filename, and optional status indicator.

use eframe::egui::{self, Color32, Response, Rounding, Stroke, Ui, Vec2};

use crate::{colors, font_size, radius, spacing};

/// Status of an attachment in the processing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttachmentStatus {
    /// Attachment is pending (not yet started)
    #[default]
    Pending,
    /// Attachment is ready for processing
    Ready,
    /// Attachment is being processed
    Processing,
    /// Processing completed successfully
    Completed,
    /// Processing failed
    Failed,
}

impl AttachmentStatus {
    /// Get the color for this status.
    pub fn color(self) -> Color32 {
        match self {
            AttachmentStatus::Pending => colors::SILVER,
            AttachmentStatus::Ready => colors::AZURE,
            AttachmentStatus::Processing => colors::VIOLET,
            AttachmentStatus::Completed => colors::JADE,
            AttachmentStatus::Failed => colors::EMBER,
        }
    }

    /// Get the icon for this status.
    pub fn icon(self) -> &'static str {
        match self {
            AttachmentStatus::Pending => "○",
            AttachmentStatus::Ready => "●",
            AttachmentStatus::Processing => "◐",
            AttachmentStatus::Completed => "✓",
            AttachmentStatus::Failed => "✕",
        }
    }

    /// Whether this status should pulse.
    pub fn should_pulse(self) -> bool {
        matches!(self, AttachmentStatus::Processing)
    }
}

/// An attachment chip component.
///
/// # Example
///
/// ```ignore
/// AttachmentChip::new("video.mp4")
///     .file_type(FileType::Video)
///     .status(AttachmentStatus::Processing)
///     .show(ui);
/// ```
pub struct AttachmentChip {
    filename: String,
    file_type: FileType,
    status: AttachmentStatus,
    removable: bool,
    size_bytes: Option<u64>,
}

/// Type of file attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileType {
    #[default]
    Video,
    Image,
    Audio,
    Document,
    Unknown,
}

impl FileType {
    /// Get the icon for this file type.
    pub fn icon(self) -> &'static str {
        match self {
            FileType::Video => "🎬",
            FileType::Image => "🖼",
            FileType::Audio => "🔊",
            FileType::Document => "📄",
            FileType::Unknown => "📎",
        }
    }

    /// Detect file type from filename extension.
    pub fn from_filename(filename: &str) -> Self {
        let ext = filename
            .rsplit('.')
            .next()
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "mp4" | "webm" | "mov" | "avi" | "mkv" | "m4v" => FileType::Video,
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" => FileType::Image,
            "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" => FileType::Audio,
            "pdf" | "doc" | "docx" | "txt" | "md" | "rtf" => FileType::Document,
            _ => FileType::Unknown,
        }
    }
}

impl AttachmentChip {
    /// Create a new attachment chip with the given filename.
    pub fn new(filename: impl Into<String>) -> Self {
        let filename = filename.into();
        let file_type = FileType::from_filename(&filename);
        Self {
            filename,
            file_type,
            status: AttachmentStatus::default(),
            removable: false,
            size_bytes: None,
        }
    }

    /// Set the file type explicitly.
    pub fn file_type(mut self, file_type: FileType) -> Self {
        self.file_type = file_type;
        self
    }

    /// Set the attachment status.
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = status;
        self
    }

    /// Enable the remove button.
    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }

    /// Set the file size for display.
    pub fn size(mut self, bytes: u64) -> Self {
        self.size_bytes = Some(bytes);
        self
    }

    /// Format file size for display.
    fn format_size(bytes: u64) -> String {
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

    /// Truncate filename for display.
    fn truncate_filename(filename: &str, max_len: usize) -> String {
        if filename.len() <= max_len {
            filename.to_string()
        } else {
            // Keep extension, truncate middle
            if let Some(dot_pos) = filename.rfind('.') {
                let ext = &filename[dot_pos..];
                let name_len = max_len.saturating_sub(ext.len() + 3); // 3 for "..."
                if name_len > 3 {
                    format!("{}...{}", &filename[..name_len], ext)
                } else {
                    format!("{}...", &filename[..max_len.saturating_sub(3)])
                }
            } else {
                format!("{}...", &filename[..max_len.saturating_sub(3)])
            }
        }
    }

    /// Display the attachment chip.
    /// Returns (overall_response, remove_clicked)
    pub fn show(self, ui: &mut Ui) -> (Response, bool) {
        let mut remove_clicked = false;

        // Calculate pulse alpha if processing
        let alpha = if self.status.should_pulse() {
            let pulse = crate::animation::pulse_alpha(ui.ctx());
            (pulse * 255.0) as u8
        } else {
            255
        };

        // Background color based on status
        let bg_color = colors::with_alpha(colors::GRAPHITE, 200);
        let border_color = colors::with_alpha(self.status.color(), if self.status.should_pulse() { alpha } else { 128 });

        let frame = egui::Frame::none()
            .fill(bg_color)
            .stroke(Stroke::new(1.0, border_color))
            .rounding(Rounding::same(radius::MD))
            .inner_margin(egui::Margin::symmetric(spacing::S2, spacing::S1));

        let response = frame
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.set_height(20.0);

                    // File type icon
                    ui.label(
                        egui::RichText::new(self.file_type.icon())
                            .size(font_size::BODY),
                    );

                    // Filename (truncated)
                    let display_name = Self::truncate_filename(&self.filename, 20);
                    ui.label(
                        egui::RichText::new(&display_name)
                            .color(colors::CHALK)
                            .size(font_size::SMALL),
                    );

                    // File size if available
                    if let Some(bytes) = self.size_bytes {
                        ui.label(
                            egui::RichText::new(format!("({})", Self::format_size(bytes)))
                                .color(colors::ASH)
                                .size(font_size::TINY),
                        );
                    }

                    // Status indicator
                    let status_color = if self.status.should_pulse() {
                        colors::with_alpha(self.status.color(), alpha)
                    } else {
                        self.status.color()
                    };
                    ui.label(
                        egui::RichText::new(self.status.icon())
                            .color(status_color)
                            .size(font_size::SMALL),
                    );

                    // Remove button if enabled
                    if self.removable {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("✕")
                                        .size(font_size::SMALL)
                                        .color(colors::ASH),
                                )
                                .fill(Color32::TRANSPARENT)
                                .frame(false),
                            )
                            .clicked()
                        {
                            remove_clicked = true;
                        }
                    }
                });
            })
            .response;

        (response, remove_clicked)
    }
}

/// Draw a simple attachment chip.
pub fn attachment_chip(ui: &mut Ui, filename: &str, status: AttachmentStatus) -> Response {
    AttachmentChip::new(filename).status(status).show(ui).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(FileType::from_filename("video.mp4"), FileType::Video);
        assert_eq!(FileType::from_filename("image.PNG"), FileType::Image);
        assert_eq!(FileType::from_filename("audio.wav"), FileType::Audio);
        assert_eq!(FileType::from_filename("doc.pdf"), FileType::Document);
        assert_eq!(FileType::from_filename("unknown.xyz"), FileType::Unknown);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(AttachmentChip::format_size(500), "500 B");
        assert_eq!(AttachmentChip::format_size(1536), "2 KB");
        assert_eq!(AttachmentChip::format_size(1_500_000), "1.4 MB");
        assert_eq!(AttachmentChip::format_size(2_500_000_000), "2.3 GB");
    }

    #[test]
    fn test_truncate_filename() {
        assert_eq!(
            AttachmentChip::truncate_filename("short.mp4", 20),
            "short.mp4"
        );
        assert_eq!(
            AttachmentChip::truncate_filename("this_is_a_very_long_filename.mp4", 20),
            "this_is_a_ver....mp4"
        );
    }

    #[test]
    fn test_status_colors() {
        assert_eq!(AttachmentStatus::Pending.color(), colors::SILVER);
        assert_eq!(AttachmentStatus::Processing.color(), colors::VIOLET);
        assert!(AttachmentStatus::Processing.should_pulse());
        assert!(!AttachmentStatus::Completed.should_pulse());
    }
}
