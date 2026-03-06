//! Yama Compositor - VLM Video Inference MVP
//!
//! Native egui application for video analysis using Vision Language Models.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use eframe::egui;
use tokio::sync::RwLock;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

// Import theme
use yama_theme::{colors, radius, spacing, YamaTheme, RichTextExt};

mod compositor;
mod event_bus;
mod http_server;
mod orchestrator;

use event_bus::EventBus;
use http_server::HttpServer;
use orchestrator::Orchestrator;

// Use inference module from the library crate
use yama_host_apple::inference::{self, InferenceChunk, JobStatus, VlmInferenceConfig, VlmInferenceService};

/// Application configuration.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub compositor: compositor::CompositorConfig,
    pub event_bus: event_bus::EventBusConfig,
    pub orchestrator: orchestrator::OrchestratorConfig,
    #[serde(default)]
    pub http_server: http_server::HttpServerConfig,
    #[serde(default)]
    pub inference: VlmInferenceConfig,
}

impl Config {
    /// Load configuration from file.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {:?}", path))?;
        toml::from_str(&content).context("Failed to parse config")
    }

    /// Load configuration from default locations.
    pub fn load_default() -> Result<Self> {
        let paths = [
            PathBuf::from("yama.toml"),
            PathBuf::from("/etc/yama/yama.toml"),
            dirs::config_dir()
                .map(|p| p.join("yama/yama.toml"))
                .unwrap_or_default(),
        ];

        for path in &paths {
            if path.exists() {
                return Self::load(path);
            }
        }

        Ok(Self::default())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            compositor: compositor::CompositorConfig::default(),
            event_bus: event_bus::EventBusConfig::default(),
            orchestrator: orchestrator::OrchestratorConfig::default(),
            http_server: http_server::HttpServerConfig::default(),
            inference: VlmInferenceConfig::default(),
        }
    }
}

/// Shared application state.
pub struct AppState {
    pub config: Config,
    pub event_bus: Arc<EventBus>,
    pub orchestrator: Arc<RwLock<Orchestrator>>,
    pub inference_service: Option<Arc<VlmInferenceService>>,
}

/// VLM Inference application.
struct YamaApp {
    /// Shared state with background services.
    state: Arc<AppState>,
    /// Tokio runtime for async operations.
    runtime: Arc<tokio::runtime::Runtime>,

    // Video selection
    selected_video: Option<PathBuf>,
    video_filename: String,

    // Model selection
    selected_model_index: usize,

    // Prompt input
    prompt: String,

    // Inference state
    is_running: bool,
    current_job_id: Option<String>,
    job_status: Option<JobStatus>,
    progress_percent: f32,
    current_frame: u64,
    total_frames: u64,
    results: Vec<InferenceChunk>,
    error_message: Option<String>,
}

impl YamaApp {
    fn new(state: Arc<AppState>, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        Self {
            state,
            runtime,
            selected_video: None,
            video_filename: String::new(),
            selected_model_index: 0,
            prompt: String::new(),
            is_running: false,
            current_job_id: None,
            job_status: None,
            progress_percent: 0.0,
            current_frame: 0,
            total_frames: 0,
            results: Vec::new(),
            error_message: None,
        }
    }

