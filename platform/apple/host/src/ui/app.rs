//! Chat application - main eframe::App implementation.

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use tracing::{error, info};
use yama_theme::{colors, font_size, radius, spacing};

use yama_host_apple::inference::{JobStatus, UploadInfo, VlmInferenceService};
use crate::ui::chat::{ChatInput, ChatInputAction, ConversationThread};
use crate::ui::{
    AttachmentStatus, ChatMessageData, Conversation, FrameResult, InferenceProgress,
    VideoAttachmentData,
};

/// Shared application state accessible to the ChatApp.
pub struct AppState {
    /// VLM inference service
    pub inference_service: Option<Arc<VlmInferenceService>>,
}

/// Chat-centric application for video analysis.
pub struct ChatApp {
    /// Application state
    state: Arc<AppState>,
    /// Tokio runtime for async operations
    runtime: Arc<tokio::runtime::Runtime>,
    /// Current conversation
    conversation: Conversation,
    /// Selected model index
    selected_model_index: usize,
}

impl ChatApp {
    /// Create a new chat application.
    pub fn new(state: Arc<AppState>, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        let mut conversation = Conversation::new();

        // Add welcome message
        conversation.add_message(ChatMessageData::system(
            "Drop a video file and describe what you want to analyze.".to_string(),
        ));

        Self {
            state,
            runtime,
            conversation,
            selected_model_index: 0,
        }
    }

    /// Poll for job status updates.
    fn poll_jobs(&mut self) {
        let Some(service) = &self.state.inference_service else {
            return;
        };

        // Find all messages with active jobs
        let job_ids: Vec<String> = self
            .conversation
            .messages
            .iter()
            .filter_map(|m| {
                if m.is_inferring() {
                    m.job_id.clone()
                } else {
                    None
                }
            })
            .collect();

        for job_id in job_ids {
            let service = service.clone();
            let job_id_clone = job_id.clone();

            // Poll job status
            if let Some(job) = self.runtime.block_on(async { service.get_job(&job_id_clone).await })
            {
                // Update the message
                if let Some(message) = self.conversation.get_message_by_job_mut(&job_id) {
                    // Update progress
                    if job.status == JobStatus::Completed || job.status == JobStatus::Failed {
                        message.progress = None;
                    } else {
                        message.progress = Some(InferenceProgress::new(
                            job.current_frame,
                            job.total_frames,
                        ));
                    }

                    // Update results
                    message.frame_results = job
                        .results
                        .iter()
                        .map(|c| FrameResult {
                            frame_number: c.frame_number,
                            timestamp_ms: c.timestamp_ms,
                            text: c.text.clone(),
                        })
                        .collect();

                    // Update error
                    if job.status == JobStatus::Failed {
                        message.error = job.error.clone();
                    }

                    // Update attachment statuses
                    for attachment in &mut message.attachments {
                        attachment.status = match job.status {
                            JobStatus::Completed => AttachmentStatus::Completed,
                            JobStatus::Failed => AttachmentStatus::Failed,
                            _ => AttachmentStatus::Processing,
                        };
                    }
                }
            }
        }
    }

    /// Handle sending a message.
    fn send_message(&mut self) {
        // Check if inference service is available
        let service = match &self.state.inference_service {
            Some(s) => s.clone(),
            None => {
                self.conversation.add_message(ChatMessageData::system(
                    "Inference service not available".to_string(),
                ));
                return;
            }
        };

        // Get draft content
        let text = self.conversation.draft_text.clone();
        let attachments = std::mem::take(&mut self.conversation.draft_attachments);

        // Validate
        if text.trim().is_empty() && attachments.is_empty() {
            return;
        }

        // Create user message
        let user_message = ChatMessageData::user(text.clone(), attachments.clone());
        self.conversation.add_message(user_message);

        // Clear draft
        self.conversation.clear_draft();

        // If we have attachments, start inference
        if !attachments.is_empty() {
            for attachment in &attachments {
                self.start_inference(&service, attachment, &text);
            }
        } else {
            // No video attached - just a text message
            self.conversation.add_message(ChatMessageData::system(
                "Please attach a video file to analyze.".to_string(),
            ));
        }
    }

    /// Start inference for a video attachment.
    fn start_inference(
        &mut self,
        service: &Arc<VlmInferenceService>,
        attachment: &VideoAttachmentData,
        prompt: &str,
    ) {
        let service = service.clone();
        let video_path = attachment.path.clone();
        let prompt = prompt.to_string();

        // Get selected model
        let models = service.list_models();
        let model_id = if self.selected_model_index < models.len() {
            models[self.selected_model_index].id.clone()
        } else {
            "vlm-default".to_string()
        };

        // Create job
        let job_id = self.runtime.block_on(async {
            // Register upload
            let upload_id = uuid::Uuid::new_v4().to_string();
            let upload_info = UploadInfo {
                upload_id: upload_id.clone(),
                filename: video_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "video.mp4".to_string()),
                size: std::fs::metadata(&video_path).map(|m| m.len()).unwrap_or(0),
                path: video_path,
                content_type: "video/mp4".to_string(),
            };
            service.register_upload(upload_info).await;

            // Create and start job
            let job_id = service.create_job(model_id, prompt).await;
            if let Err(e) = service.start_inference(job_id.clone(), upload_id).await {
                error!("Failed to start inference: {}", e);
            }

            job_id
        });

