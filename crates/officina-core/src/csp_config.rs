//! Downloading the official Custom Shaders Patch config pack
//! (ac-custom-shaders-patch/acc-extension-config) into an AC install's
//! extension/ folder.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::install;

const CSP_CONFIG_URL: &str = "https://github.com/ac-custom-shaders-patch/acc-extension-config/archive/refs/heads/master.zip";

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
        .args(["-sL", "-o", archive.to_str().unwrap(), CSP_CONFIG_URL])
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
        written += copy_dir(&src, &dst)?;
    }
    if written == 0 {
        bail!("no CSP config files found in downloaded archive");
    }
    Ok(written)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<usize> {
    fs::create_dir_all(dst)?;
    let mut written = 0usize;
    for entry in walkdir::WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = match path.strip_prefix(src) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(p) = target.parent() {
                fs::create_dir_all(p)?;
            }
            fs::copy(path, &target).with_context(|| format!("copying {}", path.display()))?;
            written += 1;
        }
    }
    Ok(written)
}