    fn format_timestamp(ms: u64) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        let millis = ms % 1000;
        format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
    }

    fn start_inference(&mut self) {
        let Some(video_path) = &self.selected_video else {
            self.error_message = Some("No video selected".to_string());
            return;
        };

        let Some(inference_service) = &self.state.inference_service else {
            self.error_message = Some("Inference service not available".to_string());
            return;
        };

        if self.prompt.trim().is_empty() {
            self.error_message = Some("Please enter a prompt".to_string());
            return;
        }

        // Get selected model
        let models = inference_service.list_models();
        let model_id = if self.selected_model_index < models.len() {
            models[self.selected_model_index].id.clone()
        } else {
            "vlm-default".to_string()
        };

        // Clear previous results
        self.results.clear();
        self.error_message = None;
        self.is_running = true;
        self.progress_percent = 0.0;
        self.current_frame = 0;
        self.total_frames = 0;
        self.job_status = Some(JobStatus::Queued);

        // Clone what we need for the async task
        let service = inference_service.clone();
        let prompt = self.prompt.clone();
        let video_path = video_path.clone();

        // Start inference in background
        let job_id = self.runtime.block_on(async {
            // Register the video as an "upload" (using the file path directly)
            let upload_id = uuid::Uuid::new_v4().to_string();
            let upload_info = inference::UploadInfo {
                upload_id: upload_id.clone(),
                filename: video_path.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "video.mp4".to_string()),
                size: std::fs::metadata(&video_path).map(|m| m.len()).unwrap_or(0),
                path: video_path,
                content_type: "video/mp4".to_string(),
            };
            service.register_upload(upload_info).await;

            // Create job
            let job_id = service.create_job(model_id, prompt).await;

            // Start inference
            if let Err(e) = service.start_inference(job_id.clone(), upload_id).await {
                error!("Failed to start inference: {}", e);
            }

            job_id
        });

        self.current_job_id = Some(job_id);
    }

    fn poll_job_status(&mut self) {
        let Some(job_id) = &self.current_job_id else {
            return;
        };

        let Some(inference_service) = &self.state.inference_service else {
            return;
        };

        let service = inference_service.clone();
        let job_id = job_id.clone();

        if let Some(job) = self.runtime.block_on(async {
            service.get_job(&job_id).await
        }) {
            self.job_status = Some(job.status);
            self.progress_percent = job.progress_percent;
            self.current_frame = job.current_frame;
            self.total_frames = job.total_frames;
            self.results = job.results;

            if let Some(err) = job.error {
                self.error_message = Some(err);
            }

            if job.status == JobStatus::Completed || job.status == JobStatus::Failed {
                self.is_running = false;
            }
        }
    }

    fn reset(&mut self) {
        self.selected_video = None;
        self.video_filename.clear();
        self.prompt.clear();
        self.is_running = false;
        self.current_job_id = None;
        self.job_status = None;
        self.progress_percent = 0.0;
        self.current_frame = 0;
        self.total_frames = 0;
        self.results.clear();
        self.error_message = None;
    }
}

