use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub push_to_talk_key: String,
    pub auto_punctuation: bool,
    pub model: String,
    pub start_with_windows: bool,
    pub input_device: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            push_to_talk_key: "RightCtrl".to_string(),
            auto_punctuation: true,
            model: "tiny.en".to_string(),
            start_with_windows: false,
            input_device: "Default".to_string(),
        }
    }
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct SettingsManager {
    config_path: PathBuf,
}

impl SettingsManager {
    pub fn new() -> Self {
        Self {
            config_path: resolve_config_path(),
        }
    }

    pub fn load(&self) -> Result<AppSettings, SettingsError> {
        if !self.config_path.exists() {
            let defaults = AppSettings::default();
            self.save(&defaults)?;
            return Ok(defaults);
        }

        let raw = fs::read_to_string(&self.config_path)?;
        let loaded = match serde_json::from_str::<AppSettings>(&raw) {
            Ok(settings) => settings,
            Err(_) => {
                self.backup_corrupt_config()?;
                let defaults = AppSettings::default();
                self.save(&defaults)?;
                return Ok(defaults);
            }
        };

        let sanitized = sanitize_settings(loaded);
        self.save(&sanitized)?;
        Ok(sanitized)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let payload = serde_json::to_string_pretty(&sanitize_settings(settings.clone()))?;
        fs::write(&self.config_path, payload)?;
        Ok(())
    }

    fn backup_corrupt_config(&self) -> Result<(), SettingsError> {
        if !self.config_path.exists() {
            return Ok(());
        }

        let backup = self.config_path.with_extension("corrupt.json");
        let _ = fs::rename(&self.config_path, backup);
        Ok(())
    }
}

fn resolve_config_path() -> PathBuf {
    let mut root = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    root.push("QuanVoice");
    root.push("config.json");
    root
}

fn sanitize_settings(mut settings: AppSettings) -> AppSettings {
    if settings.push_to_talk_key.trim().is_empty() {
        settings.push_to_talk_key = "RightCtrl".to_string();
    }
    if settings.model.trim().is_empty() {
        settings.model = "tiny.en".to_string();
    } else if settings.model != "tiny.en" {
        settings.model = "tiny.en".to_string();
    }
    if settings.input_device.trim().is_empty() {
        settings.input_device = "Default".to_string();
    }

    settings
}

pub mod text {
    pub fn clean_transcript(input: &str) -> String {
        input
            .replace('\r', "")
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }
}
