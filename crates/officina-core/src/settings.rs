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
    #[serde(default)]
    pub auto_update_csp: bool,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Settings {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("officina").join("settings.toml"))
    }

    /// Loads settings and reports any corrupt file that was preserved instead
    /// of being overwritten.
    pub fn load() -> (Self, Option<String>) {
        let mut warning = None;
        if let Some(p) = Self::path() {
            if p.exists() {
                match fs::read_to_string(&p)
                    .ok()
                    .and_then(|text| toml::from_str::<Settings>(&text).ok())
                {
                    Some(s) => return (s, None),
                    None => {
                        warning = Some(match crate::quarantine_corrupt(&p) {
                            Some(q) => format!(
                                "Corrupt settings file moved to {}; defaults loaded.",
                                q.display()
                            ),
                            None => {
                                "Corrupt settings file could not be preserved; defaults loaded."
                                    .into()
                            }
                        });
                    }
                }
            }
            // Migrate legacy settings.json if present.
            let legacy = p.with_file_name("settings.json");
            if legacy.exists() {
                match fs::read_to_string(&legacy)
                    .ok()
                    .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
                {
                    Some(s) => {
                        if let Err(e) = s.save() {
                            warning = Some(format!(
                                "Legacy settings migrated in memory but could not be saved: {e:#}"
                            ));
                        }
                        return (s, warning);
                    }
                    None => {
                        if let Some(q) = crate::quarantine_corrupt(&legacy) {
                            warning = Some(format!(
                                "Corrupt legacy settings moved to {}; defaults loaded.",
                                q.display()
                            ));
                        }
                    }
                }
            }
        }
        (Self::default(), warning)
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
