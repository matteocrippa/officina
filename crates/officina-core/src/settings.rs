//! Application settings stored as TOML under ~/.config/officina/settings.toml.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::race::QuickDrive;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub ac_root: Option<PathBuf>,
    #[serde(default)]
    pub proton: Option<PathBuf>,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub update_repo: Option<String>,
    #[serde(default)]
    pub last_car: Option<String>,
    #[serde(default)]
    pub last_track: Option<String>,
    #[serde(default)]
    pub last_skin: Option<String>,
    #[serde(default)]
    pub last_session: Option<QuickDrive>,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Settings {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("officina").join("settings.toml"))
    }

    pub fn load() -> Self {
        if let Some(p) = Self::path() {
            if p.exists() {
                if let Ok(text) = fs::read_to_string(&p) {
                    if let Ok(s) = toml::from_str::<Settings>(&text) {
                        return s;
                    }
                }
            }
            // Migrate legacy settings.json if present.
            let legacy = p.with_file_name("settings.json");
            if legacy.exists() {
                if let Ok(text) = fs::read_to_string(&legacy) {
                    if let Ok(s) = serde_json::from_str::<Settings>(&text) {
                        let _ = s.save();
                        return s;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(p) = Self::path() {
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent)?;
            }
            let text = toml::to_string_pretty(self).context("serializing settings")?;
            fs::write(&p, text).with_context(|| format!("writing {}", p.display()))?;
        }
        Ok(())
    }

    /// Resolve the AC root: explicit setting, else auto-discover.
    pub fn ac_root(&self) -> Option<PathBuf> {
        self.ac_root.clone().or_else(crate::paths::find_ac_root)
    }
}
