//! Configuration panel - project settings sidebar.
//!
//! Provides a collapsible sidebar for configuring Tools, Models, and Workflows
//! within a project.

use eframe::egui::{self, Ui};
use yama_theme::{animation, colors, font_size, radius, spacing, toggle_switch};

use super::tool_card::{tool_list, ToolConfig};

/// State for the configuration panel.
#[derive(Debug)]
pub struct ConfigPanelState {
    /// Whether the panel is open.
    pub open: bool,
    /// Whether the tools section is expanded.
    pub tools_expanded: bool,
    /// Whether the models section is expanded.
    pub models_expanded: bool,
    /// Whether the workflows section is expanded.
    pub workflows_expanded: bool,
    /// Animation progress for panel open/close (0.0 = closed, 1.0 = open).
    pub animation_progress: f32,
}

impl Default for ConfigPanelState {
    fn default() -> Self {
        Self {
            open: false,
            tools_expanded: true,
            models_expanded: false,
            workflows_expanded: false,
            animation_progress: 0.0,
        }
    }
}

impl ConfigPanelState {
    /// Toggle the panel open/closed.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }
}

/// Project configuration data.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// Available tools and their enabled state.
    pub tools: Vec<ToolConfig>,
    /// Available models and their enabled state.
    pub models: Vec<ModelConfig>,
    /// Available workflows.
    pub workflows: Vec<WorkflowConfig>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            tools: ToolConfig::default_tools(),
            models: ModelConfig::default_models(),
            workflows: WorkflowConfig::default_workflows(),
        }
    }
}

impl ProjectConfig {
    /// Count enabled tools.
    pub fn enabled_tools_count(&self) -> usize {
        self.tools.iter().filter(|t| t.enabled).count()
    }

    /// Count enabled models.
    pub fn enabled_models_count(&self) -> usize {
        self.models.iter().filter(|m| m.enabled).count()
    }

    /// Count enabled workflows.
    pub fn enabled_workflows_count(&self) -> usize {
        self.workflows.iter().filter(|w| w.enabled).count()
    }
}

/// Model configuration entry.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Model type (e.g., VLM, LLM, embedding).
    pub model_type: ModelType,
    /// Whether the model is enabled.
    pub enabled: bool,
}

/// Type of model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// Vision-Language Model.
    Vlm,
    /// Large Language Model.
    Llm,
    /// Embedding model.
    Embedding,
    /// Speech-to-text model.
    Transcription,
}

impl ModelType {
    /// Get the icon for this model type.
    pub fn icon(self) -> &'static str {
        match self {
            ModelType::Vlm => "👁️",
            ModelType::Llm => "💬",
            ModelType::Embedding => "📊",
            ModelType::Transcription => "🎤",
        }
    }

    /// Get the label for this model type.
    pub fn label(self) -> &'static str {
        match self {
            ModelType::Vlm => "Vision",
            ModelType::Llm => "Language",
            ModelType::Embedding => "Embedding",
            ModelType::Transcription => "Transcription",
        }
    }
}

impl ModelConfig {
    /// Create a new model configuration.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        model_type: ModelType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            model_type,
            enabled: true,
        }
    }

    /// Default models for a new project.
    pub fn default_models() -> Vec<Self> {
        vec![
            Self::new(
                "llava-1.6",
                "LLaVA 1.6",
                "Vision-language model for image understanding",
                ModelType::Vlm,
            ),
            Self::new(
                "whisper-large-v3",
                "Whisper Large v3",
                "Speech-to-text transcription",
                ModelType::Transcription,
            ),
            Self::new(
                "clip-vit-l",
                "CLIP ViT-L/14",
                "Visual embedding model for similarity search",
                ModelType::Embedding,
            ),
        ]
    }
}

/// Workflow configuration entry.
#[derive(Debug, Clone)]
pub struct WorkflowConfig {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Whether the workflow is enabled.
    pub enabled: bool,
}

impl WorkflowConfig {
    /// Create a new workflow configuration.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            enabled: true,
        }
    }

    /// Default workflows for a new project.
    pub fn default_workflows() -> Vec<Self> {
        vec![
            Self::new(
                "full-index",
                "Full Indexing",
                "Extract keyframes, transcribe, and generate embeddings",
            ),
            Self::new(
                "quick-scan",
                "Quick Scan",
                "Extract keyframes and generate visual embeddings only",
            ),
            Self::new(
                "transcript-only",
                "Transcript Only",
                "Transcribe audio without visual analysis",
            ),
        ]
    }
}

/// Action returned from the config panel.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigPanelAction {
    /// No action.
    None,
    /// Close the panel.
    Close,
    /// Tool enable/disable changed.
    ToolsChanged,
    /// Model enable/disable changed.
    ModelsChanged,
    /// Workflow enable/disable changed.
    WorkflowsChanged,
}

/// Configuration panel component.
pub struct ConfigPanel<'a> {
    state: &'a mut ConfigPanelState,
    config: &'a mut ProjectConfig,
    panel_width: f32,
}

