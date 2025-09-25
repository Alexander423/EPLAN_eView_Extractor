use eframe::egui;
use crate::config::Theme;

pub fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    match theme {
        Theme::Dark => apply_dark_theme(ctx),
        Theme::Light => apply_light_theme(ctx),
    }
}

fn apply_dark_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Dark color scheme
    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));

    // Window and panel backgrounds
    style.visuals.window_fill = egui::Color32::from_rgb(35, 35, 35);
    style.visuals.panel_fill = egui::Color32::from_rgb(27, 27, 27);
    style.visuals.faint_bg_color = egui::Color32::from_rgb(45, 45, 45);

    // Button styling
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(50, 50, 50);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(60, 60, 60);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 70);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(55, 55, 55);

    // Selection color
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(64, 128, 255);

    // Spacing
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.indent = 20.0;

    ctx.set_style(style);
}

fn apply_light_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Light color scheme
    style.visuals.dark_mode = false;
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(40, 40, 40));

    // Window and panel backgrounds
    style.visuals.window_fill = egui::Color32::from_rgb(248, 248, 248);
    style.visuals.panel_fill = egui::Color32::from_rgb(255, 255, 255);
    style.visuals.faint_bg_color = egui::Color32::from_rgb(240, 240, 240);

    // Button styling
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(230, 230, 230);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(220, 220, 220);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(210, 210, 210);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(200, 200, 200);

    // Selection color
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(64, 128, 255);

    // Spacing
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.indent = 20.0;

    ctx.set_style(style);
}