//! Move files/folders to the freedesktop Trash instead of deleting them.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn move_to_trash(path: &Path) -> std::io::Result<PathBuf> {
    // Try gio first (GNOME/freedesktop).
    if crate::command_exists("gio") {
        let status = std::process::Command::new("gio")
            .arg("trash")
            .arg(path)
            .status();
        if let Ok(s) = status
            && s.success()
        {
            return Ok(PathBuf::from("~/.local/share/Trash"));
        }
    }
    if crate::command_exists("trash-put") {
        let status = std::process::Command::new("trash-put").arg(path).status();
        if let Ok(s) = status
            && s.success()
        {
            return Ok(PathBuf::from("~/.local/share/Trash"));
        }
    }
    // Manual freedesktop trash fallback.
    let home = dirs::home_dir().ok_or_else(|| std::io::Error::other("no home"))?;
    let trash_files = home.join(".local/share/Trash/files");
    let trash_info = home.join(".local/share/Trash/info");
    fs::create_dir_all(&trash_files)?;
    fs::create_dir_all(&trash_info)?;

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "item".into());
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let dest_name = format!("{}_{}", name, ts);
    let dest = trash_files.join(&dest_name);

    // move (rename) — fall back to copy+delete for cross-device
    if fs::rename(path, &dest).is_err() {
        // copy recursively
        copy_recursive(path, &dest)?;
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }

    // trashinfo
    let deletion_date = chrono_like_now();
    let info_path = trash_info.join(format!("{}.trashinfo", dest_name));
    let info = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        path.display(),
        deletion_date
    );
    fs::write(info_path, info)?;
    Ok(dest)
}

fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        crate::install::copy_dir(src, dst).map_err(std::io::Error::other)?;
    } else {
        if let Some(p) = dst.parent() {
            fs::create_dir_all(p)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

fn chrono_like_now() -> String {
    // Produce YYYY-MM-DDThh:mm:ss format (approx, no tz). Use SystemTime.
    // We'll just use a simple UTC-ish timestamp via date command if available.
    if let Ok(out) = std::process::Command::new("date")
        .arg("+%Y-%m-%dT%H:%M:%S")
        .output()
        && out.status.success()
    {
        return String::from_utf8_lossy(&out.stdout).trim().to_string();
    }
    "1970-01-01T00:00:00".to_string()
}
