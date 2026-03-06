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

mod compositor;
mod event_bus;
mod http_server;
mod inference;
mod orchestrator;

use event_bus::EventBus;
use http_server::HttpServer;
use inference::{InferenceChunk, JobStatus, VlmInferenceConfig, VlmInferenceService};
use orchestrator::Orchestrator;

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

        // Color palette
        let bg_dark = egui::Color32::from_rgb(10, 10, 18);
        let surface = egui::Color32::from_rgb(15, 52, 96);
        let primary = egui::Color32::from_rgb(233, 69, 96);
        let secondary = egui::Color32::from_rgb(83, 52, 131);
        let success = egui::Color32::from_rgb(0, 210, 106);
        let text_muted = egui::Color32::from_rgb(160, 160, 160);

        // Top bar
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(22, 33, 62)))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.heading(egui::RichText::new("Yama").color(egui::Color32::WHITE).strong());
                    ui.label(egui::RichText::new("Video Analysis").color(text_muted));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if self.is_running {
                            ui.label(egui::RichText::new("Processing...").color(egui::Color32::from_rgb(255, 193, 7)));
                        } else if self.job_status == Some(JobStatus::Completed) {
                            ui.label(egui::RichText::new("Complete").color(success));
                        }
                    });
                });
                ui.add_space(8.0);
            });

        // Main content area - split layout
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(bg_dark))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left side - Video preview and controls (60%)
                    let left_width = ui.available_width() * 0.6;
                    ui.allocate_ui(egui::vec2(left_width, ui.available_height()), |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(16.0);

                            // Video preview area
                            let preview_height = (ui.available_height() - 80.0).max(200.0);
                            let preview_width = (left_width - 32.0).max(100.0);
                            let preview_rect = ui.allocate_space(egui::vec2(preview_width, preview_height)).1;

                            // Draw video preview background
                            ui.painter().rect_filled(
                                preview_rect,
                                8.0,
                                egui::Color32::from_rgb(10, 10, 18),
                            );
                            ui.painter().rect_stroke(
                                preview_rect,
                                8.0,
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 70)),
                            );

                            if self.selected_video.is_some() {
                                // Show filename
                                ui.painter().text(
                                    preview_rect.left_top() + egui::Vec2::new(12.0, 12.0),
                                    egui::Align2::LEFT_TOP,
                                    &self.video_filename,
                                    egui::FontId::monospace(12.0),
                                    egui::Color32::WHITE,
                                );

                                // Progress overlay if running
                                if self.is_running || self.job_status == Some(JobStatus::Completed) {
                                    let progress_rect = egui::Rect::from_min_size(
                                        preview_rect.left_bottom() - egui::Vec2::new(0.0, 40.0),
                                        egui::vec2(preview_rect.width(), 40.0),
                                    );

                                    // Background gradient
                                    ui.painter().rect_filled(
                                        progress_rect,
                                        0.0,
                                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200),
                                    );

                                    // Progress bar
                                    let bar_rect = egui::Rect::from_min_size(
                                        progress_rect.left_top() + egui::Vec2::new(16.0, 10.0),
                                        egui::vec2(progress_rect.width() - 32.0, 6.0),
                                    );
                                    ui.painter().rect_filled(bar_rect, 3.0, egui::Color32::from_rgb(40, 40, 50));

                                    let filled_width = bar_rect.width() * (self.progress_percent / 100.0);
                                    let filled_rect = egui::Rect::from_min_size(
                                        bar_rect.left_top(),
                                        egui::vec2(filled_width, bar_rect.height()),
                                    );
                                    ui.painter().rect_filled(filled_rect, 3.0, primary);

                                    // Progress text
                                    let progress_text = format!(
                                        "{:.0}%  Frame {}/{}",
                                        self.progress_percent, self.current_frame, self.total_frames
                                    );
                                    ui.painter().text(
                                        progress_rect.left_bottom() - egui::Vec2::new(-16.0, 8.0),
                                        egui::Align2::LEFT_BOTTOM,
                                        progress_text,
                                        egui::FontId::monospace(11.0),
                                        text_muted,
                                    );
                                }
                            } else {
                                // Empty state - prompt to select video
                                ui.painter().text(
                                    preview_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "Select a video file to analyze",
                                    egui::FontId::proportional(16.0),
                                    text_muted,
                                );
                            }

                            ui.add_space(16.0);

                            // File picker button
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                if ui.add_sized(
                                    [160.0, 36.0],
                                    egui::Button::new("Select Video...")
                                        .fill(surface)
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

                                if self.selected_video.is_some() && !self.is_running {
                                    if ui.add_sized(
                                        [100.0, 36.0],
                                        egui::Button::new("Clear")
                                            .fill(egui::Color32::TRANSPARENT)
                                    ).clicked() {
                                        self.reset();
                                    }
                                }
                            });
                        });
                    });

                    ui.add_space(16.0);

                    // Right side - Controls and results (40%)
                    ui.vertical(|ui| {
                        ui.add_space(16.0);

                        // Control panel
                        egui::Frame::none()
                            .fill(surface)
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width() - 32.0);

                                // Model selector
                                ui.label(egui::RichText::new("MODEL").size(10.0).color(text_muted));
                                ui.add_space(4.0);

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

                                ui.add_space(16.0);

                                // Prompt input
                                ui.label(egui::RichText::new("INSTRUCTIONS").size(10.0).color(text_muted));
                                ui.add_space(4.0);

                                ui.add_enabled_ui(!self.is_running, |ui| {
                                    let text_edit = egui::TextEdit::multiline(&mut self.prompt)
                                        .hint_text("Describe what you want to analyze...\n\nExamples:\n- Describe what happens in this video\n- Identify people and their actions\n- Detect and track motion")
                                        .desired_rows(6)
                                        .desired_width(ui.available_width())
                                        .font(egui::FontId::monospace(13.0));
                                    ui.add(text_edit);
                                });

                                ui.add_space(16.0);

                                // Run button
                                let can_run = self.selected_video.is_some()
                                    && !self.prompt.trim().is_empty()
                                    && !self.is_running;

                                let button_text = if self.is_running { "Running..." } else { "Run Inference" };

                                if ui.add_sized(
                                    [ui.available_width(), 40.0],
                                    egui::Button::new(egui::RichText::new(button_text).size(14.0).strong())
                                        .fill(if can_run { primary } else { egui::Color32::from_rgb(80, 80, 90) })
                                ).clicked() && can_run {
                                    self.start_inference();
                                }

                                // Error message
                                if let Some(err) = &self.error_message {
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(err).color(primary).size(12.0));
                                }
                            });

                        ui.add_space(16.0);

                        // Results panel
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(15, 20, 25))
                            .rounding(8.0)
                            .show(ui, |ui| {
                                let results_width = (ui.available_width() - 32.0).max(100.0);
                                let results_height = (ui.available_height() - 16.0).max(100.0);
                                ui.set_width(results_width);
                                ui.set_height(results_height);

                                // Header
                                ui.horizontal(|ui| {
                                    ui.add_space(12.0);
                                    ui.label(egui::RichText::new("Results").size(13.0).strong());

                                    if let Some(status) = &self.job_status {
                                        let (status_text, status_color) = match status {
                                            JobStatus::Queued => ("Queued", text_muted),
                                            JobStatus::Extracting => ("Extracting", egui::Color32::from_rgb(255, 193, 7)),
                                            JobStatus::Inferring => ("Inferring", egui::Color32::from_rgb(255, 193, 7)),
                                            JobStatus::Completed => ("Complete", success),
                                            JobStatus::Failed => ("Failed", primary),
                                        };
                                        ui.label(egui::RichText::new(format!("  {}", status_text)).size(11.0).color(status_color));
                                    }
                                });

                                ui.add_space(8.0);
                                ui.separator();

                                // Results list
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        ui.add_space(8.0);

                                        if self.results.is_empty() {
                                            if self.is_running {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(12.0);
                                                    ui.label(egui::RichText::new("Processing...").color(text_muted));
                                                    ui.spinner();
                                                });
                                            } else if self.job_status.is_none() {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(12.0);
                                                    ui.label(egui::RichText::new("Results will appear here").color(text_muted));
                                                });
                                            }
                                        } else {
                                            for chunk in &self.results {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(12.0);
                                                    ui.label(
                                                        egui::RichText::new(format!("[{}]", Self::format_timestamp(chunk.timestamp_ms)))
                                                            .color(success)
                                                            .monospace()
                                                            .size(11.0)
                                                    );
                                                    ui.add_space(8.0);
                                                });
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.add_space(12.0);
                                                    ui.label(egui::RichText::new(&chunk.text).size(12.0));
                                                });
                                                ui.add_space(8.0);
                                            }

                                            // Typing indicator if still running
                                            if self.is_running {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(12.0);
                                                    ui.spinner();
                                                });
                                            }
                                        }

                                        ui.add_space(8.0);
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
        Box::new(move |_cc| {
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
