use crate::config::AppConfig;
use crate::models::PlcTable;
use crate::scraper::{ScraperEngine, ScraperConfig};
use crate::ui::table_view::TableView;
use crate::ui::themes;
use crate::chromedriver_manager::ChromeDriverManager;
use eframe::egui;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use chrono;

pub struct EviewApp {
    config: AppConfig,
    plc_table: PlcTable,
    table_view: TableView,
    scraper: Arc<Mutex<Option<ScraperEngine>>>,
    is_extracting: bool,

    // Enhanced logging system
    log_messages: Vec<LogEntry>,
    log_text_buffer: String, // For the text editor
    log_filter_level: LogLevel,
    log_auto_scroll: bool,
    log_panel_height: f32,
    show_timestamps: bool,

    // UI state
    current_tab: AppTab,
    filter_text: String,
    status_message: String,
    progress: f32,
    app_status: AppStatus,

    // Communication channels
    progress_rx: Option<mpsc::UnboundedReceiver<ProgressUpdate>>,
    extraction_handle: Option<tokio::task::JoinHandle<()>>,

    // ChromeDriver management
    chromedriver_manager: Arc<ChromeDriverManager>,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub message: String,
    pub level: LogLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppTab {
    Main,
    Logs,
    Results,
    Settings,
}

#[derive(Debug, Clone)]
pub enum ProgressUpdate {
    Log(String, LogLevel),
    Progress(f32),
    Status(String),
    Complete(PlcTable),
    Error(String),
    StatusChange(AppStatus),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppStatus {
    Ready,
    Connecting,
    Extracting,
    Processing,
    Completed,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Success,
    Debug,
}

impl LogLevel {
    pub fn color(&self) -> egui::Color32 {
        match self {
            LogLevel::Info => egui::Color32::from_rgb(200, 200, 200),
            LogLevel::Warning => egui::Color32::from_rgb(255, 193, 7),
            LogLevel::Error => egui::Color32::from_rgb(244, 67, 54),
            LogLevel::Success => egui::Color32::from_rgb(76, 175, 80),
            LogLevel::Debug => egui::Color32::from_rgb(150, 150, 255),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LogLevel::Info => "ℹ️",
            LogLevel::Warning => "⚠️",
            LogLevel::Error => "❌",
            LogLevel::Success => "✅",
            LogLevel::Debug => "🔧",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LogLevel::Info => "Info",
            LogLevel::Warning => "Warning",
            LogLevel::Error => "Error",
            LogLevel::Success => "Success",
            LogLevel::Debug => "Debug",
        }
    }
}

impl EviewApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load config
        let config = AppConfig::load().unwrap_or_default();

        // Apply theme
        themes::apply_theme(&cc.egui_ctx, &config.theme);

        Self {
            config,
            plc_table: PlcTable::new("".to_string()),
            table_view: TableView::new(),
            scraper: Arc::new(Mutex::new(None)),
            is_extracting: false,

            // Enhanced logging system
            log_messages: Vec::new(),
            log_text_buffer: String::new(),
            log_filter_level: LogLevel::Info,
            log_auto_scroll: true,
            log_panel_height: 200.0,
            show_timestamps: true,

            // UI state
            current_tab: AppTab::Main,
            filter_text: String::new(),
            status_message: "Ready".to_string(),
            progress: 0.0,
            app_status: AppStatus::Ready,

            progress_rx: None,
            extraction_handle: None,
            chromedriver_manager: Arc::new(ChromeDriverManager::new()),
        }
    }

    fn log(&mut self, message: String, level: LogLevel) {
        let log_entry = LogEntry {
            timestamp: chrono::Local::now(),
            message,
            level,
        };

        self.log_messages.push(log_entry);
        self.update_log_buffer();

        // Keep only last 1000 messages
        if self.log_messages.len() > 1000 {
            self.log_messages.remove(0);
            self.update_log_buffer();
        }
    }

