use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub email: String,
    pub password: String,
    pub project_number: String,
    pub headless_mode: bool,
    pub debug_mode: bool, // Keep browser open for debugging
    pub export_excel: bool,
    pub export_csv: bool,
    pub export_json: bool,
    pub theme: Theme,
    pub last_export_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            email: String::new(),
            password: String::new(),
            project_number: String::new(),
            headless_mode: true,
            debug_mode: false, // Default to false for production
            export_excel: true,
            export_csv: false,
            export_json: false,
            theme: Theme::Dark,
            last_export_path: None,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Self = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "eplan", "eview-scraper")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        Ok(proj_dirs.config_dir().join("config.json"))
    }

    pub fn clear_password(&mut self) {
        self.password.clear();
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.email.is_empty() {
            errors.push("Email is required".to_string());
        }

        if self.password.is_empty() {
            errors.push("Password is required".to_string());
        }

        if self.project_number.is_empty() {
            errors.push("Project number is required".to_string());
        }

        if !self.export_excel && !self.export_csv && !self.export_json {
            errors.push("At least one export format must be selected".to_string());
        }

        errors
    }
}