impl<'a> ConfigPanel<'a> {
    /// Create a new configuration panel.
    pub fn new(state: &'a mut ConfigPanelState, config: &'a mut ProjectConfig) -> Self {
        Self {
            state,
            config,
            panel_width: 320.0,
        }
    }

    /// Set the panel width.
    pub fn width(mut self, width: f32) -> Self {
        self.panel_width = width;
        self
    }

    /// Display the configuration panel.
    pub fn show(self, ctx: &egui::Context, _ui: &mut Ui) -> ConfigPanelAction {
        let mut action = ConfigPanelAction::None;

        // Animate panel
        let target = if self.state.open { 1.0 } else { 0.0 };
        self.state.animation_progress = ctx.animate_value_with_time(
            egui::Id::new("config_panel_animation"),
            target,
            animation::duration::NORMAL,
        );

        // Don't render if fully closed
        if self.state.animation_progress < 0.01 {
            return action;
        }

        let animated_width = self.panel_width * animation::easing::ease_out_cubic(self.state.animation_progress);

        egui::SidePanel::right("config_panel")
            .resizable(false)
            .exact_width(animated_width)
            .frame(
                egui::Frame::none()
                    .fill(colors::BASALT)
                    .inner_margin(egui::Margin::same(0.0)),
            )
            .show(ctx, |ui| {
                action = self.show_content(ui);
            });

        action
    }

    fn show_content(self, ui: &mut Ui) -> ConfigPanelAction {
        let mut action = ConfigPanelAction::None;

        // Header
        ui.add_space(spacing::S3);
        ui.horizontal(|ui| {
            ui.add_space(spacing::S3);
            ui.label(
                egui::RichText::new("⚙")
                    .color(colors::AMBER)
                    .size(font_size::H3),
            );
            ui.add_space(spacing::S2);
            ui.label(
                egui::RichText::new("Project Settings")
                    .color(colors::CHALK)
                    .size(font_size::H3)
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(spacing::S3);
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("×").color(colors::SILVER).size(18.0))
                            .fill(egui::Color32::TRANSPARENT)
                            .frame(false),
                    )
                    .on_hover_text("Close settings")
                    .clicked()
                {
                    action = ConfigPanelAction::Close;
                }
            });
        });
        ui.add_space(spacing::S2);

        // Separator
        let rect = ui.available_rect_before_wrap();
        ui.painter().hline(
            rect.x_range(),
            rect.top(),
            egui::Stroke::new(1.0, colors::STONE),
        );

        // Scrollable content
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(spacing::S3);

                // Tools section
                let tools_expanded = self.state.tools_expanded;
                let tools_enabled = self.config.enabled_tools_count();
                if show_section_header(ui, "Tools", &mut self.state.tools_expanded, tools_enabled) {
                    // Section was toggled
                }
                if tools_expanded {
                    ui.add_space(spacing::S2);
                    egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(spacing::S3, 0.0))
                        .show(ui, |ui| {
                            if tool_list(ui, &mut self.config.tools, false) {
                                action = ConfigPanelAction::ToolsChanged;
                            }
                        });
                }

                ui.add_space(spacing::S2);

                // Models section
                let models_expanded = self.state.models_expanded;
                let models_enabled = self.config.enabled_models_count();
                if show_section_header(ui, "Models", &mut self.state.models_expanded, models_enabled) {
                    // Section was toggled
                }
                if models_expanded {
                    ui.add_space(spacing::S2);
                    egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(spacing::S3, 0.0))
                        .show(ui, |ui| {
                            for model in &mut self.config.models {
                                if show_model_card(ui, model) {
                                    action = ConfigPanelAction::ModelsChanged;
                                }
                                ui.add_space(spacing::S1);
                            }
                        });
                }

                ui.add_space(spacing::S2);

                // Workflows section
                let workflows_expanded = self.state.workflows_expanded;
                let workflows_enabled = self.config.enabled_workflows_count();
                if show_section_header(ui, "Workflows", &mut self.state.workflows_expanded, workflows_enabled) {
                    // Section was toggled
                }
                if workflows_expanded {
                    ui.add_space(spacing::S2);
                    egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(spacing::S3, 0.0))
                        .show(ui, |ui| {
                            for workflow in &mut self.config.workflows {
                                if show_workflow_card(ui, workflow) {
                                    action = ConfigPanelAction::WorkflowsChanged;
                                }
                                ui.add_space(spacing::S1);
                            }
                        });
                }

                ui.add_space(spacing::S4);
            });

        action
    }
}