    fn update_log_buffer(&mut self) {
        let filtered_messages: Vec<_> = self.log_messages
            .iter()
            .filter(|entry| self.should_show_log_level(&entry.level))
            .collect();

        self.log_text_buffer = filtered_messages
            .iter()
            .map(|entry| {
                let timestamp = if self.show_timestamps {
                    format!("[{}] ", entry.timestamp.format("%H:%M:%S"))
                } else {
                    String::new()
                };
                let icon = entry.level.icon();
                format!("{}{} {}", timestamp, icon, entry.message)
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    fn should_show_log_level(&self, level: &LogLevel) -> bool {
        match self.log_filter_level {
            LogLevel::Debug => true, // Show all
            LogLevel::Info => !matches!(level, LogLevel::Debug),
            LogLevel::Warning => matches!(level, LogLevel::Warning | LogLevel::Error),
            LogLevel::Error => matches!(level, LogLevel::Error),
            LogLevel::Success => matches!(level, LogLevel::Success | LogLevel::Error | LogLevel::Warning),
        }
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Add left margin to align with tabs
            ui.add_space(12.0);

            // Status badge
            let (badge_icon, badge_color, badge_text) = self.get_status_badge_info();
            ui.colored_label(badge_color, badge_icon);
            ui.label(badge_text);
            ui.add_space(8.0);

            // Extract button
            let extract_btn = ui.add_enabled(
                !self.is_extracting,
                egui::Button::new("🔄 Extract (Ctrl+E)")
                    .min_size(egui::vec2(120.0, 30.0))
            );

            if extract_btn.clicked() {
                self.start_extraction();
            }

            // Stop button
            if self.is_extracting {
                if ui.button("⏹ Stop").clicked() {
                    self.stop_extraction();
                }
            }

            ui.separator();

            // Export buttons
            ui.add_enabled(
                !self.plc_table.entries.is_empty(),
                egui::Button::new("📊 Export Excel")
            ).on_hover_text("Export to Excel format");

            ui.add_enabled(
                !self.plc_table.entries.is_empty(),
                egui::Button::new("📄 Export CSV")
            ).on_hover_text("Export to CSV format");

            ui.add_enabled(
                !self.plc_table.entries.is_empty(),
                egui::Button::new("📋 Copy Selected")
            ).on_hover_text("Copy selected entries to clipboard");

            ui.separator();

            // Search field
            ui.label("🔍");
            let search = ui.add(
                egui::TextEdit::singleline(&mut self.filter_text)
                    .desired_width(200.0)
                    .hint_text("Filter...")
            );

            if search.changed() {
                // Filter will be applied in table view
            }

            // Clear filter
            if !self.filter_text.is_empty() {
                if ui.button("✕").clicked() {
                    self.filter_text.clear();
                }
            }

            // Right side - empty for now, tabs handle navigation
        });
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configuration");
        ui.separator();

        // Login credentials
        ui.group(|ui| {
            ui.label("Microsoft Credentials");
            ui.spacing();

            ui.horizontal(|ui| {
                ui.label("Email:");
                ui.text_edit_singleline(&mut self.config.email);
            });

            ui.horizontal(|ui| {
                ui.label("Password:");
                ui.add(egui::TextEdit::singleline(&mut self.config.password)
                    .password(true));
            });
        });

        ui.add_space(10.0);

        // Project settings
        ui.group(|ui| {
            ui.label("Project Settings");
            ui.spacing();

            ui.horizontal(|ui| {
                ui.label("Project Number:");
                ui.text_edit_singleline(&mut self.config.project_number);
            });
        });

        ui.add_space(10.0);

        // Options
        ui.group(|ui| {
            ui.label("Options");
            ui.checkbox(&mut self.config.headless_mode, "Headless Mode");
            ui.checkbox(&mut self.config.export_excel, "Auto-Export Excel");
            ui.checkbox(&mut self.config.export_csv, "Auto-Export CSV");
        });

        ui.add_space(10.0);

        // Save config button
        if ui.button("💾 Save Config").clicked() {
            match self.config.save() {
                Ok(_) => self.log("Configuration saved".to_string(), LogLevel::Success),
                Err(e) => self.log(format!("Failed to save config: {}", e), LogLevel::Error),
            }
        }

        // Statistics
        ui.add_space(20.0);
        ui.separator();
        ui.label("Statistics");
        ui.label(format!("Total Entries: {}", self.plc_table.entries.len()));

        let inputs = self.plc_table.entries.iter()
            .filter(|e| matches!(e.data_type, crate::models::PlcDataType::Input))
            .count();
        let outputs = self.plc_table.entries.iter()
            .filter(|e| matches!(e.data_type, crate::models::PlcDataType::Output))
            .count();

        ui.label(format!("Inputs: {}", inputs));
        ui.label(format!("Outputs: {}", outputs));
    }

    fn apply_professional_theme(&self, ctx: &egui::Context) {
        let visuals = match self.config.theme {
            crate::config::Theme::Dark => {
                let mut v = egui::Visuals::dark();

                // Professional dark color scheme
                v.widgets.inactive.bg_fill = egui::Color32::from_rgb(48, 49, 52);
                v.widgets.hovered.bg_fill = egui::Color32::from_rgb(64, 65, 68);
                v.widgets.active.bg_fill = egui::Color32::from_rgb(26, 115, 232);
                v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200));
                v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

                // Dark panel colors
                v.panel_fill = egui::Color32::from_rgb(24, 25, 26);
                v.window_fill = egui::Color32::from_rgb(32, 33, 36);
                v.extreme_bg_color = egui::Color32::from_rgb(16, 17, 18);

                // Dark selection colors
                v.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(26, 115, 232, 80);
                v.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(26, 115, 232));

                v
            },
            crate::config::Theme::Light => {
                let mut v = egui::Visuals::light();

                // Professional light color scheme
                v.widgets.inactive.bg_fill = egui::Color32::from_rgb(248, 249, 250);
                v.widgets.hovered.bg_fill = egui::Color32::from_rgb(241, 243, 244);
                v.widgets.active.bg_fill = egui::Color32::from_rgb(26, 115, 232);
                v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 64, 67));
                v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(32, 33, 36));

                // Light panel colors
                v.panel_fill = egui::Color32::WHITE;
                v.window_fill = egui::Color32::from_rgb(255, 255, 255);
                v.extreme_bg_color = egui::Color32::from_rgb(248, 249, 250);

                // Light selection colors
                v.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(26, 115, 232, 40);
                v.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(26, 115, 232));

                v
            }
        };

        ctx.set_visuals(visuals);
    }

    fn get_status_badge_info(&self) -> (&'static str, egui::Color32, &'static str) {
        match &self.app_status {
            AppStatus::Ready => ("●", egui::Color32::from_rgb(76, 175, 80), "Ready"),
            AppStatus::Connecting => ("●", egui::Color32::from_rgb(255, 193, 7), "Connecting"),
            AppStatus::Extracting => ("●", egui::Color32::from_rgb(33, 150, 243), "Extracting"),
            AppStatus::Processing => ("●", egui::Color32::from_rgb(156, 39, 176), "Processing"),
            AppStatus::Completed => ("●", egui::Color32::from_rgb(76, 175, 80), "Completed"),
            AppStatus::Error(_) => ("●", egui::Color32::from_rgb(244, 67, 54), "Error"),
        }
    }

    fn get_panel_colors(&self) -> (egui::Color32, egui::Color32, egui::Color32) {
        match self.config.theme {
            crate::config::Theme::Dark => (
                egui::Color32::from_rgb(32, 33, 36),  // toolbar/status background
                egui::Color32::from_rgb(40, 41, 44),  // tab bar background
                egui::Color32::from_rgb(24, 25, 26),  // main content background
            ),
            crate::config::Theme::Light => (
                egui::Color32::from_rgb(248, 249, 250), // toolbar/status background
                egui::Color32::from_rgb(241, 243, 244), // tab bar background
                egui::Color32::WHITE,                    // main content background
            )
        }
    }

    fn get_border_color(&self) -> egui::Color32 {
        match self.config.theme {
            crate::config::Theme::Dark => egui::Color32::from_rgb(60, 61, 64),
            crate::config::Theme::Light => egui::Color32::from_rgb(218, 220, 224),
        }
    }

    fn render_tab_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Add left margin for better alignment with extract button
            ui.add_space(12.0);
            ui.spacing_mut().item_spacing.x = 2.0;

            let tabs = [
                (AppTab::Main, "🏠 Main", "Main dashboard with extraction controls (Esc)"),
                (AppTab::Logs, "📝 Logs (Ctrl+L)", "View detailed extraction logs"),
                (AppTab::Results, "📊 Results (Ctrl+R)", "View and export extracted data"),
                (AppTab::Settings, "🛠️ Settings (Ctrl+,)", "Login credentials and application preferences"),
            ];

            for (tab, label, tooltip) in tabs {
                let is_active = self.current_tab == tab;

                // Theme-based colors for tabs
                let (active_bg, inactive_bg, active_border, inactive_border) = match self.config.theme {
                    crate::config::Theme::Dark => (
                        egui::Color32::from_rgb(26, 115, 232),     // Active: Blue
                        egui::Color32::from_rgb(48, 49, 52),       // Inactive: Dark gray
                        egui::Color32::from_rgb(66, 135, 252),     // Active border: Light blue
                        egui::Color32::from_rgb(60, 61, 64),       // Inactive border: Gray
                    ),
                    crate::config::Theme::Light => (
                        egui::Color32::from_rgb(26, 115, 232),     // Active: Blue (same)
                        egui::Color32::WHITE,                       // Inactive: White
                        egui::Color32::from_rgb(66, 135, 252),     // Active border: Light blue
                        egui::Color32::from_rgb(218, 220, 224),    // Inactive border: Light gray
                    ),
                };

                let button_color = if is_active { active_bg } else { inactive_bg };
                let border_color = if is_active { active_border } else { inactive_border };

                let button = egui::Button::new(label)
                    .fill(button_color)
                    .stroke(egui::Stroke::new(
                        if is_active { 2.0 } else { 1.0 },
                        border_color
                    ))
                    .min_size(egui::Vec2::new(120.0, 32.0));

                if ui.add(button)
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    self.current_tab = tab;
                }
            }

            // Remove help button - cleaner UI
        });
    }

    fn render_main_tab(&mut self, ctx: &egui::Context) {
        let (toolbar_bg, _tab_bg, content_bg) = self.get_panel_colors();
        let border_color = self.get_border_color();

        // Sidebar for main tab
        egui::SidePanel::left("main_sidebar")
            .default_width(320.0)
            .resizable(true)
            .frame(egui::Frame {
                fill: toolbar_bg,
                stroke: egui::Stroke::new(1.0, border_color),
                inner_margin: egui::Margin::same(12.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_extraction_controls(ui);
                });
            });

        // Main content - Table view
        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: content_bg,
                inner_margin: egui::Margin::same(8.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.table_view.render(ui, &mut self.plc_table, &self.filter_text);
            });
    }


    fn render_logs_tab(&mut self, ctx: &egui::Context) {
        let (_toolbar_bg, _tab_bg, content_bg) = self.get_panel_colors();

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: content_bg,
                inner_margin: egui::Margin::same(8.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.heading("📝 Extraction Logs");
                ui.separator();
                ui.add_space(8.0);
                self.render_log_panel(ui);
            });
    }

    fn render_results_tab(&mut self, ctx: &egui::Context) {
        let (_toolbar_bg, _tab_bg, content_bg) = self.get_panel_colors();

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: content_bg,
                inner_margin: egui::Margin::same(8.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.heading("📊 Extraction Results");
                ui.separator();
                ui.add_space(8.0);

                // Export options bar
                ui.horizontal(|ui| {
                    ui.label("Export Options:");

                    ui.add_enabled(
                        !self.plc_table.entries.is_empty(),
                        egui::Button::new("📊 Excel")
                            .fill(egui::Color32::from_rgb(16, 124, 16))
                    ).on_hover_text("Export to Excel format");

                    ui.add_enabled(
                        !self.plc_table.entries.is_empty(),
                        egui::Button::new("📄 CSV")
                            .fill(egui::Color32::from_rgb(16, 124, 16))
                    ).on_hover_text("Export to CSV format");

                    ui.add_enabled(
                        !self.plc_table.entries.is_empty(),
                        egui::Button::new("📋 Copy")
                            .fill(egui::Color32::from_rgb(26, 115, 232))
                    ).on_hover_text("Copy selected to clipboard");
                });

                ui.add_space(8.0);

                // Search field
                ui.horizontal(|ui| {
                    ui.label("🔍 Filter:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.filter_text)
                            .desired_width(300.0)
                            .hint_text("Search entries...")
                    );
                    if !self.filter_text.is_empty() {
                        if ui.button("✕").clicked() {
                            self.filter_text.clear();
                        }
                    }
                });

                ui.add_space(8.0);
                self.table_view.render(ui, &mut self.plc_table, &self.filter_text);
            });
    }

    fn render_settings_tab(&mut self, ctx: &egui::Context) {
        let (_toolbar_bg, _tab_bg, content_bg) = self.get_panel_colors();

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: content_bg,
                inner_margin: egui::Margin::same(16.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("🛠️ Configuration & Settings");
                    ui.separator();
                    ui.add_space(16.0);

                    // Microsoft Credentials
                    ui.group(|ui| {
                        ui.label("🔐 Microsoft Credentials");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Email:");
                            let email_response = ui.add(
                                egui::TextEdit::singleline(&mut self.config.email)
                                    .desired_width(250.0)
                                    .hint_text("your.email@company.com")
                            );
                            if email_response.changed() {
                                let _ = self.config.save();
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Password:");
                            let password_response = ui.add(
                                egui::TextEdit::singleline(&mut self.config.password)
                                    .desired_width(250.0)
                                    .password(true)
                                    .hint_text("Enter password")
                            );
                            if password_response.changed() {
                                let _ = self.config.save();
                            }
                        });
                    });

                    ui.add_space(12.0);

                    // Project Settings
                    ui.group(|ui| {
                        ui.label("📋 Project Settings");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Project Number:");
                            let project_response = ui.add(
                                egui::TextEdit::singleline(&mut self.config.project_number)
                                    .desired_width(150.0)
                                    .hint_text("e.g., P12345")
                            );
                            if project_response.changed() {
                                let _ = self.config.save();
                            }
                        });
                    });

                    ui.add_space(16.0);

                    // Theme settings
                    ui.group(|ui| {
                        ui.label("🎨 Theme Settings");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Theme:");
                            egui::ComboBox::from_id_salt("theme_selector")
                                .selected_text(match self.config.theme {
                                    crate::config::Theme::Light => "Light",
                                    crate::config::Theme::Dark => "Dark",
                                })
                                .show_ui(ui, |ui| {
                                    if ui.selectable_value(&mut self.config.theme, crate::config::Theme::Light, "Light").clicked() {
                                        let _ = self.config.save();
                                    }
                                    if ui.selectable_value(&mut self.config.theme, crate::config::Theme::Dark, "Dark").clicked() {
                                        let _ = self.config.save();
                                    }
                                });
                        });
                    });

                    ui.add_space(12.0);

                    // Browser settings
                    ui.group(|ui| {
                        ui.label("🌐 Browser Settings");
                        ui.separator();

                        if ui.checkbox(&mut self.config.headless_mode, "Headless mode (browser runs in background)").changed() {
                            let _ = self.config.save();
                        }
                        if ui.checkbox(&mut self.config.debug_mode, "Debug mode (keep browser open on errors)").changed() {
                            let _ = self.config.save();
                        }
                    });

                    ui.add_space(12.0);

                    // Export settings
                    ui.group(|ui| {
                        ui.label("📤 Export Settings");
                        ui.separator();

                        if ui.checkbox(&mut self.config.export_excel, "Enable Excel export").changed() {
                            let _ = self.config.save();
                        }
                        if ui.checkbox(&mut self.config.export_csv, "Enable CSV export").changed() {
                            let _ = self.config.save();
                        }
                        if ui.checkbox(&mut self.config.export_json, "Enable JSON export").changed() {
                            let _ = self.config.save();
                        }

                        ui.horizontal(|ui| {
                            ui.label("Last export path:");
                            if let Some(path) = &self.config.last_export_path {
                                ui.label(path);
                            } else {
                                ui.label("(not set)");
                            }
                        });
                    });

                    ui.add_space(20.0);

                    // Save button
                    if ui.button("💾 Save Settings").clicked() {
                        if let Err(_e) = self.config.save() {
                            // Add error to log
                        } else {
                            // Add success to log
                        }
                    }
                });
            });
    }

    fn render_extraction_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔧 Extraction Controls");
        ui.separator();
        ui.add_space(8.0);

        // Login credentials section
        ui.group(|ui| {
            ui.label("🔐 Microsoft Credentials");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Email:");
                let email_response = ui.add(
                    egui::TextEdit::singleline(&mut self.config.email)
                        .desired_width(200.0)
                        .hint_text("your.email@company.com")
                );
                if email_response.changed() {
                    let _ = self.config.save();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Password:");
                let password_response = ui.add(
                    egui::TextEdit::singleline(&mut self.config.password)
                        .desired_width(200.0)
                        .password(true)
                        .hint_text("Enter password")
                );
                if password_response.changed() {
                    let _ = self.config.save();
                }
            });
        });

        ui.add_space(12.0);

        // Project settings section
        ui.group(|ui| {
            ui.label("📋 Project Settings");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Project Number:");
                let project_response = ui.add(
                    egui::TextEdit::singleline(&mut self.config.project_number)
                        .desired_width(150.0)
                        .hint_text("e.g., P12345")
                );
                if project_response.changed() {
                    let _ = self.config.save();
                }
            });
        });

        ui.add_space(16.0);

        // Status and progress
        if self.is_extracting {
            ui.group(|ui| {
                ui.label("🚀 Extraction in Progress");
                ui.separator();

                let progress_bar = egui::ProgressBar::new(self.progress)
                    .desired_width(280.0)
                    .text(format!("{:.0}%", self.progress * 100.0));
                ui.add(progress_bar);

                ui.label(&self.status_message);

                if ui.button("⏹ Stop Extraction").clicked() {
                    self.stop_extraction();
                }
            });
        } else {
            // Validation and extract button
            let validation_errors = self.config.validate();
            let can_extract = validation_errors.is_empty();

            if !validation_errors.is_empty() {
                ui.group(|ui| {
                    ui.label("⚠️ Configuration Issues");
                    ui.separator();
                    for error in &validation_errors {
                        ui.colored_label(egui::Color32::from_rgb(244, 67, 54), format!("• {}", error));
                    }
                });
                ui.add_space(8.0);
            }

            // Keyboard shortcuts section
            ui.group(|ui| {
                ui.label("⌨️ Keyboard Shortcuts");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Ctrl+E:");
                    ui.weak("Start Extraction");
                });
                ui.horizontal(|ui| {
                    ui.label("Ctrl+S:");
                    ui.weak("Save Settings");
                });
                ui.horizontal(|ui| {
                    ui.label("F5:");
                    ui.weak("Restart Extraction");
                });
                ui.horizontal(|ui| {
                    ui.label("Esc:");
                    ui.weak("Cancel/Main Tab");
                });
            });

            ui.add_space(12.0);

            let extract_btn = ui.add_sized(
                egui::Vec2::new(280.0, 40.0),
                egui::Button::new("🚀 Start Extraction")
                    .fill(if can_extract {
                        egui::Color32::from_rgb(16, 124, 16)
                    } else {
                        egui::Color32::from_rgb(100, 100, 100)
                    })
            )
            .on_hover_text(
                if can_extract {
                    "Start extracting PLC tables from eView"
                } else {
                    "Please fix configuration issues first"
                }
            );

            if extract_btn.clicked() && can_extract {
                self.start_extraction();
            }
        }
    }

    fn render_log_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("📋 Logs");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Clear logs button
                if ui.button("🗑 Clear").clicked() {
                    self.log_messages.clear();
                    self.update_log_buffer();
                }

                // Save logs button
                if ui.button("💾 Save").clicked() {
                    self.save_logs_to_file();
                }

                // Copy all logs button
                if ui.button("📋 Copy All").clicked() {
                    ui.output_mut(|o| o.copied_text = self.log_text_buffer.clone());
                    self.log("Logs copied to clipboard".to_string(), LogLevel::Success);
                }

                // Auto-scroll toggle
                if ui.selectable_label(self.log_auto_scroll, "📍 Auto-scroll").clicked() {
                    self.log_auto_scroll = !self.log_auto_scroll;
                }

                // Timestamps toggle
                if ui.selectable_label(self.show_timestamps, "⏰ Timestamps").clicked() {
                    self.show_timestamps = !self.show_timestamps;
                    self.update_log_buffer();
                }
            });
        });

        ui.separator();

        // Log level filter
        ui.horizontal(|ui| {
            ui.label("Filter:");

            let current_filter = self.log_filter_level.clone();
            egui::ComboBox::from_label("")
                .selected_text(format!("{} {}", current_filter.icon(), current_filter.name()))
                .show_ui(ui, |ui| {
                    for level in [LogLevel::Debug, LogLevel::Info, LogLevel::Success, LogLevel::Warning, LogLevel::Error] {
                        let text = format!("{} {}", level.icon(), level.name());
                        if ui.selectable_value(&mut self.log_filter_level, level.clone(), text).clicked() {
                            self.update_log_buffer();
                        }
                    }
                });

            ui.separator();
            ui.label(format!("{} entries", self.log_messages.len()));
        });

        ui.separator();

        // Enhanced resizable log area
        let available_height = ui.available_height() - 50.0; // Leave room for status bar
        let log_height = self.log_panel_height.min(available_height).max(100.0);

        ui.vertical(|ui| {
            // Resizable text area
            let text_response = ui.add_sized(
                [ui.available_width(), log_height],
                egui::TextEdit::multiline(&mut self.log_text_buffer)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(10)
                    .desired_width(f32::INFINITY)
                    .interactive(true) // Allow text selection
            );

            // Handle resize drag
            let resize_handle_rect = egui::Rect::from_min_size(
                egui::pos2(ui.min_rect().left(), text_response.rect.bottom()),
                egui::vec2(ui.available_width(), 8.0)
            );

            let resize_response = ui.allocate_rect(resize_handle_rect, egui::Sense::drag());
            if resize_response.dragged() {
                self.log_panel_height = (self.log_panel_height + resize_response.drag_delta().y)
                    .clamp(100.0, 600.0);
            }

            // Visual resize handle
            if resize_response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }

            ui.painter().hline(
                resize_handle_rect.x_range(),
                resize_handle_rect.center().y,
                egui::Stroke::new(2.0, if resize_response.hovered() {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::GRAY
                })
            );

            // Auto-scroll to bottom if enabled
            if self.log_auto_scroll && text_response.changed() {
                text_response.scroll_to_me(Some(egui::Align::BOTTOM));
            }
        });

        // Keyboard shortcuts info
        if ui.input(|i| i.key_pressed(egui::Key::F1)) {
            self.log("Keyboard shortcuts: Ctrl+A (Select All), Ctrl+C (Copy Selected), F1 (Help)".to_string(), LogLevel::Info);
        }
    }

    fn save_logs_to_file(&mut self) {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("eview_scraper_logs_{}.txt", timestamp);

        match std::fs::write(&filename, &self.log_text_buffer) {
            Ok(_) => {
                self.log(format!("Logs saved to {}", filename), LogLevel::Success);
            }
            Err(e) => {
                self.log(format!("Failed to save logs: {}", e), LogLevel::Error);
            }
        }
    }

    fn render_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.status_message);

            // Progress bar if extracting
            if self.is_extracting {
                ui.add(egui::ProgressBar::new(self.progress)
                    .desired_width(200.0)
                    .animate(true));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Add small right margin to prevent text cutoff
                ui.add_space(10.0);
                ui.label(format!(
                    "v{} | {} entries loaded",
                    env!("CARGO_PKG_VERSION"),
                    self.plc_table.entries.len()
                ));
            });
        });
    }

    fn start_extraction(&mut self) {
        // Validate config
        let errors = self.config.validate();
        if !errors.is_empty() {
            for error in errors {
                self.log(error, LogLevel::Error);
            }
            return;
        }

        // Check if already extracting
        if self.is_extracting {
            self.log("Extraction already in progress".to_string(), LogLevel::Warning);
            return;
        }

        // Cancel any previous extraction task
        if let Some(handle) = self.extraction_handle.take() {
            handle.abort();
        }
        self.progress_rx = None;

        self.is_extracting = true;
        self.status_message = "Starting extraction...".to_string();
        self.progress = 0.0;
        self.app_status = AppStatus::Connecting;
        self.log("Starting EPLAN eVIEW extraction".to_string(), LogLevel::Info);

        // Create communication channel
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();
        self.progress_rx = Some(progress_rx);

        // Clone config and chromedriver manager for the async task
        let config = self.config.clone();
        let chromedriver_manager = self.chromedriver_manager.clone();

        // Spawn async extraction task - simplified without panic handling
        let handle = tokio::spawn(async move {
            Self::run_extraction_async(config, chromedriver_manager, progress_tx).await
        });

        self.extraction_handle = Some(handle);
    }

    async fn run_extraction_async(
        config: AppConfig,
        chromedriver_manager: Arc<ChromeDriverManager>,
        progress_tx: mpsc::UnboundedSender<ProgressUpdate>,
    ) {
        let _ = progress_tx.send(ProgressUpdate::StatusChange(AppStatus::Connecting));
        let _ = progress_tx.send(ProgressUpdate::Log(
            "🚀 Starting extraction process...".to_string(),
            LogLevel::Info,
        ));

        let _ = progress_tx.send(ProgressUpdate::Progress(0.05));

        // Debug: Log the configuration (without password)
        let _ = progress_tx.send(ProgressUpdate::Log(
            format!("📧 Email: {}", config.email),
            LogLevel::Info,
        ));
        let _ = progress_tx.send(ProgressUpdate::Log(
            format!("🏢 Project: {}", config.project_number),
            LogLevel::Info,
        ));
        let _ = progress_tx.send(ProgressUpdate::Log(
            format!("👻 Headless mode: {}", config.headless_mode),
            LogLevel::Info,
        ));

        let _ = progress_tx.send(ProgressUpdate::Log(
            "🚀 Starting ChromeDriver on port 9515...".to_string(),
            LogLevel::Info,
        ));

        // ChromeDriver will be started by ScraperEngine
        let _ = progress_tx.send(ProgressUpdate::Progress(0.1));

        let _ = progress_tx.send(ProgressUpdate::Progress(0.15));

        let _ = progress_tx.send(ProgressUpdate::Log(
            "⚙️ Initializing scraper engine...".to_string(),
            LogLevel::Info,
        ));

        let scraper_config = ScraperConfig {
            base_url: "https://eview.eplan.com/".to_string(),
            username: config.email.clone(),
            password: config.password.clone(),
            project_number: config.project_number.clone(),
            headless: config.headless_mode,
        };

        let debug_mode = config.debug_mode;

        // Create a simple logger for the scraper
        struct UiLogger {
            tx: mpsc::UnboundedSender<ProgressUpdate>,
        }

        impl crate::scraper::Logger for UiLogger {
            fn log(&self, message: String, level: crate::scraper::LogLevel) {
                let ui_level = match level {
                    crate::scraper::LogLevel::Info => LogLevel::Info,
                    crate::scraper::LogLevel::Warning => LogLevel::Warning,
                    crate::scraper::LogLevel::Error => LogLevel::Error,
                    crate::scraper::LogLevel::Success => LogLevel::Success,
                    crate::scraper::LogLevel::Debug => LogLevel::Info,
                };
                let _ = self.tx.send(ProgressUpdate::Log(message, ui_level));
            }
        }

        let logger = Arc::new(Mutex::new(Box::new(UiLogger { tx: progress_tx.clone() }) as Box<dyn crate::scraper::Logger>));

        let _ = progress_tx.send(ProgressUpdate::Progress(0.2));
        let _ = progress_tx.send(ProgressUpdate::Log(
            "🔌 Creating scraper engine...".to_string(),
            LogLevel::Info,
        ));

        // Wrap scraper creation in error handling
        let scraper_result = match ScraperEngine::new(scraper_config, logger, chromedriver_manager).await {
            Ok(scraper) => {
                let _ = progress_tx.send(ProgressUpdate::Progress(0.3));
                let _ = progress_tx.send(ProgressUpdate::Status("🌐 Browser connected successfully".to_string()));
                let _ = progress_tx.send(ProgressUpdate::Log(
                    "✅ Scraper engine created successfully".to_string(),
                    LogLevel::Success,
                ));
                Ok(scraper)
            }
            Err(e) => {
                let _ = progress_tx.send(ProgressUpdate::Error(format!("❌ Failed to initialize scraper: {}", e)));
                let _ = progress_tx.send(ProgressUpdate::Log(
                    format!("❌ Scraper initialization failed: {}", e),
                    LogLevel::Error,
                ));
                let _ = progress_tx.send(ProgressUpdate::Log(
                    "💡 Common causes: ChromeDriver version mismatch, Chrome not installed, or port conflict".to_string(),
                    LogLevel::Info,
                ));
                Err(e)
            }
        };

        if let Ok(mut scraper) = scraper_result {
            let _ = progress_tx.send(ProgressUpdate::StatusChange(AppStatus::Extracting));
            let _ = progress_tx.send(ProgressUpdate::Log(
                "🚀 Starting extraction process...".to_string(),
                LogLevel::Info,
            ));

            let _ = progress_tx.send(ProgressUpdate::Log(
                "📍 Phase 1: Navigating to eView and handling Microsoft login...".to_string(),
                LogLevel::Info,
            ));

            // Wrap extraction in detailed error handling
            let extraction_result = match scraper.run_extraction().await {
                Ok(table) => {
                    let _ = progress_tx.send(ProgressUpdate::StatusChange(AppStatus::Processing));
                    let _ = progress_tx.send(ProgressUpdate::Progress(1.0));
                    let _ = progress_tx.send(ProgressUpdate::Status("🎉 Extraction complete!".to_string()));
                    let _ = progress_tx.send(ProgressUpdate::Log(
                        format!("✅ Extraction completed! Found {} entries", table.entries.len()),
                        LogLevel::Success,
                    ));
                    let _ = progress_tx.send(ProgressUpdate::Complete(table));
                    Ok(())
                }
                Err(e) => {
                    // More detailed error analysis
                    let error_msg = format!("{}", e);
                    let _ = progress_tx.send(ProgressUpdate::Error(format!("❌ Extraction failed: {}", error_msg)));

                    // Provide specific troubleshooting based on error type
                    if error_msg.contains("Microsoft login") || error_msg.contains("login") {
                        let _ = progress_tx.send(ProgressUpdate::Log(
                            "💡 Login issue detected. Check credentials and try again.".to_string(),
                            LogLevel::Info,
                        ));
                    } else if error_msg.contains("project") || error_msg.contains("Project") {
                        let _ = progress_tx.send(ProgressUpdate::Log(
                            "💡 Project access issue. Verify project number and permissions.".to_string(),
                            LogLevel::Info,
                        ));
                    } else if error_msg.contains("timeout") || error_msg.contains("Timeout") {
                        let _ = progress_tx.send(ProgressUpdate::Log(
                            "💡 Timeout occurred. eView might be slow - try again or check internet connection.".to_string(),
                            LogLevel::Info,
                        ));
                    } else if error_msg.contains("element") || error_msg.contains("Element") {
                        let _ = progress_tx.send(ProgressUpdate::Log(
                            "💡 Web element not found. eView interface may have changed.".to_string(),
                            LogLevel::Info,
                        ));
                    }

                    let _ = progress_tx.send(ProgressUpdate::Log(
                        format!("🔍 Full error details: {}", error_msg),
                        LogLevel::Error,
                    ));
                    Err(e)
                }
            };

            // Browser cleanup - respect debug mode
            if debug_mode && extraction_result.is_err() {
                let _ = progress_tx.send(ProgressUpdate::Log(
                    "🔍 Debug mode: Browser left open for inspection (you can manually close it)".to_string(),
                    LogLevel::Info,
                ));
                let _ = progress_tx.send(ProgressUpdate::Log(
                    "💡 This allows you to inspect the current page state and identify issues".to_string(),
                    LogLevel::Info,
                ));
            } else {
                let _ = progress_tx.send(ProgressUpdate::Log(
                    "🧹 Cleaning up browser...".to_string(),
                    LogLevel::Info,
                ));

                match scraper.close().await {
                    Ok(_) => {
                        let _ = progress_tx.send(ProgressUpdate::Log(
                            "✅ Browser cleanup complete".to_string(),
                            LogLevel::Success,
                        ));
                    }
                    Err(e) => {
                        let _ = progress_tx.send(ProgressUpdate::Log(
                            format!("⚠️ Browser cleanup warning: {} (this is usually not critical)", e),
                            LogLevel::Warning,
                        ));
                    }
                }
            }

            // Report final status
            if extraction_result.is_ok() {
                let _ = progress_tx.send(ProgressUpdate::Log(
                    "🏁 Extraction process completed successfully".to_string(),
                    LogLevel::Success,
                ));
            } else {
                let _ = progress_tx.send(ProgressUpdate::Log(
                    "🏁 Extraction process finished with errors - see above for details".to_string(),
                    LogLevel::Error,
                ));
            }
        }

        let _ = progress_tx.send(ProgressUpdate::Log(
            "🏁 Extraction process finished".to_string(),
            LogLevel::Info,
        ));
    }

    fn stop_extraction(&mut self) {
        // Cancel the extraction task if running
        if let Some(handle) = self.extraction_handle.take() {
            handle.abort();
        }

        self.is_extracting = false;
        self.status_message = "Extraction stopped".to_string();
        self.progress = 0.0;
        self.progress_rx = None;
        self.log("Extraction stopped by user".to_string(), LogLevel::Warning);
    }

    fn process_progress_updates(&mut self) {
        let mut updates_to_process = Vec::new();

        // Collect all updates first
        if let Some(rx) = &mut self.progress_rx {
            while let Ok(update) = rx.try_recv() {
                updates_to_process.push(update);
            }
        }

        // Process all collected updates
        for update in updates_to_process {
            match update {
                ProgressUpdate::Log(message, level) => {
                    self.log(message, level);
                }
                ProgressUpdate::Progress(progress) => {
                    self.progress = progress;
                }
                ProgressUpdate::Status(status) => {
                    self.status_message = status;
                }
                ProgressUpdate::Complete(table) => {
                    self.plc_table = table;
                    self.is_extracting = false;
                    self.progress_rx = None;
                    self.extraction_handle = None;
                    self.status_message = format!("Extraction complete - {} entries loaded", self.plc_table.entries.len());
                    self.progress = 0.0;
                    self.app_status = AppStatus::Completed;
                }
                ProgressUpdate::Error(error) => {
                    self.log(format!("💥 Error: {}", error), LogLevel::Error);
                    self.is_extracting = false;
                    self.progress_rx = None;
                    self.extraction_handle = None;
                    self.status_message = "❌ Extraction failed - check log for details".to_string();
                    self.progress = 0.0;
                    self.app_status = AppStatus::Error(error);
                    // Keep GUI open and responsive for user to see errors and retry
                }
                ProgressUpdate::StatusChange(status) => {
                    self.app_status = status;
                }
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| i.clone());

        // Handle keyboard shortcuts
        if input.modifiers.ctrl {
            if input.key_pressed(egui::Key::E) {
                // Ctrl+E: Extract
                if !self.is_extracting {
                    self.start_extraction();
                }
            } else if input.key_pressed(egui::Key::S) {
                // Ctrl+S: Save settings
                let _ = self.config.save();
            } else if input.key_pressed(egui::Key::L) {
                // Ctrl+L: Switch to Logs tab
                self.current_tab = AppTab::Logs;
            } else if input.key_pressed(egui::Key::R) {
                // Ctrl+R: Switch to Results tab
                self.current_tab = AppTab::Results;
            } else if input.key_pressed(egui::Key::Comma) {
                // Ctrl+, : Switch to Settings tab
                self.current_tab = AppTab::Settings;
            }
        }

        // Handle Escape key
        if input.key_pressed(egui::Key::Escape) {
            if self.is_extracting {
                // Cancel extraction
                if let Some(handle) = self.extraction_handle.take() {
                    handle.abort();
                }
                self.is_extracting = false;
                self.progress_rx = None;
                self.app_status = AppStatus::Ready;
                self.log("🚫 Extraction cancelled by user".to_string(), LogLevel::Warning);
            } else {
                // Switch to Main tab
                self.current_tab = AppTab::Main;
            }
        }

        // Handle F5 for refresh/restart
        if input.key_pressed(egui::Key::F5) {
            if !self.is_extracting {
                self.start_extraction();
            }
        }
    }
}

