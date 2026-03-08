//! Chat input component with drag-and-drop support.

use std::path::PathBuf;

use eframe::egui::{self, Ui};
use yama_theme::{colors, font_size, radius, spacing};

use crate::ui::{AttachmentStatus, Conversation, VideoAttachmentData};

/// Action returned from the chat input.
#[derive(Debug)]
pub enum ChatInputAction {
    /// No action
    None,
    /// Send the current message
    Send,
    /// Remove an attachment by ID
    RemoveAttachment(String),
    /// Open file picker
    OpenFilePicker,
}

/// Chat input component with text area and attachments.
pub struct ChatInput<'a> {
    conversation: &'a mut Conversation,
    is_disabled: bool,
}

impl<'a> ChatInput<'a> {
    /// Create a new chat input component.
    pub fn new(conversation: &'a mut Conversation) -> Self {
        Self {
            conversation,
            is_disabled: false,
        }
    }

    /// Disable the input (e.g., while inference is running).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.is_disabled = disabled;
        self
    }

    /// Render the chat input and return any action to take.
    pub fn show(self, ui: &mut Ui) -> ChatInputAction {
        let mut action = ChatInputAction::None;

        // Check for dropped files first
        let dropped_files: Vec<PathBuf> = ui.ctx().input(|i| {
            i.raw.dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        // Add dropped files as attachments
        for path in dropped_files {
            let attachment = VideoAttachmentData::new(path);
            if attachment.is_video() {
                self.conversation.add_draft_attachment(attachment);
            }
        }

        // Check for drag hover state
        let is_hovering = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
        self.conversation.drag_hover = is_hovering;

        // Determine border color based on state
        let border_color = if is_hovering {
            colors::VIOLET
        } else if self.is_disabled {
            colors::GRAPHITE
        } else {
            colors::STONE
        };

        // Main input container
        egui::Frame::none()
            .fill(colors::BASALT)
            .stroke(egui::Stroke::new(2.0, border_color))
            .rounding(egui::Rounding::same(radius::LG))
            .inner_margin(spacing::S3)
            .show(ui, |ui| {
                ui.set_enabled(!self.is_disabled);

                // Show drag overlay when hovering
                if is_hovering {
                    self.show_drag_overlay(ui);
                    return;
                }

                ui.vertical(|ui| {
                    // Attachment chips (if any)
                    if !self.conversation.draft_attachments.is_empty() {
                        action = self.show_attachments(ui);
                        ui.add_space(spacing::S2);
                    }

                    // Input row with text area and buttons
                    ui.horizontal(|ui| {
                        // Attach button
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("📎")
                                        .size(font_size::H3),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .frame(false),
                            )
                            .on_hover_text("Attach video")
                            .clicked()
                        {
                            action = ChatInputAction::OpenFilePicker;
                        }

                        ui.add_space(spacing::S2);

                        // Text input
                        let text_edit = egui::TextEdit::multiline(&mut self.conversation.draft_text)
                            .hint_text("Describe what you want to analyze...")
                            .desired_rows(2)
                            .desired_width(ui.available_width() - 60.0)
                            .font(egui::FontId::proportional(font_size::BODY))
                            .frame(false);

                        let response = ui.add(text_edit);

                        // Submit on Ctrl+Enter or Cmd+Enter
                        if response.has_focus() {
                            let modifiers = ui.ctx().input(|i| i.modifiers);
                            let enter_pressed = ui.ctx().input(|i| i.key_pressed(egui::Key::Enter));

                            if enter_pressed && (modifiers.command || modifiers.ctrl) {
                                if self.conversation.can_send() && !self.is_disabled {
                                    action = ChatInputAction::Send;
                                }
                            }
                        }

                        ui.add_space(spacing::S2);

                        // Send button
                        let can_send = self.conversation.can_send() && !self.is_disabled;
                        let button_color = if can_send {
                            colors::AMBER
                        } else {
                            colors::GRAPHITE
                        };

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("▶")
                                        .size(font_size::H3)
                                        .color(if can_send {
                                            colors::OBSIDIAN
                                        } else {
                                            colors::ASH
                                        }),
                                )
                                .fill(button_color)
                                .rounding(egui::Rounding::same(radius::MD)),
                            )
                            .on_hover_text("Send (Ctrl+Enter)")
                            .clicked()
                            && can_send
                        {
                            action = ChatInputAction::Send;
                        }
                    });

                    // Hint text
                    ui.add_space(spacing::S1);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Ctrl+Enter to send • Drop videos to attach")
                                .size(font_size::TINY)
                                .color(colors::ASH),
                        );
                    });
                });
            });

        action
    }

    /// Show the attachment chips.
    fn show_attachments(&self, ui: &mut Ui) -> ChatInputAction {
        let mut action = ChatInputAction::None;
        let mut to_remove: Option<String> = None;

        ui.horizontal_wrapped(|ui| {
            for attachment in &self.conversation.draft_attachments {
                let remove_clicked = show_attachment_chip(ui, attachment);
                if remove_clicked {
                    to_remove = Some(attachment.id.clone());
                }
            }
        });

        if let Some(id) = to_remove {
            action = ChatInputAction::RemoveAttachment(id);
        }

        action
    }

    /// Show the drag overlay when files are being dragged over.
    fn show_drag_overlay(&self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();

        // Semi-transparent overlay
        ui.painter().rect_filled(
            rect,
            radius::LG,
            colors::with_alpha(colors::VIOLET, 30),
        );

        // Border
        ui.painter().rect_stroke(
            rect,
            radius::LG,
            egui::Stroke::new(2.0, colors::VIOLET),
        );

        // Center content
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(rect.height() / 2.0 - 20.0);

                ui.label(
                    egui::RichText::new("📹")
                        .size(32.0)
                        .color(colors::VIOLET),
                );

                ui.label(
                    egui::RichText::new("Drop video to attach")
                        .size(font_size::BODY)
                        .color(colors::VIOLET)
                        .strong(),
                );
            });
        });
    }
}