impl eframe::App for YamaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll job status if running
        if self.is_running {
            self.poll_job_status();
            ctx.request_repaint();
        }

        // Pulsing animation for status indicators
        let pulse = yama_theme::animation::pulse_alpha(ctx);

        // Top bar with gradient accent
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::none()
                .fill(colors::BASALT)
                .inner_margin(egui::Margin::symmetric(spacing::S6, spacing::S3)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Logo/brand with amber accent
                    ui.label(egui::RichText::new("◆")
                        .size(24.0)
                        .color(colors::AMBER));
                    ui.add_space(spacing::S2);
                    ui.heading(egui::RichText::new("Yama")
                        .color(colors::CHALK)
                        .strong()
                        .size(yama_theme::font_size::H2));
                    ui.label(egui::RichText::new("Video Analysis")
                        .color(colors::VIOLET)
                        .size(yama_theme::font_size::BODY));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.is_running {
                            // Animated status indicator
                            let pulse_color = colors::with_alpha(colors::AMBER, (pulse * 255.0) as u8);
                            ui.label(egui::RichText::new("● Processing...")
                                .color(pulse_color)
                                .strong());
                        } else if self.job_status == Some(JobStatus::Completed) {
                            ui.label(egui::RichText::new("✓ Complete")
                                .color(colors::JADE)
                                .strong());
                        }
                    });
                });
            });

        // Bottom status bar with accent line
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none()
                .fill(colors::BASALT)
                .inner_margin(egui::Margin::symmetric(spacing::S6, spacing::S2)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Accent indicator
                    let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 4.0, colors::JADE);

                    ui.label(egui::RichText::new("Ready")
                        .color(colors::TEXT_MUTED)
                        .size(yama_theme::font_size::SMALL));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("Mock Backend")
                            .color(colors::AZURE)
                            .size(yama_theme::font_size::SMALL));
                    });
                });
            });

        // Main content area with proper margins
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(colors::OBSIDIAN)
                .inner_margin(egui::Margin::same(spacing::S6)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left side - Video preview (55%)
                    let left_width = ui.available_width() * 0.55;
                    ui.allocate_ui(egui::vec2(left_width, ui.available_height()), |ui| {
                        ui.vertical(|ui| {
                            // Video preview card
                            let preview_height = (ui.available_height() - 70.0).max(200.0);
                            let preview_width = left_width - spacing::S4;

                            // Card with violet accent border when video loaded
                            let border_color = if self.selected_video.is_some() {
                                colors::VIOLET
                            } else {
                                colors::STONE
                            };

                            egui::Frame::none()
                                .fill(colors::SLATE)
                                .stroke(egui::Stroke::new(2.0, border_color))
                                .rounding(egui::Rounding::same(radius::XL))
                                .inner_margin(spacing::S1)
                                .show(ui, |ui| {
                                    let (preview_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(preview_width - spacing::S2, preview_height),
                                        egui::Sense::hover()
                                    );

                                    // Inner dark area
                                    ui.painter().rect_filled(
                                        preview_rect,
                                        radius::LG,
                                        colors::OBSIDIAN,
                                    );

                                    if self.selected_video.is_some() {
                                        // Filename badge at top
                                        let badge_rect = egui::Rect::from_min_size(
                                            preview_rect.left_top() + egui::Vec2::new(spacing::S3, spacing::S3),
                                            egui::vec2(200.0, 24.0),
                                        );
                                        ui.painter().rect_filled(
                                            badge_rect,
                                            radius::SM,
                                            colors::with_alpha(colors::VIOLET, 40),
                                        );
                                        ui.painter().text(
                                            badge_rect.left_center() + egui::Vec2::new(spacing::S2, 0.0),
                                            egui::Align2::LEFT_CENTER,
                                            &self.video_filename,
                                            egui::FontId::monospace(yama_theme::font_size::SMALL),
                                            colors::VIOLET,
                                        );

                                        // Center icon
                                        ui.painter().text(
                                            preview_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "▶",
                                            egui::FontId::proportional(48.0),
                                            colors::with_alpha(colors::AMBER, 100),
                                        );

                                        // Progress overlay if running
                                        if self.is_running || self.job_status == Some(JobStatus::Completed) {
                                            let progress_rect = egui::Rect::from_min_size(
                                                preview_rect.left_bottom() - egui::Vec2::new(0.0, 50.0),
                                                egui::vec2(preview_rect.width(), 50.0),
                                            );

                                            // Gradient overlay
                                            ui.painter().rect_filled(
                                                progress_rect,
                                                0.0,
                                                colors::with_alpha(colors::OBSIDIAN, 230),
                                            );

                                            // Progress bar track
                                            let bar_rect = egui::Rect::from_min_size(
                                                progress_rect.left_top() + egui::Vec2::new(spacing::S4, spacing::S3),
                                                egui::vec2(progress_rect.width() - spacing::S8, 8.0),
                                            );
                                            ui.painter().rect_filled(bar_rect, radius::SM, colors::GRAPHITE);

                                            // Progress bar fill with gradient effect
                                            let filled_width = bar_rect.width() * (self.progress_percent / 100.0);
                                            if filled_width > 0.0 {
                                                let filled_rect = egui::Rect::from_min_size(
                                                    bar_rect.left_top(),
                                                    egui::vec2(filled_width, bar_rect.height()),
                                                );
                                                // Amber to Ember gradient simulation
                                                let progress_color = if self.progress_percent > 50.0 {
                                                    colors::EMBER
                                                } else {
                                                    colors::AMBER
                                                };
                                                ui.painter().rect_filled(filled_rect, radius::SM, progress_color);

                                                // Glow effect
                                                ui.painter().rect_filled(
                                                    filled_rect.expand(2.0),
                                                    radius::MD,
                                                    colors::with_alpha(progress_color, 30),
                                                );
                                            }

                                            // Progress text
                                            let progress_text = format!(
                                                "{:.0}%  •  Frame {}/{}",
                                                self.progress_percent, self.current_frame, self.total_frames
                                            );
                                            ui.painter().text(
                                                progress_rect.left_bottom() - egui::Vec2::new(-spacing::S4, spacing::S2),
                                                egui::Align2::LEFT_BOTTOM,
                                                progress_text,
                                                egui::FontId::monospace(yama_theme::font_size::SMALL),
                                                colors::CHALK,
                                            );
                                        }
                                    } else {
                                        // Empty state with styled prompt
                                        ui.painter().text(
                                            preview_rect.center() - egui::Vec2::new(0.0, 20.0),
                                            egui::Align2::CENTER_CENTER,
                                            "📹",
                                            egui::FontId::proportional(48.0),
                                            colors::STONE,
                                        );
                                        ui.painter().text(
                                            preview_rect.center() + egui::Vec2::new(0.0, 30.0),
                                            egui::Align2::CENTER_CENTER,
                                            "Select a video to analyze",
                                            egui::FontId::proportional(yama_theme::font_size::H3),
                                            colors::TEXT_MUTED,
                                        );
                                    }
                                });

                            ui.add_space(spacing::S4);

                            // Action buttons with accent colors
                            ui.horizontal(|ui| {
                                // Select Video button - Azure accent
                                if ui.add_sized(
                                    [180.0, 40.0],
                                    egui::Button::new(
                                        egui::RichText::new("📁  Select Video...")
                                            .color(colors::CHALK)
                                            .size(yama_theme::font_size::BODY)
                                    )
                                    .fill(colors::AZURE)
                                    .rounding(egui::Rounding::same(radius::MD))
                                ).clicked() && !self.is_running {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Video", &["mp4", "webm", "mov", "avi"])
                                        .pick_file()
                                    {
                                        self.video_filename = path.file_name()
                                            .map(|s| s.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        self.selected_video = Some(path);
                                        self.results.clear();
                                        self.job_status = None;
                                        self.error_message = None;
                                    }
                                }

                                ui.add_space(spacing::S2);

                                if self.selected_video.is_some() && !self.is_running {
                                    if ui.add_sized(
                                        [100.0, 40.0],
                                        egui::Button::new(
                                            egui::RichText::new("Clear")
                                                .color(colors::TEXT_SECONDARY)
                                        )
                                        .fill(egui::Color32::TRANSPARENT)
                                        .stroke(egui::Stroke::new(1.0, colors::STONE))
                                        .rounding(egui::Rounding::same(radius::MD))
                                    ).clicked() {
                                        self.reset();
                                    }
                                }
                            });
                        });
                    });

                    ui.add_space(spacing::S4);

                    // Right side - Controls and results
                    ui.vertical(|ui| {
                        // Control panel with gradient header
                        egui::Frame::none()
                            .fill(colors::SLATE)
                            .stroke(egui::Stroke::new(1.0, colors::STONE))
                            .rounding(egui::Rounding::same(radius::LG))
                            .show(ui, |ui| {
                                // Header with amber accent
                                egui::Frame::none()
                                    .fill(colors::GRAPHITE)
                                    .rounding(egui::Rounding {
                                        nw: radius::LG, ne: radius::LG,
                                        sw: 0.0, se: 0.0,
                                    })
                                    .inner_margin(egui::Margin::symmetric(spacing::S4, spacing::S3))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("⚡")
                                                .size(16.0)
                                                .color(colors::AMBER));
                                            ui.label(egui::RichText::new("Inference Settings")
                                                .color(colors::CHALK)
                                                .strong()
                                                .size(yama_theme::font_size::BODY));
                                        });
                                    });

                                ui.add_space(spacing::S3);

                                // Content padding
                                egui::Frame::none()
                                    .inner_margin(egui::Margin::symmetric(spacing::S4, 0.0))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());

                                        // Model selector
                                        ui.label(egui::RichText::new("MODEL")
                                            .size(yama_theme::font_size::TINY)
                                            .color(colors::AMBER));
                                        ui.add_space(spacing::S1);

                                        let models: Vec<String> = self.state.inference_service
                                            .as_ref()
                                            .map(|s| s.list_models().iter().map(|m| m.name.clone()).collect())
                                            .unwrap_or_else(|| vec!["VLM Default".to_string()]);

                                        let selected_text = models.get(self.selected_model_index)
                                            .cloned()
                                            .unwrap_or_else(|| "Select model".to_string());

                                        ui.add_enabled_ui(!self.is_running, |ui| {
                                            egui::ComboBox::from_id_salt("model_selector")
                                                .selected_text(&selected_text)
                                                .width(ui.available_width())
                                                .show_ui(ui, |ui| {
                                                    for (i, model) in models.iter().enumerate() {
                                                        ui.selectable_value(&mut self.selected_model_index, i, model);
                                                    }
                                                });
                                        });

                                        ui.add_space(spacing::S4);

                                        // Prompt input
                                        ui.label(egui::RichText::new("INSTRUCTIONS")
                                            .size(yama_theme::font_size::TINY)
                                            .color(colors::VIOLET));
                                        ui.add_space(spacing::S1);

                                        ui.add_enabled_ui(!self.is_running, |ui| {
                                            let text_edit = egui::TextEdit::multiline(&mut self.prompt)
                                                .hint_text("What should I analyze?\n\nExamples:\n• Describe what happens\n• Count people and objects\n• Detect motion events")
                                                .desired_rows(5)
                                                .desired_width(ui.available_width())
                                                .font(egui::FontId::proportional(yama_theme::font_size::BODY));
                                            ui.add(text_edit);
                                        });

                                        ui.add_space(spacing::S4);

                                        // Run button - prominent with state feedback
                                        let can_run = self.selected_video.is_some()
                                            && !self.prompt.trim().is_empty()
                                            && !self.is_running;

                                        let (button_text, button_color) = if self.is_running {
                                            ("⟳ Running...", colors::VIOLET)
                                        } else if !self.selected_video.is_some() {
                                            ("Select a video first", colors::GRAPHITE)
                                        } else if self.prompt.trim().is_empty() {
                                            ("Enter instructions above", colors::GRAPHITE)
                                        } else {
                                            ("▶ Run Inference", colors::JADE)
                                        };

                                        if ui.add_sized(
                                            [ui.available_width(), 44.0],
                                            egui::Button::new(
                                                egui::RichText::new(button_text)
                                                    .size(yama_theme::font_size::BODY)
                                                    .strong()
                                                    .color(if can_run || self.is_running { colors::OBSIDIAN } else { colors::TEXT_MUTED })
                                            )
                                            .fill(button_color)
                                            .rounding(egui::Rounding::same(radius::MD))
                                        ).clicked() && can_run {
                                            self.start_inference();
                                        }

                                        // Error message with ember color
                                        if let Some(err) = &self.error_message {
                                            ui.add_space(spacing::S2);
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("⚠")
                                                    .color(colors::EMBER));
                                                ui.label(egui::RichText::new(err)
                                                    .color(colors::EMBER)
                                                    .size(yama_theme::font_size::SMALL));
                                            });
                                        }

                                        ui.add_space(spacing::S3);
                                    });
                            });

                        ui.add_space(spacing::S4);

                        // Results panel
                        egui::Frame::none()
                            .fill(colors::BASALT)
                            .stroke(egui::Stroke::new(1.0, colors::STONE))
                            .rounding(egui::Rounding::same(radius::LG))
                            .show(ui, |ui| {
                                let results_height = (ui.available_height() - spacing::S2).max(150.0);
                                ui.set_height(results_height);

                                // Header
                                egui::Frame::none()
                                    .inner_margin(egui::Margin::symmetric(spacing::S4, spacing::S3))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("📊")
                                                .size(14.0));
                                            ui.label(egui::RichText::new("Results")
                                                .color(colors::CHALK)
                                                .strong());

                                            if let Some(status) = &self.job_status {
                                                let (status_text, status_color, status_icon) = match status {
                                                    JobStatus::Queued => ("Queued", colors::TEXT_MUTED, "○"),
                                                    JobStatus::Extracting => ("Extracting", colors::AMBER, "◐"),
                                                    JobStatus::Inferring => ("Inferring", colors::VIOLET, "◑"),
                                                    JobStatus::Completed => ("Complete", colors::JADE, "●"),
                                                    JobStatus::Failed => ("Failed", colors::EMBER, "✕"),
                                                };
                                                ui.label(egui::RichText::new(format!("  {} {}", status_icon, status_text))
                                                    .color(status_color)
                                                    .size(yama_theme::font_size::SMALL));
                                            }
                                        });
                                    });

                                ui.separator();

                                // Results list
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        ui.add_space(spacing::S2);

                                        egui::Frame::none()
                                            .inner_margin(egui::Margin::symmetric(spacing::S4, 0.0))
                                            .show(ui, |ui| {
                                                if self.results.is_empty() {
                                                    if self.is_running {
                                                        ui.horizontal(|ui| {
                                                            ui.spinner();
                                                            ui.label(egui::RichText::new("Analyzing video...")
                                                                .color(colors::VIOLET));
                                                        });
                                                    } else if self.job_status.is_none() {
                                                        ui.label(egui::RichText::new("Results will appear here after inference")
                                                            .color(colors::TEXT_MUTED)
                                                            .italics());
                                                    }
                                                } else {
                                                    for chunk in &self.results {
                                                        // Timestamp badge
                                                        ui.horizontal(|ui| {
                                                            egui::Frame::none()
                                                                .fill(colors::with_alpha(colors::JADE, 30))
                                                                .rounding(egui::Rounding::same(radius::SM))
                                                                .inner_margin(egui::Margin::symmetric(spacing::S2, spacing::S1))
                                                                .show(ui, |ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(Self::format_timestamp(chunk.timestamp_ms))
                                                                            .color(colors::JADE)
                                                                            .monospace()
                                                                            .size(yama_theme::font_size::SMALL)
                                                                    );
                                                                });
                                                        });
                                                        ui.add_space(spacing::S1);
                                                        ui.label(egui::RichText::new(&chunk.text)
                                                            .size(yama_theme::font_size::BODY));
                                                        ui.add_space(spacing::S3);
                                                    }

                                                    if self.is_running {
                                                        ui.horizontal(|ui| {
                                                            ui.spinner();
                                                            ui.label(egui::RichText::new("...")
                                                                .color(colors::VIOLET));
                                                        });
                                                    }
                                                }
                                            });

                                        ui.add_space(spacing::S2);
                                    });
                            });
                    });
                });
            });
    }
}

