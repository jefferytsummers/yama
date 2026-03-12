//! Yama Design System Style Guide
//!
//! Visual reference for the Obsidian Lens design system.
//! Run with: `cargo run --example style_guide -p yama-theme`

use eframe::egui;
use yama_theme::{
    animation, colors, components::{self, *}, font_size, radius, spacing, status_dot, RichTextExt,
    Status, YamaTheme,
};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 800.0])
            .with_title("Yama Style Guide"),
        ..Default::default()
    };

    eframe::run_native(
        "Yama Style Guide",
        options,
        Box::new(|cc| {
            YamaTheme::new().apply(&cc.egui_ctx);
            Ok(Box::new(StyleGuideApp::default()))
        }),
    )
}

#[derive(Default)]
struct StyleGuideApp {
    text_input: String,
    slider_value: f32,
    checkbox: bool,
}

impl eframe::App for StyleGuideApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(egui::RichText::new("Yama Style Guide").heading1());
                ui.label(
                    egui::RichText::new("Obsidian Lens Design System")
                        .color(colors::TEXT_MUTED)
                        .size(font_size::H3),
                );
                ui.add_space(spacing::S6);

                // Color Palette
                ui.heading(egui::RichText::new("Color Palette").heading2());
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    color_swatch(ui, "Obsidian", colors::OBSIDIAN);
                    color_swatch(ui, "Basalt", colors::BASALT);
                    color_swatch(ui, "Slate", colors::SLATE);
                    color_swatch(ui, "Graphite", colors::GRAPHITE);
                    color_swatch(ui, "Stone", colors::STONE);
                });

                ui.add_space(spacing::S3);

                ui.horizontal(|ui| {
                    color_swatch(ui, "Amber", colors::AMBER);
                    color_swatch(ui, "Ember", colors::EMBER);
                    color_swatch(ui, "Jade", colors::JADE);
                    color_swatch(ui, "Azure", colors::AZURE);
                    color_swatch(ui, "Violet", colors::VIOLET);
                });

                ui.add_space(spacing::S3);

                ui.horizontal(|ui| {
                    color_swatch(ui, "Chalk", colors::CHALK);
                    color_swatch(ui, "Silver", colors::SILVER);
                    color_swatch(ui, "Ash", colors::ASH);
                });

                ui.add_space(spacing::S6);

                // Typography
                ui.heading(egui::RichText::new("Typography").heading2());
                ui.add_space(spacing::S2);

                ui.label(egui::RichText::new("Display Text (32px)").size(font_size::DISPLAY));
                ui.label(egui::RichText::new("Heading 1 (24px)").heading1());
                ui.label(egui::RichText::new("Heading 2 (20px)").heading2());
                ui.label(egui::RichText::new("Heading 3 (16px)").heading3());
                ui.label(egui::RichText::new("Body Text (14px)").body());
                ui.label(egui::RichText::new("Small Text (12px)").small());
                ui.label(egui::RichText::new("Monospace").mono());

                ui.add_space(spacing::S6);

                // Status Indicators
                ui.heading(egui::RichText::new("Status Indicators").heading2());
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Running (Glow + Jade)");
                        status_dot(ui, Status::Running, 16.0);
                    });
                    ui.add_space(spacing::S4);
                    ui.vertical(|ui| {
                        ui.label("Stopped (Ash)");
                        status_dot(ui, Status::Stopped, 16.0);
                    });
                    ui.add_space(spacing::S4);
                    ui.vertical(|ui| {
                        ui.label("Starting (Pulse)");
                        status_dot(ui, Status::Starting, 16.0);
                    });
                    ui.add_space(spacing::S4);
                    ui.vertical(|ui| {
                        ui.label("Failed (Ember)");
                        status_dot(ui, Status::Failed, 16.0);
                    });
                    ui.add_space(spacing::S4);
                    ui.vertical(|ui| {
                        ui.label("Inferring (Violet + Pulse)");
                        status_dot(ui, Status::Inferring, 16.0);
                    });
                });

                ui.add_space(spacing::S2);

                // Status with labels using new component
                ui.horizontal(|ui| {
                    StatusIndicator::new(components::Status::Running)
                        .with_label()
                        .show(ui);
                    ui.add_space(spacing::S4);
                    StatusIndicator::new(components::Status::Inferring)
                        .with_label()
                        .show(ui);
                });

                ui.add_space(spacing::S6);

                // Buttons
                ui.heading(egui::RichText::new("Buttons").heading2());
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    if ui.add(primary_button("Primary")).clicked() {}
                    ui.add_space(spacing::S2);
                    if ui.add(secondary_button("Secondary")).clicked() {}
                    ui.add_space(spacing::S2);
                    if ui.add(danger_button("Danger")).clicked() {}
                });

                ui.add_space(spacing::S6);

                // Badges
                ui.heading(egui::RichText::new("Badges").heading2());
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    Badge::new("Default").show(ui);
                    ui.add_space(spacing::S2);
                    Badge::new("Primary").variant(BadgeVariant::Primary).show(ui);
                    ui.add_space(spacing::S2);
                    Badge::new("Success").variant(BadgeVariant::Success).show(ui);
                    ui.add_space(spacing::S2);
                    Badge::new("Error").variant(BadgeVariant::Error).show(ui);
                    ui.add_space(spacing::S2);
                    Badge::new("Info").variant(BadgeVariant::Info).show(ui);
                    ui.add_space(spacing::S2);
                    Badge::new("Inference")
                        .variant(BadgeVariant::Inference)
                        .show(ui);
                });

                ui.add_space(spacing::S6);

                // Progress Bars
                ui.heading(egui::RichText::new("Progress Bars").heading2());
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    ui.label("0%");
                    ui.add_space(spacing::S2);
                });
                ProgressBar::new(0.0).height(4.0).ui(ui);
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    ui.label("33%");
                    ui.add_space(spacing::S2);
                });
                ProgressBar::new(0.33).height(4.0).ui(ui);
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    ui.label("66%");
                    ui.add_space(spacing::S2);
                });
                ProgressBar::new(0.66).height(4.0).ui(ui);
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    ui.label("100%");
                    ui.add_space(spacing::S2);
                });
                ProgressBar::new(1.0).height(4.0).ui(ui);
                ui.add_space(spacing::S2);

                ui.label("Large (8px height):");
                ProgressBar::new(0.75).height(8.0).ui(ui);

                ui.add_space(spacing::S6);

                // Cards
                ui.heading(egui::RichText::new("Cards").heading2());
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    Card::new().with_title("Service Card").show(ui, |ui| {
                        ui.horizontal(|ui| {
                            status_dot(ui, Status::Running, 12.0);
                            ui.label("Video Decoder");
                        });
                        ui.label(
                            egui::RichText::new("Processing 30 fps")
                                .small()
                                .color(colors::TEXT_MUTED),
                        );
                    });

                    ui.add_space(spacing::S4);

                    Card::new().with_title("Metrics").show(ui, |ui| {
                        ui.label("CPU: 45%");
                        ui.label("Memory: 2.3 GB");
                        ui.label("GPU: 78%");
                    });
                });

                ui.add_space(spacing::S6);

                // Form Elements
                ui.heading(egui::RichText::new("Form Elements").heading2());
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    ui.label("Text Input:");
                    ui.text_edit_singleline(&mut self.text_input);
                });

                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    ui.label("Slider:");
                    ui.add(egui::Slider::new(&mut self.slider_value, 0.0..=100.0));
                });

                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.checkbox, "Checkbox");
                });

                ui.add_space(spacing::S6);

                // Animation Demo
                ui.heading(egui::RichText::new("Animation").heading2());
                ui.add_space(spacing::S2);

                let pulse = animation::pulse_alpha(ctx);
                ui.horizontal(|ui| {
                    ui.label(format!("Pulse Alpha: {:.2}", pulse));
                    let pulsing_color = colors::with_alpha(colors::AMBER, (pulse * 255.0) as u8);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 8.0, pulsing_color);
                });

                ui.add_space(spacing::S6);

                // Spacing & Radius
                ui.heading(egui::RichText::new("Spacing Scale").heading2());
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    spacing_swatch(ui, "S1", spacing::S1);
                    spacing_swatch(ui, "S2", spacing::S2);
                    spacing_swatch(ui, "S3", spacing::S3);
                    spacing_swatch(ui, "S4", spacing::S4);
                    spacing_swatch(ui, "S6", spacing::S6);
                    spacing_swatch(ui, "S8", spacing::S8);
                });

                ui.add_space(spacing::S4);

                ui.heading(egui::RichText::new("Border Radius").heading2());
                ui.add_space(spacing::S2);

                ui.horizontal(|ui| {
                    radius_swatch(ui, "SM", radius::SM);
                    radius_swatch(ui, "MD", radius::MD);
                    radius_swatch(ui, "LG", radius::LG);
                    radius_swatch(ui, "XL", radius::XL);
                });

                ui.add_space(spacing::S8);
            });
        });
    }
}

fn color_swatch(ui: &mut egui::Ui, name: &str, color: egui::Color32) {
    ui.vertical(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(60.0, 40.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(radius::SM), color);
        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(radius::SM),
            egui::Stroke::new(1.0, colors::BORDER),
        );
        ui.label(egui::RichText::new(name).small());
    });
}

fn spacing_swatch(ui: &mut egui::Ui, name: &str, size: f32) {
    ui.vertical(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, 20.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::Rounding::ZERO, colors::AMBER);
        ui.label(egui::RichText::new(format!("{} ({})", name, size)).small());
    });
}

fn radius_swatch(ui: &mut egui::Ui, name: &str, r: f32) {
    ui.vertical(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(50.0, 30.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(r), colors::SURFACE);
        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(r),
            egui::Stroke::new(1.0, colors::BORDER),
        );
        ui.label(egui::RichText::new(format!("{} ({})", name, r)).small());
    });
}