impl eframe::App for EviewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ctx);

        // Process progress updates from async extraction
        self.process_progress_updates();

        // Request repaint if extracting to ensure UI updates
        if self.is_extracting {
            ctx.request_repaint();
        }

        // Apply professional theme (light or dark)
        self.apply_professional_theme(ctx);

        // Get theme-based colors
        let (toolbar_bg, tab_bg, _content_bg) = self.get_panel_colors();
        let border_color = self.get_border_color();

        // Top toolbar with theme-based styling
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame {
                fill: toolbar_bg,
                shadow: egui::epaint::Shadow {
                    offset: egui::Vec2::new(0.0, 2.0),
                    blur: 8.0,
                    spread: 0.0,
                    color: match self.config.theme {
                        crate::config::Theme::Dark => egui::Color32::from_black_alpha(80),
                        crate::config::Theme::Light => egui::Color32::from_black_alpha(20),
                    },
                },
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.add_space(8.0);
                self.render_toolbar(ui);
                ui.add_space(8.0);
            });

        // Tab bar with theme-based styling
        egui::TopBottomPanel::top("tab_bar")
            .frame(egui::Frame {
                fill: tab_bg,
                stroke: egui::Stroke::new(1.0, border_color),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.render_tab_bar(ui);
                ui.add_space(4.0);
            });

        // Status bar with theme-based styling
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame {
                fill: toolbar_bg,
                stroke: egui::Stroke::new(1.0, border_color),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.render_status_bar(ui);
                ui.add_space(4.0);
            });

        // Render content based on current tab
        match self.current_tab {
            AppTab::Main => self.render_main_tab(ctx),
            AppTab::Logs => self.render_logs_tab(ctx),
            AppTab::Results => self.render_results_tab(ctx),
            AppTab::Settings => self.render_settings_tab(ctx),
        }

        // All UI is now handled through tabs - no separate dialogs needed
    }
}