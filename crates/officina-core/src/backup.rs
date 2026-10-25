//! Snapshots of AC's `Documents/Assetto Corsa/cfg` for easy restore.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

pub fn backup_root() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("officina").join("backups"))
}

pub fn create_backup(ac_root: &Path) -> Result<PathBuf> {
    let src = crate::paths::documents_ac_dir(ac_root)
        .map(|d| d.join("cfg"))
        .ok_or_else(|| anyhow::anyhow!("Documents/Assetto Corsa not found"))?;
    let root = backup_root().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    fs::create_dir_all(&root)?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let dest = root.join(ts.to_string());
    fs::create_dir_all(&dest)?;
    if src.is_dir() {
        copy_dir(&src, &dest)?;
    }
    Ok(dest)
}

pub fn list_backups() -> Vec<PathBuf> {
    let Some(root) = backup_root() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|de| de.path()))
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    v.reverse();
    v
}

pub fn restore_backup(backup_path: &Path, ac_root: &Path) -> Result<()> {
    let dst = crate::paths::documents_ac_dir(ac_root)
        .map(|d| d.join("cfg"))
        .ok_or_else(|| anyhow::anyhow!("Documents/Assetto Corsa not found"))?;
    fs::create_dir_all(&dst)?;
    copy_dir(backup_path, &dst)?;
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let sp = e.path();
        let dp = dst.join(e.file_name());
        if sp.is_dir() {
            copy_dir(&sp, &dp)?;
        } else {
            fs::copy(&sp, &dp).with_context(|| format!("copying {}", sp.display()))?;
        }
    }
    Ok(())
}
