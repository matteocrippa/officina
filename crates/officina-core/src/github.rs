//! GitHub release lookups used to install community tools.

use std::process::Command;

use anyhow::{Context, Result, bail};

/// Fetches the latest release JSON for a repository.
pub fn release_json(repo: &str) -> Result<serde_json::Value> {
    if repo.is_empty() {
        bail!("update_repo not configured");
    }
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let out = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            "30",
            "-A",
            "officina-updater",
            &url,
        ])
        .output()
        .context("curl failed")?;
    if !out.status.success() {
        bail!("curl failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    serde_json::from_slice(&out.stdout).context("parse failed")
}

/// Resolves the browser download URL of the newest supported archive asset in
/// the latest release. Prefers `.zip`, then 7z/tar/rar formats.
pub fn latest_release_asset_url(repo: &str) -> Result<String> {
    let v = release_json(repo)?;
    let assets = v["assets"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("no assets in latest release"))?;
    for suffix in [".zip", ".7z", ".tar.gz", ".tgz", ".tar", ".rar"] {
        if let Some(url) = assets.iter().find_map(|x| {
            let name = x["name"].as_str()?;
            if name.to_lowercase().ends_with(suffix) {
                x["browser_download_url"].as_str()
            } else {
                None
            }
        }) {
            return Ok(url.to_string());
        }
    }
    bail!("no supported archive asset found in latest release")
}