        // Add assistant message (pending)
        self.conversation
            .add_message(ChatMessageData::assistant_pending(job_id));
    }

    /// Open file picker and add selected files.
    fn open_file_picker(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Video", &["mp4", "webm", "mov", "avi", "mkv", "m4v"])
            .pick_files()
        {
            for path in paths {
                let attachment = VideoAttachmentData::new(path);
                if attachment.is_video() {
                    self.conversation.add_draft_attachment(attachment);
                }
            }
        }
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for job updates if we have active inference
        if self.conversation.has_active_inference() {
            self.poll_jobs();
            ctx.request_repaint();
        }

        // Check for dropped files globally
        let dropped_files: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        for path in dropped_files {
            let attachment = VideoAttachmentData::new(path);
            if attachment.is_video() {
                self.conversation.add_draft_attachment(attachment);
            }
        }

        // Top bar
        egui::TopBottomPanel::top("top_bar")
            .frame(
                egui::Frame::none()
                    .fill(colors::BASALT)
                    .inner_margin(egui::Margin::symmetric(spacing::S6, spacing::S3)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Logo
                    ui.label(egui::RichText::new("◆").size(24.0).color(colors::AMBER));
                    ui.add_space(spacing::S2);
                    ui.heading(
                        egui::RichText::new("Yama")
                            .color(colors::CHALK)
                            .strong()
                            .size(font_size::H2),
                    );
                    ui.label(
                        egui::RichText::new("Video Analysis")
                            .color(colors::VIOLET)
                            .size(font_size::BODY),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Status indicator
                        if self.conversation.has_active_inference() {
                            let pulse = yama_theme::animation::pulse_alpha(ctx);
                            let pulse_color = colors::with_alpha(colors::VIOLET, (pulse * 255.0) as u8);
                            ui.label(
                                egui::RichText::new("● Analyzing...")
                                    .color(pulse_color)
                                    .strong(),
                            );
                        }

                        // Model selector
                        if let Some(service) = &self.state.inference_service {
                            let models: Vec<String> = service
                                .list_models()
                                .iter()
                                .map(|m| m.name.clone())
                                .collect();

                            let selected_text = models
                                .get(self.selected_model_index)
                                .cloned()
                                .unwrap_or_else(|| "Select model".to_string());

                            ui.add_space(spacing::S4);

                            egui::ComboBox::from_id_salt("model_selector")
                                .selected_text(&selected_text)
                                .show_ui(ui, |ui| {
                                    for (i, model) in models.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut self.selected_model_index,
                                            i,
                                            model,
                                        );
                                    }
                                });
                        }
                    });
                });
            });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::none()
                    .fill(colors::BASALT)
                    .inner_margin(egui::Margin::symmetric(spacing::S6, spacing::S2)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Ready indicator
                    let (dot_rect, _) =
                        ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(dot_rect.center(), 4.0, colors::JADE);

                    ui.label(
                        egui::RichText::new("Ready")
                            .color(colors::TEXT_MUTED)
                            .size(font_size::SMALL),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Backend indicator
                        if let Some(service) = &self.state.inference_service {
                            let (backend_text, backend_color) = match service.backend_name() {
                                "mock" => ("Mock Backend", colors::AZURE),
                                "event_bus" => ("VLM Container", colors::VIOLET),
                                _ => ("Unknown", colors::SILVER),
                            };

                            ui.label(
                                egui::RichText::new(backend_text)
                                    .color(backend_color)
                                    .size(font_size::SMALL),
                            );

                            // Readiness check for event_bus
                            if service.backend_name() == "event_bus" {
                                let is_ready =
                                    self.runtime.block_on(async { service.is_ready().await });

                                ui.add_space(spacing::S3);

                                let (ready_dot, _) =
                                    ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                                let (ready_text, ready_color) = if is_ready {
                                    ("VLM Ready", colors::JADE)
                                } else {
                                    ("VLM Not Connected", colors::EMBER)
                                };

                                ui.painter()
                                    .circle_filled(ready_dot.center(), 4.0, ready_color);
                                ui.label(
                                    egui::RichText::new(ready_text)
                                        .color(ready_color)
                                        .size(font_size::SMALL),
                                );
                            }
                        }
                    });
                });
            });

        // Input panel at bottom
        egui::TopBottomPanel::bottom("input_panel")
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(colors::OBSIDIAN)
                    .inner_margin(egui::Margin::symmetric(spacing::S6, spacing::S4)),
            )
            .show(ctx, |ui| {
                let is_inferring = self.conversation.has_active_inference();
                let action = ChatInput::new(&mut self.conversation)
                    .disabled(is_inferring)
                    .show(ui);

                match action {
                    ChatInputAction::Send => {
                        self.send_message();
                    }
                    ChatInputAction::RemoveAttachment(id) => {
                        self.conversation.remove_draft_attachment(&id);
                    }
                    ChatInputAction::OpenFilePicker => {
                        self.open_file_picker();
                    }
                    ChatInputAction::None => {}
                }
            });

        // Main chat area
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(colors::OBSIDIAN)
                    .inner_margin(egui::Margin::same(0.0)),
            )
            .show(ctx, |ui| {
                ConversationThread::new(&self.conversation).show(ui);
            });
    }
}
