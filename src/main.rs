use anyhow::Result;
use eframe::egui;
use tracing_subscriber;

mod ui;
mod scraper;
mod models;
mod export;
mod config;
mod chromedriver_manager;

use ui::EviewApp;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Setup native options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EPLAN eVIEW SPS Table Extractor")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_icon(load_icon()),
        centered: true,
        ..Default::default()
    };

    // Run the app
    eframe::run_native(
        "EPLAN eVIEW Scraper",
        options,
        Box::new(|cc| {
            // Configure fonts and style
            configure_fonts(&cc.egui_ctx);
            Ok(Box::new(EviewApp::new(cc)))
        }),
    ).map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))
}

fn load_icon() -> egui::IconData {
    // For now, return default icon. Later we can load from file
    egui::IconData::default()
}

fn configure_fonts(ctx: &egui::Context) {
    // Use default fonts for now
    // Later we can add custom fonts if needed
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(14.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(14.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(18.0),
    );
    ctx.set_style(style);
}