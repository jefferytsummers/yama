//! Application - main eframe::App implementation.

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use tracing::{error, info};
use yama_theme::{colors, font_size, radius, spacing};

use yama_host_apple::inference::{JobStatus, UploadInfo, VlmInferenceService};
use crate::ui::analyst::{
    AnalystState, Dashboard, DashboardAction, DashboardState,
    IndexingProgress, IndexingStage,
    LibraryAction, LibraryModal, LibraryModalAction, LibraryView,
    Project, ProjectManager,
    QueryAction, QueryPanel, QueryPanelState,
    SearchAction, SearchBar, SearchResults,
    ConfigPanel, ConfigPanelAction, ConfigPanelState, ProjectConfig,
};
use crate::ui::chat::{ChatInput, ChatInputAction, ConversationThread};
use crate::ui::{
    AttachmentStatus, ChatMessageData, Conversation, FrameResult, InferenceProgress,
    VideoAttachmentData,
};

/// Application view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Chat-based VLM interaction
    #[default]
    Chat,
    /// Content Analyst library view
    Analyst,
}

/// Shared application state.
pub struct AppState {
    /// VLM inference service
    pub inference_service: Option<Arc<VlmInferenceService>>,
}

/// Main Yama application.
pub struct YamaApp {
    /// Application state
    state: Arc<AppState>,
    /// Tokio runtime for async operations
    runtime: Arc<tokio::runtime::Runtime>,
    /// Current view mode
    view_mode: ViewMode,
    /// Chat conversation state
    conversation: Conversation,
    /// Project manager (for Analyst mode)
    project_manager: ProjectManager,
    /// Dashboard state
    dashboard_state: DashboardState,
    /// Query panel state (per-project, but shared UI state)
    query_state: QueryPanelState,
    /// Selected model index
    selected_model_index: usize,
    /// Width of the library panel (for resizable split)
    library_panel_width: f32,
    /// Whether library modal is open
    library_modal_open: bool,
    /// Library modal search filter
    library_modal_filter: String,
    /// Library modal selection
    library_modal_selection: std::collections::HashSet<String>,
    /// Current indexing progress (for active project)
    indexing_progress: Option<IndexingProgress>,
    /// Whether to show model settings panel
    show_settings: bool,
    /// Configuration panel state
    config_panel_state: ConfigPanelState,
    /// Project configuration (tools, models, workflows)
    project_config: ProjectConfig,
}

/// Legacy alias for backwards compatibility.
pub type ChatApp = YamaApp;

