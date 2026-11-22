//! Core domain logic for Assetto Corsa: paths, content scanning, INI, race.ini,
//! launching, mod installation. No UI dependencies.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const AC_APP_ID: u32 = 244210;

/// Rename an unreadable or unparseable file aside so a later save cannot
/// silently overwrite the user's original data.
pub fn quarantine_corrupt(path: &Path) -> Option<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".corrupt-{stamp}"));
    let target = PathBuf::from(name);
    fs::rename(path, &target).ok().map(|_| target)
}

pub fn command_exists(cmd: &str) -> bool {
    if cmd.contains(std::path::MAIN_SEPARATOR) {
        return Path::new(cmd).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(cmd).is_file()))
}

pub mod acd;
pub mod backup;
pub mod content;
pub mod csp_config;
pub mod events;
pub mod game_cfg;
pub mod github;
pub mod ini;
pub mod install;
pub mod json;
pub mod launch;
pub mod mods;
pub mod paths;
pub mod race;
pub mod results;
pub mod settings;
pub mod setup;
pub mod setups;
pub mod trash;
pub mod validate;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarantine_preserves_original() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("settings.toml");
        fs::write(&file, b"broken").unwrap();
        let moved = quarantine_corrupt(&file).unwrap();
        assert!(!file.exists());
        assert_eq!(fs::read(&moved).unwrap(), b"broken");
    }

    #[test]
    fn command_lookup_uses_path() {
        assert!(command_exists("sh"));
        assert!(!command_exists("officina-definitely-not-a-command"));
    }
}
