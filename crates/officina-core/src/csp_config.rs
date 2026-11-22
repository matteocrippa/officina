//! Downloading the official Custom Shaders Patch config pack
//! (ac-custom-shaders-patch/acc-extension-config) into an AC install's
//! extension/ folder.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::install;

const CSP_CONFIG_URL: &str =
    "https://github.com/ac-custom-shaders-patch/acc-extension-config/archive/refs/heads/master.zip";

/// Subdirectories of the archive (under `acc-extension-config-master/`) that
/// are copied into `<ac_root>/extension/`.
const SUBDIRS: &[&str] = &["config", "vao-patches", "backgrounds", "textures", "fonts"];

pub fn update_csp_configs(ac_root: &Path) -> Result<usize> {
    let extension_dir = ac_root.join("extension");
    fs::create_dir_all(&extension_dir).context("creating extension dir")?;

    let tmp = tempfile::tempdir().context("creating temp dir")?;
    let archive = tmp.path().join("csp-config.zip");

    // Download.
    let status = Command::new("curl")
        .args(["-fsSL", "--max-time", "120", "-o"])
        .arg(&archive)
        .arg(CSP_CONFIG_URL)
        .status()
        .context("running curl")?;
    if !status.success() {
        bail!("curl download failed");
    }

    // Extract whole archive.
    let extract_dir = tmp.path().join("extracted");
    fs::create_dir_all(&extract_dir).context("creating extract dir")?;
    install::extract_to_temp(&archive, &extract_dir).context("extracting archive")?;

    // The archive extracts to acc-extension-config-master/.
    let src_root = extract_dir.join("acc-extension-config-master");
    let src_root = if src_root.is_dir() {
        src_root
    } else {
        // Fallback: find the single top-level dir.
        extract_dir
            .read_dir()
            .context("reading extract dir")?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.is_dir())
            .unwrap_or_else(|| extract_dir.clone())
    };

    let mut written = 0usize;
    for sub in SUBDIRS {
        let src = src_root.join(sub);
        if !src.is_dir() {
            continue;
        }
        let dst = extension_dir.join(sub);
        written += install::copy_dir(&src, &dst)?;
    }
    if written == 0 {
        bail!("no CSP config files found in downloaded archive");
    }
    Ok(written)
}