/// Show a single attachment chip. Returns true if remove was clicked.
fn show_attachment_chip(ui: &mut Ui, attachment: &VideoAttachmentData) -> bool {
    let mut remove_clicked = false;

    let status_color = yama_theme::components::AttachmentStatus::from(attachment.status).color();

    egui::Frame::none()
        .fill(colors::GRAPHITE)
        .stroke(egui::Stroke::new(1.0, colors::with_alpha(status_color, 128)))
        .rounding(egui::Rounding::same(radius::MD))
        .inner_margin(egui::Margin::symmetric(spacing::S2, spacing::S1))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_height(20.0);

                // Video icon
                ui.label(egui::RichText::new("🎬").size(font_size::BODY));

                // Filename (truncated)
                let display_name = truncate_filename(&attachment.filename, 20);
                ui.label(
                    egui::RichText::new(&display_name)
                        .color(colors::CHALK)
                        .size(font_size::SMALL),
                );

                // File size
                let size_str = format_file_size(attachment.size);
                ui.label(
                    egui::RichText::new(format!("({})", size_str))
                        .color(colors::ASH)
                        .size(font_size::TINY),
                );

                // Remove button
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("✕")
                                .size(font_size::SMALL)
                                .color(colors::ASH),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .frame(false),
                    )
                    .clicked()
                {
                    remove_clicked = true;
                }
            });
        });

    remove_clicked
}

/// Truncate filename for display.
fn truncate_filename(filename: &str, max_len: usize) -> String {
    if filename.len() <= max_len {
        filename.to_string()
    } else if let Some(dot_pos) = filename.rfind('.') {
        let ext = &filename[dot_pos..];
        let name_len = max_len.saturating_sub(ext.len() + 3);
        if name_len > 3 {
            format!("{}...{}", &filename[..name_len], ext)
        } else {
            format!("{}...", &filename[..max_len.saturating_sub(3)])
        }
    } else {
        format!("{}...", &filename[..max_len.saturating_sub(3)])
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