fn main() -> Result<()> {
    // Initialize logging
    let _guard = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .pretty()
        .try_init();

    info!("Starting Yama compositor (Apple Silicon)");
    info!("Platform: {} {}", std::env::consts::OS, std::env::consts::ARCH);

    // Create tokio runtime for async services
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("Failed to create tokio runtime")?,
    );

    // Load configuration
    let config = Config::load_default().context("Failed to load configuration")?;
    info!("Configuration loaded");

    // Initialize services in the runtime
    let state = runtime.block_on(async {
        // Initialize event bus
        let event_bus = Arc::new(
            EventBus::new(config.event_bus.clone())
                .await
                .context("Failed to initialize event bus")?,
        );
        info!("Event bus initialized");

        // Initialize orchestrator
        let orchestrator = Arc::new(RwLock::new(
            Orchestrator::new(config.orchestrator.clone(), event_bus.clone())
                .await
                .context("Failed to initialize orchestrator")?,
        ));
        info!("Orchestrator initialized");

        // Initialize VLM inference service
        let inference_service = match VlmInferenceService::new(config.inference.clone()).await {
            Ok(service) => {
                info!("VLM inference service initialized");
                Some(Arc::new(service))
            }
            Err(e) => {
                error!("Failed to initialize VLM inference service: {}", e);
                None
            }
        };

        Ok::<_, anyhow::Error>(Arc::new(AppState {
            config,
            event_bus,
            orchestrator,
            inference_service,
        }))
    })?;

    // Start background services
    let event_bus = state.event_bus.clone();
    let orchestrator = state.orchestrator.clone();

    runtime.spawn(async move {
        if let Err(e) = event_bus.run().await {
            error!("Event bus error: {}", e);
        }
    });

    runtime.spawn(async move {
        let mut orch = orchestrator.write().await;
        if let Err(e) = orch.start().await {
            error!("Orchestrator error: {}", e);
        }
    });

    // Start HTTP server for Host UI
    let http_config = state.config.http_server.clone();
    let http_state = state.clone();
    runtime.spawn(async move {
        let server = HttpServer::new(http_config, http_state);
        if let Err(e) = server.run().await {
            error!("HTTP server error: {}", e);
        }
    });

    // Configure eframe
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Yama - Video Analysis"),
        ..Default::default()
    };

    info!("Starting GUI");

    // Run eframe (this blocks until window is closed)
    eframe::run_native(
        "Yama",
        options,
        Box::new(move |cc| {
            // Apply Yama theme
            YamaTheme::new().apply(&cc.egui_ctx);
            Ok(Box::new(YamaApp::new(state, runtime)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    info!("Shutdown complete");
    Ok(())
}

// Re-export for use in other modules
mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library/Application Support"))
        }
        #[cfg(not(target_os = "macos"))]
        {
            std::env::var("XDG_CONFIG_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
        }
    }
}
