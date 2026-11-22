use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

const ARCHIVE_NAME: &str = "officina-linux-x86_64.tar.gz";
const CHECKSUM_NAME: &str = "officina-linux-x86_64.tar.gz.sha256";

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub struct Release {
    pub tag: String,
    pub archive_url: String,
    pub checksum_url: Option<String>,
}

pub fn check_latest(repo: &str) -> Result<Release, String> {
    let v = officina_core::github::release_json(repo).map_err(|e| e.to_string())?;
    let tag = v["tag_name"]
        .as_str()
        .ok_or_else(|| "no tag_name in release".to_string())?
        .to_string();
    let assets = v["assets"]
        .as_array()
        .ok_or_else(|| "no assets in release".to_string())?;
    let asset_url = |name: &str| {
        assets.iter().find_map(|a| {
            if a["name"].as_str() == Some(name) {
                a["browser_download_url"].as_str().map(str::to_string)
            } else {
                None
            }
        })
    };
    let archive_url = asset_url(ARCHIVE_NAME)
        .ok_or_else(|| format!("no {ARCHIVE_NAME} asset found in release"))?;
    Ok(Release {
        tag,
        archive_url,
        checksum_url: asset_url(CHECKSUM_NAME),
    })
}

fn version_parts(v: &str) -> Vec<u64> {
    let digits: String = v
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits
        .split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect()
}

pub fn update_available(latest: &str, current: &str) -> bool {
    let latest = version_parts(latest);
    let current = version_parts(current);
    if latest.is_empty() || current.is_empty() {
        return false;
    }
    for (a, b) in latest.iter().zip(current.iter()) {
        if a != b {
            return a > b;
        }
    }
    latest.len() > current.len()
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            "120",
            "-o",
        ])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|e| format!("curl download failed: {e}"))?;
    if !status.success() {
        return Err(format!("download failed for {url}"));
    }
    Ok(())
}

fn verify_checksum(archive: &Path, checksum: &Path) -> Result<(), String> {
    let text = fs::read_to_string(checksum).map_err(|e| format!("reading checksum: {e}"))?;
    let expected = text
        .split_whitespace()
        .next()
        .ok_or_else(|| "empty checksum file".to_string())?;
    let bytes = fs::read(archive).map_err(|e| format!("reading archive: {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = format!("{:x}", hasher.finalize());
    if !expected.eq_ignore_ascii_case(&actual) {
        return Err("SHA-256 checksum mismatch; refusing to update".into());
    }
    Ok(())
}

fn verify_listing(archive: &Path) -> Result<(), String> {
    let out = Command::new("tar")
        .arg("-tzf")
        .arg(archive)
        .output()
        .map_err(|e| format!("tar listing failed: {e}"))?;
    if !out.status.success() {
        return Err("cannot read update archive".into());
    }
    let listing = String::from_utf8_lossy(&out.stdout);
    let names: Vec<&str> = listing
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if names.len() != 1 || !matches!(names[0], "officia" | "./officia") {
        return Err("unexpected files in update archive".into());
    }
    Ok(())
}

fn set_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("setting executable bit: {e}"))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

pub fn download_and_install(release: &Release) -> Result<String, String> {
    if !update_available(&release.tag, current_version()) {
        return Err(format!(
            "{} is not newer than {}",
            release.tag,
            current_version()
        ));
    }
    let checksum_url = release
        .checksum_url
        .as_ref()
        .ok_or_else(|| "release has no checksum asset; refusing to update".to_string())?;

    let tmp = tempfile::tempdir().map_err(|e| format!("temp dir failed: {e}"))?;
    let archive = tmp.path().join(ARCHIVE_NAME);
    let checksum = tmp.path().join(CHECKSUM_NAME);
    download(&release.archive_url, &archive)?;
    download(checksum_url, &checksum)?;
    verify_checksum(&archive, &checksum)?;
    verify_listing(&archive)?;

    let extract_dir = tmp.path().join("extract");
    fs::create_dir_all(&extract_dir).map_err(|e| format!("mkdir failed: {e}"))?;
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(&extract_dir)
        .status()
        .map_err(|e| format!("tar extract failed: {e}"))?;
    if !status.success() {
        return Err("extract failed".into());
    }
    let binary = extract_dir.join("officia");
    if !fs::symlink_metadata(&binary)
        .map_err(|e| format!("checking extracted binary: {e}"))?
        .file_type()
        .is_file()
    {
        return Err("update archive did not contain a regular officina binary".into());
    }

    let current = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let parent = current
        .parent()
        .ok_or_else(|| "cannot determine executable directory".to_string())?;
    let staged: PathBuf = parent.join(format!(".officina-new-{}", std::process::id()));
    let backup = current.with_extension("bak");
    fs::copy(&binary, &staged).map_err(|e| format!("staging binary failed: {e}"))?;
    set_executable(&staged)?;

    let _ = fs::remove_file(&backup);
    fs::rename(&current, &backup).map_err(|e| format!("backing up binary failed: {e}"))?;
    if let Err(e) = fs::rename(&staged, &current) {
        let _ = fs::rename(&backup, &current);
        let _ = fs::remove_file(&staged);
        return Err(format!("replacing binary failed: {e}"));
    }

    Ok(format!(
        "Updated to {}. Restart the app. (backup: {})",
        release.tag,
        backup.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison() {
        assert!(update_available("v0.1.5", "0.1.4"));
        assert!(update_available("0.2.0", "0.1.9"));
        assert!(!update_available("v0.1.4", "0.1.4"));
        assert!(!update_available("v0.1.3", "0.1.4"));
        assert!(!update_available("garbage", "0.1.4"));
    }

    #[test]
    fn archive_listing_accepts_only_the_binary() {
        let dir = tempfile::tempdir().unwrap();
        let stage = dir.path().join("stage");
        fs::create_dir_all(&stage).unwrap();
        fs::write(stage.join("officia"), b"binary").unwrap();
        let archive = dir.path().join("update.tar.gz");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(&stage)
            .arg("officia")
            .status()
            .unwrap();
        assert!(status.success());
        assert!(verify_listing(&archive).is_ok());

        fs::write(stage.join("other"), b"unexpected").unwrap();
        let bad = dir.path().join("bad.tar.gz");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&bad)
            .arg("-C")
            .arg(&stage)
            .args(["officia", "other"])
            .status()
            .unwrap();
        assert!(status.success());
        assert!(verify_listing(&bad).is_err());
    }

    #[test]
    fn checksum_parsing() {
        let dir = tempfile::tempdir().unwrap();
        let data = b"officina test payload";
        let archive = dir.path().join("archive.bin");
        let checksum = dir.path().join("archive.sha256");
        fs::write(&archive, data).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(data);
        fs::write(&checksum, format!("{:x}  archive.bin\n", hasher.finalize())).unwrap();
        assert!(verify_checksum(&archive, &checksum).is_ok());
        fs::write(&archive, b"tampered").unwrap();
        assert!(verify_checksum(&archive, &checksum).is_err());
    }
}
