use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn check_latest(repo: &str) -> Result<String, String> {
    if repo.is_empty() {
        return Err("update_repo not configured".into());
    }
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let out = Command::new("curl")
        .args(["-sL", "-A", "officina-updater", &url])
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "curl failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("parse failed: {e}"))?;
    v["tag_name"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "no tag_name in release".into())
}

pub fn download_and_install(repo: &str) -> Result<String, String> {
    let tag = check_latest(repo)?;
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let out = Command::new("curl")
        .args(["-sL", "-A", "officina-updater", &url])
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("parse failed: {e}"))?;
    let asset_url = v["assets"]
        .as_array()
        .and_then(|a| {
            a.iter().find_map(|x| {
                let name = x["name"].as_str()?;
                if name.contains("linux") {
                    x["browser_download_url"].as_str()
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| "no linux asset found".to_string())?;

    let tmp = std::env::temp_dir().join("officina-update");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| format!("mkdir failed: {e}"))?;

    let archive = tmp.join("update.tar.gz");
    let status = Command::new("curl")
        .args(["-sL", "-o", archive.to_str().unwrap(), asset_url])
        .status()
        .map_err(|e| format!("curl download failed: {e}"))?;
    if !status.success() {
        return Err("download failed".into());
    }

    // extract
    let status = Command::new("tar")
        .args([
            "-xzf",
            archive.to_str().unwrap(),
            "-C",
            tmp.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("tar failed: {e}"))?;
    if !status.success() {
        return Err("extract failed".into());
    }

    // find binary
    let bin = find_binary(&tmp).ok_or_else(|| "binary not found in archive".to_string())?;
    let current = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let backup = current.with_extension("bak");
    let _ = fs::copy(&current, &backup);
    fs::copy(&bin, &current).map_err(|e| format!("replace failed: {e}"))?;

    Ok(format!(
        "Updated to {tag}. Restart the app. (backup: {})",
        backup.display()
    ))
}

fn find_binary(dir: &PathBuf) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() && p.file_name().map(|n| n == "officina").unwrap_or(false) {
                return Some(p);
            }
            if p.is_dir()
                && let Some(b) = find_binary(&p)
            {
                return Some(b);
            }
        }
    }
    None
}