impl YamaApp {
    /// Create a new Yama application.
    pub fn new(state: Arc<AppState>, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        let mut conversation = Conversation::new();

        // Add welcome message
        conversation.add_message(ChatMessageData::system(
            "Drop a video file and describe what you want to analyze.".to_string(),
        ));

        Self {
            state,
            runtime,
            view_mode: ViewMode::Analyst, // Start in Analyst mode for Phase 0 UX testing
            conversation,
            project_manager: ProjectManager::new_with_mocks(),
            dashboard_state: DashboardState::default(),
            query_state: QueryPanelState::default(),
            selected_model_index: 0,
            library_panel_width: 450.0,
            library_modal_open: false,
            library_modal_filter: String::new(),
            library_modal_selection: std::collections::HashSet::new(),
            indexing_progress: None,
            show_settings: false,
            config_panel_state: ConfigPanelState::default(),
            project_config: ProjectConfig::default(),
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

    /// Open file picker for analyst mode.
    fn open_analyst_file_picker(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Video", &["mp4", "webm", "mov", "avi", "mkv", "m4v"])
            .pick_files()
        {
            self.import_videos_to_current_project(paths);
        }
    }

    /// Update indexing progress (mock simulation).
    fn update_indexing_progress(&mut self) {
        if let Some(progress) = &mut self.indexing_progress {
            if progress.paused || progress.stage == IndexingStage::Complete {
                return;
            }

            // Simulate progress
            if progress.processed_videos < progress.total_videos {
                progress.processed_videos += 1;
                progress.eta_seconds = Some(
                    ((progress.total_videos - progress.processed_videos) as u64) * 2,
                );

                // Get current file name from project
                if let Some(project) = self.project_manager.current_project() {
                    if let Some(video) = project.videos.get(progress.processed_videos as usize - 1) {
                        progress.current_file = Some(video.filename.clone());
                    }
                }
            }

            // Advance stages
            let stage_progress = progress.processed_videos as f32 / progress.total_videos as f32;
            progress.stage = match stage_progress {
                p if p < 0.25 => IndexingStage::Scanning,
                p if p < 0.50 => IndexingStage::ExtractingKeyframes,
                p if p < 0.75 => IndexingStage::GeneratingEmbeddings,
                p if p < 1.0 => IndexingStage::Transcribing,
                _ => IndexingStage::Complete,
            };

            if progress.stage == IndexingStage::Complete {
                // Mark all videos as indexed in current project
                if let Some(project) = self.project_manager.current_project_mut() {
                    for video in &mut project.videos {
                        video.indexed = true;
                        video.has_transcript = true;
                        video.keyframe_count = (video.duration_ms / 5000) as u32;
                    }
                }
                self.indexing_progress = None;
            }
        }
    }
}

impl eframe::App for YamaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for updates based on mode
        match self.view_mode {
            ViewMode::Chat => {
                if self.conversation.has_active_inference() {
                    self.poll_jobs();
                    ctx.request_repaint();
                }
            }
            ViewMode::Analyst => {
                if self.indexing_progress.is_some() {
                    // Request repaint for progress animation
                    ctx.request_repaint_after(std::time::Duration::from_millis(500));
                    self.update_indexing_progress();
                }
            }
        }

        // Top bar (shared between modes)
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

                    // Mode indicator
                    let mode_label = match self.view_mode {
                        ViewMode::Chat => "Chat",
                        ViewMode::Analyst => "Content Analyst",
                    };
                    ui.label(
                        egui::RichText::new(mode_label)
                            .color(colors::VIOLET)
                            .size(font_size::BODY),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Settings button - different behavior based on mode
                        let settings_active = match self.view_mode {
                            ViewMode::Chat => self.show_settings,
                            ViewMode::Analyst => self.config_panel_state.open,
                        };
                        let settings_color = if settings_active { colors::AMBER } else { colors::SILVER };

                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("⚙").color(settings_color))
                                    .fill(colors::GRAPHITE)
                                    .rounding(radius::SM),
                            )
                            .on_hover_text(if self.view_mode == ViewMode::Analyst && !self.project_manager.is_dashboard() {
                                "Project Settings"
                            } else {
                                "Settings"
                            })
                            .clicked()
                        {
                            match self.view_mode {
                                ViewMode::Chat => {
                                    self.show_settings = !self.show_settings;
                                }
                                ViewMode::Analyst => {
                                    // Only toggle config panel when in a project
                                    if !self.project_manager.is_dashboard() {
                                        self.config_panel_state.toggle();
                                    } else {
                                        self.show_settings = !self.show_settings;
                                    }
                                }
                            }
                        }

                        ui.add_space(spacing::S2);

