//! Dashboard - project overview and management.

use eframe::egui;
use yama_theme::{colors, font_size, radius, spacing};

use super::project::{Project, ProjectManager};

/// Action from the dashboard.
#[derive(Debug, Clone, PartialEq)]
pub enum DashboardAction {
    None,
    /// Open an existing project
    OpenProject(String),
    /// Create a new project
    CreateProject,
    /// Create project with a specific name
    CreateProjectNamed(String),
    /// Delete a project
    DeleteProject(String),
    /// Import files into a new project
    ImportToNewProject(Vec<std::path::PathBuf>),
}

/// Dashboard state.
#[derive(Debug, Default)]
pub struct DashboardState {
    /// Whether the new project dialog is open
    pub new_project_dialog: bool,
    /// New project name input
    pub new_project_name: String,
    /// Search/filter input
    pub filter: String,
    /// Confirm delete dialog (project ID)
    pub confirm_delete: Option<String>,
}

/// Dashboard view component.
pub struct Dashboard<'a> {
    manager: &'a ProjectManager,
    state: &'a mut DashboardState,
}

impl<'a> Dashboard<'a> {
    pub fn new(manager: &'a ProjectManager, state: &'a mut DashboardState) -> Self {
        Self { manager, state }
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> DashboardAction {
        let mut action = DashboardAction::None;

        // Check for dropped files
        let dropped_files: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        if !dropped_files.is_empty() {
            return DashboardAction::ImportToNewProject(dropped_files);
        }

        // Header
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("Projects")
                    .color(colors::CHALK)
                    .size(font_size::H1),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // New project button
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("+ New Project").color(colors::OBSIDIAN),
                        )
                        .fill(colors::AMBER)
                        .rounding(radius::MD),
                    )
                    .clicked()
                {
                    self.state.new_project_dialog = true;
                    self.state.new_project_name.clear();
                }

                ui.add_space(spacing::S3);