/// Show a collapsible section header.
fn show_section_header(ui: &mut Ui, title: &str, expanded: &mut bool, enabled_count: usize) -> bool {
    let mut toggled = false;

    ui.horizontal(|ui| {
        ui.add_space(spacing::S3);

        // Collapse/expand arrow
        let arrow = if *expanded { "▼" } else { "▶" };
        if ui
            .add(
                egui::Button::new(egui::RichText::new(arrow).color(colors::SILVER).size(12.0))
                    .fill(egui::Color32::TRANSPARENT)
                    .frame(false),
            )
            .clicked()
        {
            *expanded = !*expanded;
            toggled = true;
        }

        ui.add_space(spacing::S1);

        // Section title
        ui.label(
            egui::RichText::new(title)
                .color(colors::CHALK)
                .size(font_size::BODY)
                .strong(),
        );

        // Count badge
        ui.label(
            egui::RichText::new(format!("({} enabled)", enabled_count))
                .color(colors::ASH)
                .size(font_size::SMALL),
        );
    });

    toggled
}

/// Show a model configuration card.
fn show_model_card(ui: &mut Ui, model: &mut ModelConfig) -> bool {
    let mut changed = false;

    let height = 48.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );

    // Background
    let bg_color = if response.hovered() {
        colors::GRAPHITE
    } else {
        colors::SLATE
    };

    ui.painter().rect(
        rect,
        radius::MD,
        bg_color,
        egui::Stroke::new(1.0, colors::STONE),
    );

    // Content
    let content_rect = rect.shrink(spacing::S3);

    // Icon
    ui.painter().text(
        egui::pos2(content_rect.left() + 10.0, content_rect.center().y),
        egui::Align2::CENTER_CENTER,
        model.model_type.icon(),
        egui::FontId::proportional(16.0),
        colors::SILVER,
    );

    // Name
    ui.painter().text(
        egui::pos2(content_rect.left() + 28.0, content_rect.top() + spacing::S2),
        egui::Align2::LEFT_TOP,
        &model.name,
        egui::FontId::proportional(font_size::BODY),
        colors::CHALK,
    );

    // Type label
    ui.painter().text(
        egui::pos2(content_rect.left() + 28.0, content_rect.bottom() - spacing::S2),
        egui::Align2::LEFT_BOTTOM,
        model.model_type.label(),
        egui::FontId::proportional(font_size::TINY),
        colors::ASH,
    );

    // Toggle
    let toggle_rect = egui::Rect::from_min_size(
        egui::pos2(content_rect.right() - 36.0, content_rect.center().y - 10.0),
        egui::vec2(36.0, 20.0),
    );

    let mut toggle_ui = ui.new_child(egui::UiBuilder::new()
        .max_rect(toggle_rect)
        .layout(egui::Layout::left_to_right(egui::Align::Center)));
    if toggle_switch(&mut toggle_ui, &mut model.enabled).changed() {
        changed = true;
    }

    // Handle card click
    if response.clicked() {
        let click_pos = response.interact_pointer_pos().unwrap_or_default();
        if !toggle_rect.contains(click_pos) {
            model.enabled = !model.enabled;
            changed = true;
        }
    }

    changed
}

/// Show a workflow configuration card.
fn show_workflow_card(ui: &mut Ui, workflow: &mut WorkflowConfig) -> bool {
    let mut changed = false;

    let height = 56.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );

    // Background
    let bg_color = if response.hovered() {
        colors::GRAPHITE
    } else {
        colors::SLATE
    };

    ui.painter().rect(
        rect,
        radius::MD,
        bg_color,
        egui::Stroke::new(1.0, colors::STONE),
    );

    // Content
    let content_rect = rect.shrink(spacing::S3);

    // Icon
    ui.painter().text(
        egui::pos2(content_rect.left() + 10.0, content_rect.center().y),
        egui::Align2::CENTER_CENTER,
        "⚡",
        egui::FontId::proportional(16.0),
        colors::VIOLET,
    );

    // Name
    ui.painter().text(
        egui::pos2(content_rect.left() + 28.0, content_rect.top() + spacing::S1),
        egui::Align2::LEFT_TOP,
        &workflow.name,
        egui::FontId::proportional(font_size::BODY),
        colors::CHALK,
    );

    // Description (truncated)
    let truncated_desc = if workflow.description.len() > 40 {
        format!("{}...", &workflow.description[..37])
    } else {
        workflow.description.clone()
    };

    ui.painter().text(
        egui::pos2(content_rect.left() + 28.0, content_rect.bottom() - spacing::S1),
        egui::Align2::LEFT_BOTTOM,
        &truncated_desc,
        egui::FontId::proportional(font_size::SMALL),
        colors::SILVER,
    );

    // Toggle
    let toggle_rect = egui::Rect::from_min_size(
        egui::pos2(content_rect.right() - 36.0, content_rect.center().y - 10.0),
        egui::vec2(36.0, 20.0),
    );

    let mut toggle_ui = ui.new_child(egui::UiBuilder::new()
        .max_rect(toggle_rect)
        .layout(egui::Layout::left_to_right(egui::Align::Center)));
    if toggle_switch(&mut toggle_ui, &mut workflow.enabled).changed() {
        changed = true;
    }

    // Handle card click
    if response.clicked() {
        let click_pos = response.interact_pointer_pos().unwrap_or_default();
        if !toggle_rect.contains(click_pos) {
            workflow.enabled = !workflow.enabled;
            changed = true;
        }
    }

    changed
}