                        // Mode toggle
                        let toggle_label = match self.view_mode {
                            ViewMode::Chat => "Library",
                            ViewMode::Analyst => "Chat",
                        };
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(toggle_label).color(colors::CHALK))
                                    .fill(colors::GRAPHITE)
                                    .rounding(radius::SM),
                            )
                            .clicked()
                        {
                            self.view_mode = match self.view_mode {
                                ViewMode::Chat => ViewMode::Analyst,
                                ViewMode::Analyst => ViewMode::Chat,
                            };
                        }

                        // Mode-specific status
                        match self.view_mode {
                            ViewMode::Chat => {
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
                            }
                            ViewMode::Analyst => {
                                // Show current project name if in a project
                                if let Some(project) = self.project_manager.current_project() {
                                    ui.label(
                                        egui::RichText::new(&project.name)
                                            .color(colors::SILVER)
                                            .size(font_size::BODY),
                                    );
                                }

                                // Indexing status
                                if let Some(progress) = &self.indexing_progress {
                                    if progress.stage != IndexingStage::Complete {
                                        ui.add_space(spacing::S2);
                                        let pulse = yama_theme::animation::pulse_alpha(ctx);
                                        let pulse_color = colors::with_alpha(colors::AMBER, (pulse * 255.0) as u8);
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "● Indexing {}/{}",
                                                progress.processed_videos, progress.total_videos
                                            ))
                                            .color(pulse_color)
                                            .strong(),
                                        );
                                    }
                                }
                            }
                        }
                    });
                });
            });

        // Bottom status bar (shared)
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
                        // Project stats in Analyst mode
                        if self.view_mode == ViewMode::Analyst {
                            if let Some(project) = self.project_manager.current_project() {
                                let indexed = project.indexed_count();
                                let total = project.video_count();
                                ui.label(
                                    egui::RichText::new(format!("{}/{} indexed", indexed, total))
                                        .color(colors::SILVER)
                                        .size(font_size::SMALL),
                                );
                            } else {
                                let total_projects = self.project_manager.projects.len();
                                ui.label(
                                    egui::RichText::new(format!("{} projects", total_projects))
                                        .color(colors::SILVER)
                                        .size(font_size::SMALL),
                                );
                            }
                        }

                        // Backend indicator
                        if let Some(service) = &self.state.inference_service {
                            ui.add_space(spacing::S4);

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

        // Mode-specific content
        match self.view_mode {
            ViewMode::Chat => self.show_chat_mode(ctx),
            ViewMode::Analyst => self.show_analyst_mode(ctx),
        }
    }
}

