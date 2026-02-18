use std::{fs, path::PathBuf};

use crate::whisper;
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
        settings.model = whisper::fallback_model_id().to_string();
    } else if !whisper::is_phase_one_model(&settings.model) {
        settings.model = whisper::fallback_model_id().to_string();
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

#[cfg(test)]
mod tests {
    use super::{sanitize_settings, text, AppSettings};

    #[test]
    fn sanitize_restores_defaults_for_empty_values() {
        let input = AppSettings {
            push_to_talk_key: "  ".to_string(),
            auto_punctuation: false,
            model: "".to_string(),
            start_with_windows: false,
            input_device: " ".to_string(),
        };

        let out = sanitize_settings(input);
        assert_eq!(out.push_to_talk_key, "RightCtrl");
        assert_eq!(out.model, "tiny.en");
        assert_eq!(out.input_device, "Default");
    }

    #[test]
    fn sanitize_keeps_phase_one_model() {
        let input = AppSettings {
            model: "base.en".to_string(),
            ..AppSettings::default()
        };

        let out = sanitize_settings(input);
        assert_eq!(out.model, "base.en");
    }

    #[test]
    fn sanitize_falls_back_for_unsupported_model() {
        let input = AppSettings {
            model: "medium.en".to_string(),
            ..AppSettings::default()
        };

        let out = sanitize_settings(input);
        assert_eq!(out.model, "tiny.en");
    }

    #[test]
    fn clean_transcript_trims_lines_and_outer_whitespace() {
        let cleaned = text::clean_transcript("  hello  \r\nworld   \n\n");
        assert_eq!(cleaned, "hello\nworld");
    }
}
