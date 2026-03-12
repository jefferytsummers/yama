//! Conversation thread component - scrollable message list.

use eframe::egui::{self, Ui};
use yama_theme::{colors, spacing};

use crate::ui::{Conversation, MessageRole};

use super::ChatMessage;

/// Renders the conversation thread with all messages.
pub struct ConversationThread<'a> {
    conversation: &'a Conversation,
}

impl<'a> ConversationThread<'a> {
    /// Create a new conversation thread renderer.
    pub fn new(conversation: &'a Conversation) -> Self {
        Self { conversation }
    }

    /// Render the conversation thread.
    pub fn show(self, ui: &mut Ui) {
        let has_messages = !self.conversation.messages.is_empty();

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                if has_messages {
                    self.show_messages(ui);
                } else {
                    self.show_empty_state(ui);
                }

                // Add some padding at the bottom
                ui.add_space(spacing::S4);
            });
    }

    /// Render all messages in the conversation.
    fn show_messages(&self, ui: &mut Ui) {
        ui.add_space(spacing::S4);

        for message in &self.conversation.messages {
            // Add spacing between messages
            ui.add_space(spacing::S3);

            // Add horizontal padding
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(spacing::S4, 0.0))
                .show(ui, |ui| {
                    // Layout based on role
                    match message.role {
                        MessageRole::User => {
                            // Right-align user messages
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::TOP),
                                |ui| {
                                    ui.set_max_width(ui.available_width() * 0.8);
                                    ChatMessage::new(message).show(ui);
                                },
                            );
                        }
                        MessageRole::Assistant => {
                            // Left-align assistant messages
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::TOP),
                                |ui| {
                                    ui.set_max_width(ui.available_width() * 0.8);
                                    ChatMessage::new(message).show(ui);
                                },
                            );
                        }
                        MessageRole::System => {
                            // Center system messages
                            ui.with_layout(
                                egui::Layout::top_down(egui::Align::Center),
                                |ui| {
                                    ui.set_max_width(ui.available_width() * 0.9);
                                    ChatMessage::new(message).show(ui);
                                },
                            );
                        }
                    }
                });
        }
    }

    /// Show empty state when no messages exist.
    fn show_empty_state(&self, ui: &mut Ui) {
        let available = ui.available_size();

        ui.allocate_ui_at_rect(
            egui::Rect::from_center_size(
                ui.clip_rect().center(),
                egui::vec2(400.0, 200.0),
            ),
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(available.y * 0.3);

                    // Icon
                    ui.label(
                        egui::RichText::new("◆")
                            .size(48.0)
                            .color(colors::with_alpha(colors::AMBER, 128)),
                    );

                    ui.add_space(spacing::S4);

                    // Title
                    ui.label(
                        egui::RichText::new("Video Analysis")
                            .size(yama_theme::font_size::H2)
                            .color(colors::CHALK)
                            .strong(),
                    );

                    ui.add_space(spacing::S2);

                    // Instructions
                    ui.label(
                        egui::RichText::new("Drop a video file here or click the attach button")
                            .size(yama_theme::font_size::BODY)
                            .color(colors::SILVER),
                    );

                    ui.add_space(spacing::S1);

                    ui.label(
                        egui::RichText::new("Then describe what you want to analyze")
                            .size(yama_theme::font_size::BODY)
                            .color(colors::ASH),
                    );
                });
            },
        );
    }
}