impl YamaApp {
    /// Show the chat mode UI.
    fn show_chat_mode(&mut self, ctx: &egui::Context) {
        // Check for dropped files
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

    /// Show the Content Analyst mode UI.
    fn show_analyst_mode(&mut self, ctx: &egui::Context) {
        // Check if we're in a project or on the dashboard
        if self.project_manager.is_dashboard() {
            self.show_dashboard(ctx);
        } else {
            self.show_project_view(ctx);
        }
    }

    /// Show the project dashboard.
    fn show_dashboard(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(colors::OBSIDIAN)
                    .inner_margin(egui::Margin::symmetric(spacing::S6, spacing::S4)),
            )
            .show(ctx, |ui| {
                let action = Dashboard::new(&self.project_manager, &mut self.dashboard_state)
                    .show(ctx, ui);

                match action {
                    DashboardAction::OpenProject(id) => {
                        self.project_manager.open_project(&id);
                        // Reset query state for new project
                        self.query_state = QueryPanelState::default();
                    }
                    DashboardAction::CreateProject => {
                        self.dashboard_state.new_project_dialog = true;
                    }
                    DashboardAction::CreateProjectNamed(name) => {
                        let id = self.project_manager.create_project(&name);
                        self.project_manager.open_project(&id);
                        self.query_state = QueryPanelState::default();
                    }
                    DashboardAction::DeleteProject(id) => {
                        self.project_manager.delete_project(&id);
                    }
                    DashboardAction::ImportToNewProject(paths) => {
                        // Create new project from dropped files
                        let name = if paths.len() == 1 {
                            paths[0]
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "New Project".to_string())
                        } else {
                            format!("Import ({} videos)", paths.len())
                        };
                        let id = self.project_manager.create_project(&name);
                        self.project_manager.open_project(&id);
                        self.import_videos_to_current_project(paths);
                        self.query_state = QueryPanelState::default();
                    }
                    DashboardAction::None => {}
                }
            });
    }

    /// Show a project view (query panel + library modal + config panel).
    fn show_project_view(&mut self, ctx: &egui::Context) {
        // Check for dropped files
        let dropped_files: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        if !dropped_files.is_empty() {
            self.import_videos_to_current_project(dropped_files);
        }

        // Show library modal if open
        if self.library_modal_open {
            self.show_library_modal(ctx);
        }

        // Show config panel if open (rendered before central panel for proper layering)
        if self.config_panel_state.open || self.config_panel_state.animation_progress > 0.01 {
            // Create a temporary mutable UI to show the config panel
            // We need to show this as a side panel, which requires egui::Context
            let panel_action = {
                let mut dummy_ui_rect = egui::Rect::NOTHING;
                egui::Area::new(egui::Id::new("config_panel_area"))
                    .fixed_pos(egui::pos2(0.0, 0.0))
                    .show(ctx, |ui| {
                        dummy_ui_rect = ui.available_rect_before_wrap();
                        ConfigPanel::new(&mut self.config_panel_state, &mut self.project_config)
                            .show(ctx, ui)
                    })
                    .inner
            };

            match panel_action {
                ConfigPanelAction::Close => {
                    self.config_panel_state.open = false;
                }
                ConfigPanelAction::ToolsChanged => {
                    info!("Tools configuration changed");
                }
                ConfigPanelAction::ModelsChanged => {
                    info!("Models configuration changed");
                }
                ConfigPanelAction::WorkflowsChanged => {
                    info!("Workflows configuration changed");
                }
                ConfigPanelAction::None => {}
            }
        }

        // Main project view
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(colors::BASALT)
                    .inner_margin(egui::Margin::symmetric(spacing::S6, spacing::S4)),
            )
            .show(ctx, |ui| {
                // Top action bar
                ui.horizontal(|ui| {
                    // Back to dashboard
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("← Projects").color(colors::CHALK))
                                .fill(colors::GRAPHITE)
                                .rounding(radius::MD),
                        )
                        .clicked()
                    {
                        self.project_manager.close_project();
                        return;
                    }

                    ui.add_space(spacing::S3);

                    // Project name
                    if let Some(project) = self.project_manager.current_project() {
                        ui.heading(
                            egui::RichText::new(&project.name)
                                .color(colors::CHALK)
                                .size(font_size::H3),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Import button (opens modal)
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("📁 Import").color(colors::OBSIDIAN),
                                )
                                .fill(colors::AMBER)
                                .rounding(radius::MD),
                            )
                            .on_hover_text("Import videos into this project")
                            .clicked()
                        {
                            self.library_modal_open = true;
                        }

                        ui.add_space(spacing::S2);

                        // Quick stats
                        if let Some(project) = self.project_manager.current_project() {
                            let total = project.video_count();
                            let indexed = project.indexed_count();

                            if total > 0 {
                                ui.label(
                                    egui::RichText::new(format!("{} indexed", indexed))
                                        .color(colors::JADE)
                                        .size(font_size::BODY),
                                );

                                ui.label(egui::RichText::new("•").color(colors::STONE));

                                ui.label(
                                    egui::RichText::new(format!("{} videos", total))
                                        .color(colors::SILVER)
                                        .size(font_size::BODY),
                                );
                            }
                        }

                        // Indexing progress indicator
                        if let Some(progress) = &self.indexing_progress {
                            if progress.stage != IndexingStage::Complete {
                                ui.add_space(spacing::S3);

                                let pulse = yama_theme::animation::pulse_alpha(ctx);
                                let progress_color = colors::with_alpha(colors::AMBER, (pulse * 255.0) as u8);

                                ui.label(
                                    egui::RichText::new(format!(
                                        "● {} {}/{}",
                                        progress.stage.label(),
                                        progress.processed_videos,
                                        progress.total_videos
                                    ))
                                    .color(progress_color)
                                    .size(font_size::SMALL),
                                );
                            }
                        }
                    });
                });

                ui.add_space(spacing::S3);
                ui.separator();
                ui.add_space(spacing::S3);

                // Query panel (full width) - use project's video count for context
                if let Some(project) = self.project_manager.current_project() {
                    // Create a temporary AnalystState from the project for the query panel
                    let mut temp_state = AnalystState::new();
                    temp_state.videos = project.videos.clone();

                    let action = QueryPanel::new(&mut self.query_state, &temp_state).show(ui);
                    self.handle_query_action(action);
                }
            });
    }

    /// Show the library modal for the current project.
    fn show_library_modal(&mut self, ctx: &egui::Context) {
        // We need to work with the current project's videos
        if let Some(project) = self.project_manager.current_project_mut() {
            // Create a temporary AnalystState from the project
            let mut temp_state = AnalystState::new();
            temp_state.videos = std::mem::take(&mut project.videos);

            let action = LibraryModal::new(
                &mut temp_state,
                &mut self.library_modal_filter,
                &mut self.library_modal_selection,
            )
            .show(ctx);

            // Put videos back
            if let Some(project) = self.project_manager.current_project_mut() {
                project.videos = temp_state.videos;
            }

            match action {
                LibraryModalAction::Close => {
                    self.library_modal_open = false;
                }
                LibraryModalAction::BrowseFiles => {
                    self.open_analyst_file_picker();
                }
                LibraryModalAction::SelectVideo(id) => {
                    if let Some(project) = self.project_manager.current_project() {
                        if let Some(video) = project.get_video(&id) {
                            self.query_state.add_video_reference(&video.filename);
                        }
                    }
                    self.library_modal_open = false;
                    self.library_modal_selection.clear();
                }
                LibraryModalAction::SelectVideos(ids) => {
                    if let Some(project) = self.project_manager.current_project() {
                        for id in &ids {
                            if let Some(video) = project.get_video(id) {
                                self.query_state.add_video_reference(&video.filename);
                            }
                        }
                    }
                    self.library_modal_open = false;
                    self.library_modal_selection.clear();
                }
                LibraryModalAction::ImportFiles(paths) => {
                    self.import_videos_to_current_project(paths);
                }
                LibraryModalAction::None => {}
            }
        }
    }

    /// Import videos into the current project.
    fn import_videos_to_current_project(&mut self, paths: Vec<PathBuf>) {
        if let Some(project) = self.project_manager.current_project_mut() {
            let video_count = paths.len() as u32;
            project.import_videos(paths);

            // Start mock indexing
            if video_count > 0 {
                self.indexing_progress = Some(IndexingProgress::new(video_count));
            }
        }
    }

    /// Handle query panel actions.
    fn handle_query_action(&mut self, action: QueryAction) {
        match action {
            QueryAction::SubmitQuery(query) => {
                info!("Query submitted: {}", query);

                // Clear input
                self.query_state.query_input.clear();

                // Add exchange
                self.query_state.add_exchange(query.clone());

                // Mock: simulate search results
                // In a real implementation, this would search the project's indexed videos
                self.query_state.mock_complete_query(Vec::new());
            }
            QueryAction::PreviewResult { video_id, timestamp_ms } => {
                info!("Preview: {} @ {}ms", video_id, timestamp_ms);
                // TODO: Open video preview
            }
            QueryAction::ExtractClip { video_id, start_ms, end_ms } => {
                info!("Extract clip: {} from {}ms to {}ms", video_id, start_ms, end_ms);
                // TODO: Actually extract clip
            }
            QueryAction::AnalyzeFrame { video_id, timestamp_ms } => {
                info!("Analyze frame: {} @ {}ms", video_id, timestamp_ms);
                // TODO: Run VLM analysis on frame
            }
            QueryAction::GenerateSummary => {
                info!("Generate summary");
                // TODO: Generate summary from results
            }
            QueryAction::None => {}
        }
    }
}