                // Search/filter
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.filter)
                        .desired_width(200.0)
                        .hint_text("🔍 Search projects...")
                        .font(egui::FontId::proportional(font_size::BODY)),
                );
            });
        });

        ui.add_space(spacing::S4);

        // Stats bar
        let total_projects = self.manager.projects.len();
        let total_videos: usize = self.manager.projects.iter().map(|p| p.video_count()).sum();

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} projects", total_projects))
                    .color(colors::SILVER)
                    .size(font_size::BODY),
            );
            ui.label(egui::RichText::new("•").color(colors::STONE));
            ui.label(
                egui::RichText::new(format!("{} videos total", total_videos))
                    .color(colors::SILVER)
                    .size(font_size::BODY),
            );
        });

        ui.add_space(spacing::S4);
        ui.separator();
        ui.add_space(spacing::S4);

        // Project grid
        let filtered_projects: Vec<&Project> = self
            .manager
            .projects
            .iter()
            .filter(|p| {
                self.state.filter.is_empty()
                    || p.name.to_lowercase().contains(&self.state.filter.to_lowercase())
            })
            .collect();

        if filtered_projects.is_empty() {
            // Empty state
            ui.vertical_centered(|ui| {
                ui.add_space(spacing::S8);

                if self.manager.projects.is_empty() {
                    ui.label(
                        egui::RichText::new("No projects yet")
                            .color(colors::SILVER)
                            .size(font_size::H2),
                    );
                    ui.add_space(spacing::S3);
                    ui.label(
                        egui::RichText::new("Create a new project or drop video files here")
                            .color(colors::ASH)
                            .size(font_size::BODY),
                    );
                    ui.add_space(spacing::S4);

                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Create First Project").color(colors::OBSIDIAN),
                            )
                            .fill(colors::AMBER)
                            .rounding(radius::MD),
                        )
                        .clicked()
                    {
                        self.state.new_project_dialog = true;
                    }
                } else {
                    ui.label(
                        egui::RichText::new("No projects match your search")
                            .color(colors::SILVER)
                            .size(font_size::H3),
                    );
                }

                ui.add_space(spacing::S8);
            });
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let available_width = ui.available_width();
                    let card_width = 280.0;
                    let card_spacing = spacing::S3;
                    let columns = ((available_width + card_spacing) / (card_width + card_spacing))
                        .floor() as usize;
                    let columns = columns.max(1);

                    egui::Grid::new("project_grid")
                        .spacing([card_spacing, card_spacing])
                        .show(ui, |ui| {
                            for (i, project) in filtered_projects.iter().enumerate() {
                                let card_action = self.show_project_card(ui, project, card_width);
                                if card_action != DashboardAction::None {
                                    action = card_action;
                                }

                                if (i + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
        }

        // New project dialog
        if self.state.new_project_dialog {
            let dialog_action = self.show_new_project_dialog(ctx);
            if dialog_action != DashboardAction::None {
                action = dialog_action;
            }
        }

        // Confirm delete dialog
        if self.state.confirm_delete.is_some() {
            let delete_action = self.show_confirm_delete_dialog(ctx);
            if delete_action != DashboardAction::None {
                action = delete_action;
            }
        }

        action
    }

    fn show_project_card(&mut self, ui: &mut egui::Ui, project: &Project, width: f32) -> DashboardAction {
        let mut action = DashboardAction::None;
        let height = 160.0;

        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

        // Background
        let bg_color = if response.hovered() {
            colors::GRAPHITE
        } else {
            colors::SLATE
        };

        ui.painter().rect(
            rect,
            radius::LG,
            bg_color,
            egui::Stroke::new(1.0, colors::STONE),
        );

        let padding = spacing::S3;

        // Thumbnail area (top portion)
        let thumb_height = 70.0;
        let thumb_rect = egui::Rect::from_min_size(
            rect.min + egui::vec2(padding, padding),
            egui::vec2(width - padding * 2.0, thumb_height),
        );

        ui.painter().rect_filled(
            thumb_rect,
            radius::MD,
            colors::OBSIDIAN,
        );

        // Video count badge on thumbnail
        let badge_text = format!("{} videos", project.video_count());
        let badge_rect = egui::Rect::from_min_size(
            thumb_rect.left_top() + egui::vec2(6.0, 6.0),
            egui::vec2(70.0, 20.0),
        );
        ui.painter().rect_filled(badge_rect, radius::SM, colors::with_alpha(colors::OBSIDIAN, 200));
        ui.painter().text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            &badge_text,
            egui::FontId::proportional(font_size::TINY),
            colors::CHALK,
        );

        // Duration on thumbnail (right side)
        if project.video_count() > 0 {
            let dur_text = project.duration_string();
            let dur_rect = egui::Rect::from_min_size(
                thumb_rect.right_top() + egui::vec2(-50.0, 6.0),
                egui::vec2(44.0, 20.0),
            );
            ui.painter().rect_filled(dur_rect, radius::SM, colors::with_alpha(colors::OBSIDIAN, 200));
            ui.painter().text(
                dur_rect.center(),
                egui::Align2::CENTER_CENTER,
                &dur_text,
                egui::FontId::proportional(font_size::TINY),
                colors::SILVER,
            );
        }

        // Folder icon
        ui.painter().text(
            thumb_rect.center(),
            egui::Align2::CENTER_CENTER,
            "📁",
            egui::FontId::proportional(24.0),
            colors::with_alpha(colors::CHALK, 100),
        );

        // Project name
        let name_top = thumb_rect.bottom() + spacing::S2;
        ui.painter().text(
            egui::pos2(rect.left() + padding, name_top),
            egui::Align2::LEFT_TOP,
            &project.name,
            egui::FontId::proportional(font_size::BODY),
            colors::CHALK,
        );

        // Indexed status
        let indexed = project.indexed_count();
        let total = project.video_count();
        let status_top = name_top + font_size::BODY + 4.0;

        let (status_text, status_color) = if total == 0 {
            ("Empty project".to_string(), colors::ASH)
        } else if indexed == total {
            ("All indexed".to_string(), colors::JADE)
        } else if indexed > 0 {
            (format!("{}/{} indexed", indexed, total), colors::AMBER)
        } else {
            ("Not indexed".to_string(), colors::ASH)
        };

        ui.painter().text(
            egui::pos2(rect.left() + padding, status_top),
            egui::Align2::LEFT_TOP,
            &status_text,
            egui::FontId::proportional(font_size::SMALL),
            status_color,
        );

        // Updated timestamp
        let updated_top = status_top + font_size::SMALL + 4.0;
        ui.painter().text(
            egui::pos2(rect.left() + padding, updated_top),
            egui::Align2::LEFT_TOP,
            &project.updated_ago_string(),
            egui::FontId::proportional(font_size::TINY),
            colors::ASH,
        );

        // Delete button (show on hover)
        if response.hovered() {
            let delete_rect = egui::Rect::from_min_size(
                rect.right_bottom() - egui::vec2(30.0, 30.0),
                egui::vec2(24.0, 24.0),
            );

            let delete_hovered = delete_rect.contains(response.hover_pos().unwrap_or_default());
            let delete_bg = if delete_hovered { colors::EMBER } else { colors::GRAPHITE };

            ui.painter().rect_filled(delete_rect, radius::SM, delete_bg);
            ui.painter().text(
                delete_rect.center(),
                egui::Align2::CENTER_CENTER,
                "🗑",
                egui::FontId::proportional(12.0),
                colors::CHALK,
            );

            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    if delete_rect.contains(pos) {
                        self.state.confirm_delete = Some(project.id.clone());
                        return DashboardAction::None;
                    }
                }
            }
        }

        // Handle click (open project)
        if response.clicked() {
            action = DashboardAction::OpenProject(project.id.clone());
        }

        action
    }

    fn show_new_project_dialog(&mut self, ctx: &egui::Context) -> DashboardAction {
        let mut action = DashboardAction::None;
        let mut close = false;

        egui::Window::new("New Project")
            .collapsible(false)
            .resizable(false)
            .default_size([350.0, 150.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(
                egui::Frame::none()
                    .fill(colors::SLATE)
                    .rounding(radius::LG)
                    .inner_margin(spacing::S4)
                    .stroke(egui::Stroke::new(1.0, colors::STONE)),
            )
            .show(ctx, |ui| {
                ui.heading(
                    egui::RichText::new("Create New Project")
                        .color(colors::CHALK)
                        .size(font_size::H3),
                );

                ui.add_space(spacing::S4);

                ui.label(
                    egui::RichText::new("Project Name")
                        .color(colors::SILVER)
                        .size(font_size::SMALL),
                );

                let name_response = ui.add(
                    egui::TextEdit::singleline(&mut self.state.new_project_name)
                        .desired_width(ui.available_width())
                        .hint_text("Enter project name...")
                        .font(egui::FontId::proportional(font_size::BODY)),
                );

                // Auto-focus
                if name_response.gained_focus() || self.state.new_project_name.is_empty() {
                    name_response.request_focus();
                }

                ui.add_space(spacing::S4);

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_create = !self.state.new_project_name.trim().is_empty();

                        if ui
                            .add_enabled(
                                can_create,
                                egui::Button::new(
                                    egui::RichText::new("Create").color(colors::OBSIDIAN),
                                )
                                .fill(if can_create { colors::AMBER } else { colors::STONE })
                                .rounding(radius::MD),
                            )
                            .clicked()
                        {
                            action = DashboardAction::CreateProjectNamed(
                                self.state.new_project_name.trim().to_string(),
                            );
                            close = true;
                        }

                        ui.add_space(spacing::S2);

                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("Cancel").color(colors::CHALK))
                                    .fill(colors::GRAPHITE)
                                    .rounding(radius::MD),
                            )
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });

                // Handle Enter key
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.state.new_project_name.trim().is_empty() {
                    action = DashboardAction::CreateProjectNamed(
                        self.state.new_project_name.trim().to_string(),
                    );
                    close = true;
                }

                // Handle Escape key
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });

        if close {
            self.state.new_project_dialog = false;
            self.state.new_project_name.clear();
        }

        action
    }

    fn show_confirm_delete_dialog(&mut self, ctx: &egui::Context) -> DashboardAction {
        let mut action = DashboardAction::None;
        let mut close = false;

        let project_id = self.state.confirm_delete.clone().unwrap_or_default();
        let project_name = self
            .manager
            .get_project(&project_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        egui::Window::new("Delete Project")
            .collapsible(false)
            .resizable(false)
            .default_size([350.0, 120.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(
                egui::Frame::none()
                    .fill(colors::SLATE)
                    .rounding(radius::LG)
                    .inner_margin(spacing::S4)
                    .stroke(egui::Stroke::new(1.0, colors::EMBER)),
            )
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(format!("Delete \"{}\"?", project_name))
                        .color(colors::CHALK)
                        .size(font_size::BODY),
                );

                ui.add_space(spacing::S2);

                ui.label(
                    egui::RichText::new("This action cannot be undone.")
                        .color(colors::EMBER)
                        .size(font_size::SMALL),
                );

                ui.add_space(spacing::S4);

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("Delete").color(colors::CHALK))
                                    .fill(colors::EMBER)
                                    .rounding(radius::MD),
                            )
                            .clicked()
                        {
                            action = DashboardAction::DeleteProject(project_id.clone());
                            close = true;
                        }

                        ui.add_space(spacing::S2);

                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("Cancel").color(colors::CHALK))
                                    .fill(colors::GRAPHITE)
                                    .rounding(radius::MD),
                            )
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });

                // Handle Escape key
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });

        if close {
            self.state.confirm_delete = None;
        }

        action
    }
}
