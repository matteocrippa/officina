//! GitHub release lookups used to install community tools.

use std::process::Command;

use anyhow::{Context, Result, bail};

/// Resolves the browser download URL of the latest `.zip` release asset.
pub fn latest_release_asset_url(repo: &str) -> Result<String> {
    if repo.is_empty() {
        bail!("update_repo not configured");
    }
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let out = Command::new("curl")
        .args(["-sL", "-A", "officina-updater", &url])
        .output()
        .context("curl failed")?;
    if !out.status.success() {
        bail!("curl failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).context("parse failed")?;
    let asset_url = v["assets"]
        .as_array()
        .and_then(|a| {
            a.iter().find_map(|x| {
                let name = x["name"].as_str()?;
                if name.to_lowercase().ends_with(".zip") {
                    x["browser_download_url"].as_str()
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| anyhow::anyhow!("no .zip asset found in latest release"))?;
    Ok(asset_url.to_string())
}
