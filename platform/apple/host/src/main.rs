//! Yama Host Process - Apple Silicon
//!
//! This is the main entry point for the Yama compositor on Apple Silicon.
//! It runs an eframe/egui GUI with Metal rendering and manages background
//! services via tokio.

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
mod orchestrator;

use event_bus::EventBus;
use http_server::HttpServer;
use orchestrator::Orchestrator;

/// Application configuration.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub compositor: compositor::CompositorConfig,
    pub event_bus: event_bus::EventBusConfig,
    pub orchestrator: orchestrator::OrchestratorConfig,
    #[serde(default)]
    pub http_server: http_server::HttpServerConfig,
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
        }
    }
}

/// Shared application state.
pub struct AppState {
    pub config: Config,
    pub event_bus: Arc<EventBus>,
    pub orchestrator: Arc<RwLock<Orchestrator>>,
}

/// The main eframe application - End User View.
///
/// This is the Application UI for end users, focused on:
/// - Camera feeds with AI overlays
/// - Agent chat interface
/// - Clean UX without technical details
///
/// For system administration (service management, configuration, logs),
/// use the Host UI at http://localhost:8080
struct YamaApp {
    /// Shared state with background services.
    #[allow(dead_code)]
    state: Arc<AppState>,
    /// Tokio runtime for async operations.
    #[allow(dead_code)]
    runtime: Arc<tokio::runtime::Runtime>,
    /// Connection status indicator.
    connected: bool,
    /// Agent chat input.
    chat_input: String,
    /// Chat history.
    chat_history: Vec<ChatMessage>,
    /// Confidence threshold for detections.
    confidence_threshold: f32,
    /// Show about dialog.
    show_about: bool,
}

/// A chat message in the agent conversation.
struct ChatMessage {
    role: ChatRole,
    content: String,
}

#[derive(Clone, Copy, PartialEq)]
enum ChatRole {
    User,
    Agent,
}

impl YamaApp {
    fn new(state: Arc<AppState>, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        Self {
            state,
            runtime,
            connected: true, // Will be updated by actual connection status
            chat_input: String::new(),
            chat_history: vec![
                ChatMessage {
                    role: ChatRole::Agent,
                    content: "Hello! I'm your Vision AI assistant. Ask me about what I see in the camera feeds.".to_string(),
                },
            ],
            confidence_threshold: 0.5,
            show_about: false,
        }
    }
}

impl eframe::App for YamaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar - simplified for end users
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About Yama").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });

                // Connection status indicator on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.connected {
                        ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "Connected");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(200, 100, 100), "Disconnected");
                    }
                    ui.label("|");
                });
            });
        });

        // Right panel - Agent Chat (primary interaction for end users)
        egui::SidePanel::right("chat_panel")
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.heading("Vision Assistant");
                ui.separator();

                // Chat history
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 80.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for msg in &self.chat_history {
                            let (prefix, color) = match msg.role {
                                ChatRole::User => ("You: ", egui::Color32::from_rgb(100, 150, 255)),
                                ChatRole::Agent => ("AI: ", egui::Color32::from_rgb(100, 200, 100)),
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(color, prefix);
                                ui.label(&msg.content);
                            });
                            ui.add_space(8.0);
                        }
                    });

                ui.separator();

                // Chat input
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.chat_input)
                            .hint_text("Ask about what you see...")
                            .desired_width(ui.available_width() - 60.0),
                    );

                    let send_clicked = ui.button("Send").clicked();
                    let enter_pressed = response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    if (send_clicked || enter_pressed) && !self.chat_input.trim().is_empty() {
                        // Add user message
                        self.chat_history.push(ChatMessage {
                            role: ChatRole::User,
                            content: self.chat_input.clone(),
                        });

                        // TODO: Send to actual agent service
                        // For now, add a placeholder response
                        self.chat_history.push(ChatMessage {
                            role: ChatRole::Agent,
                            content: "I'm analyzing the camera feeds... (Agent service not connected)".to_string(),
                        });

                        self.chat_input.clear();
                    }
                });
            });

        // Left panel - Detections (simplified, no technical details)
        egui::SidePanel::left("detections_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Detections");
                ui.separator();

                // Confidence slider
                ui.horizontal(|ui| {
                    ui.label("Sensitivity:");
                });
                ui.add(egui::Slider::new(&mut self.confidence_threshold, 0.0..=1.0).show_value(false));
                ui.add_space(8.0);

                ui.separator();

                // Detection list (placeholder)
                ui.label("No objects detected");
                ui.add_space(16.0);

                // Camera selector
                ui.separator();
                ui.heading("Cameras");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "EAGLE-1");
                    ui.label("(active)");
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::GRAY, "EAGLE-2");
                    ui.label("(offline)");
                });
            });

        // Central panel - Video feed (main focus)
        egui::CentralPanel::default().show(ctx, |ui| {
            // Video feed takes up as much space as possible
            let available_size = ui.available_size();
            let video_rect = egui::Rect::from_min_size(
                ui.cursor().min,
                egui::Vec2::new(available_size.x, available_size.y),
            );

            // Draw video placeholder with dark background
            ui.painter().rect_filled(
                video_rect,
                8.0,
                egui::Color32::from_rgb(20, 20, 30),
            );
            ui.painter().rect_stroke(
                video_rect,
                8.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 70)),
            );

            // Center text in video area
            let text = "Waiting for video feed...\n\nCamera: EAGLE-1";
            ui.painter().text(
                video_rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(18.0),
                egui::Color32::from_rgb(120, 120, 140),
            );

            // Camera name overlay (top-left)
            ui.painter().text(
                video_rect.left_top() + egui::Vec2::new(12.0, 12.0),
                egui::Align2::LEFT_TOP,
                "EAGLE-1",
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        });

        // About dialog
        if self.show_about {
            egui::Window::new("About Yama")
                .open(&mut self.show_about)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Yama Vision AI");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(16.0);
                        ui.label("Intelligent camera monitoring with");
                        ui.label("AI-powered detection and analysis.");
                        ui.add_space(16.0);
                        ui.hyperlink_to(
                            "Administration Panel",
                            format!("http://localhost:{}", self.state.config.http_server.port),
                        );
                    });
                });
        }

        // Request continuous repainting for video
        ctx.request_repaint();
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

        Ok::<_, anyhow::Error>(Arc::new(AppState {
            config,
            event_bus,
            orchestrator,
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
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Yama - Vision AI Compositor"),
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
