//! Enable/disable mods by moving their folders in and out of the game's
//! content directories (JSGME-style), without deleting anything.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn disabled_dir(ac_root: &Path, kind: &str) -> PathBuf {
    ac_root.join("content").join(format!("{}_disabled", kind))
}

pub fn content_dir(ac_root: &Path, kind: &str) -> PathBuf {
    ac_root.join("content").join(kind)
}

pub fn list_disabled(ac_root: &Path, kind: &str) -> Vec<String> {
    let dir = disabled_dir(ac_root, kind);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut v: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

pub fn disable(ac_root: &Path, kind: &str, id: &str) -> Result<()> {
    let src = content_dir(ac_root, kind).join(id);
    let dst_dir = disabled_dir(ac_root, kind);
    fs::create_dir_all(&dst_dir)?;
    let dst = dst_dir.join(id);
    if dst.exists() {
        anyhow::bail!("already disabled: {}", id);
    }
    fs::rename(&src, &dst).with_context(|| format!("disabling {id}"))?;
    Ok(())
}

pub fn enable(ac_root: &Path, kind: &str, id: &str) -> Result<()> {
    let src = disabled_dir(ac_root, kind).join(id);
    let dst = content_dir(ac_root, kind).join(id);
    if dst.exists() {
        anyhow::bail!("already enabled: {}", id);
    }
    fs::rename(&src, &dst).with_context(|| format!("enabling {id}"))?;
    Ok(())
}
