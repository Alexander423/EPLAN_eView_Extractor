use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::crypto::{EncryptedPassword, PasswordCrypto};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub email: String,
    #[serde(skip)] // Don't serialize the plaintext password
    password_plaintext: String,
    #[serde(rename = "password")] // Serialize encrypted password as "password" field
    password_encrypted: Option<String>, // JSON-serialized EncryptedPassword
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
            password_plaintext: String::new(),
            password_encrypted: None,
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
            let mut config: Self = serde_json::from_str(&content)?;

            // Load and decrypt password if it exists
            config.load_password()?;

            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Load encrypted password and decrypt it to plaintext
    fn load_password(&mut self) -> Result<()> {
        if let Some(encrypted_json) = &self.password_encrypted {
            if encrypted_json.is_empty() {
                self.password_plaintext = String::new();
                return Ok(());
            }

            // Check if this looks like encrypted data (JSON) or legacy plaintext
            if PasswordCrypto::is_likely_encrypted(encrypted_json) {
                // New encrypted format
                let encrypted: EncryptedPassword = serde_json::from_str(encrypted_json)
                    .map_err(|e| anyhow::anyhow!("Failed to parse encrypted password: {}", e))?;

                self.password_plaintext = PasswordCrypto::decrypt_password(&encrypted)
                    .unwrap_or_else(|e| {
                        eprintln!("Warning: Failed to decrypt password: {}. Using empty password.", e);
                        String::new()
                    });
            } else {
                // Legacy plaintext format - migrate it
                self.password_plaintext = encrypted_json.clone();
                self.encrypt_and_save_password()?;
            }
        } else {
            self.password_plaintext = String::new();
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create a copy for saving with encrypted password
        let mut config_to_save = self.clone();
        config_to_save.encrypt_password_for_save()?;

        let content = serde_json::to_string_pretty(&config_to_save)?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    /// Encrypt the plaintext password for JSON serialization
    fn encrypt_password_for_save(&mut self) -> Result<()> {
        if !self.password_plaintext.is_empty() {
            let encrypted = PasswordCrypto::encrypt_password(&self.password_plaintext)?;
            self.password_encrypted = Some(serde_json::to_string(&encrypted)?);
        } else {
            self.password_encrypted = None;
        }
        Ok(())
    }

    /// Encrypt and immediately save the password (for migration)
    fn encrypt_and_save_password(&mut self) -> Result<()> {
        self.encrypt_password_for_save()?;
        // Note: We don't call save() here to avoid recursion, migration happens during load
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "eplan", "eview-scraper")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        Ok(proj_dirs.config_dir().join("config.json"))
    }

    /// Get the plaintext password (for UI and authentication)
    pub fn password(&self) -> &str {
        &self.password_plaintext
    }

    /// Set the plaintext password (UI calls this)
    pub fn set_password(&mut self, password: String) {
        self.password_plaintext = password;
    }

    /// Clear the password
    pub fn clear_password(&mut self) {
        self.password_plaintext.clear();
        self.password_encrypted = None;
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.email.is_empty() {
            errors.push("Email is required".to_string());
        }

        if self.password_plaintext.is_empty() {
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