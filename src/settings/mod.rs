use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    English,
    Chinese,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: Theme,
    pub language: Language,
    pub max_history_count: usize,
    pub global_shortcut: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            language: Language::English,
            max_history_count: 100,
            global_shortcut: "Cmd+Shift+V".to_string(),
        }
    }
}

impl Settings {
    /// Load settings from file or create default
    pub fn load(path: PathBuf) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            let settings = Self::default();
            settings.save(path)?;
            Ok(settings)
        }
    }

    /// Save settings to file
    pub fn save(&self, path: PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Validate settings
    pub fn validate(&self) -> Result<()> {
        if self.max_history_count == 0 {
            anyhow::bail!("Max history count must be greater than 0");
        }
        if self.max_history_count > 10000 {
            anyhow::bail!("Max history count cannot exceed 10000");
        }
        Ok(())
    }
}
