//! Installing cars and tracks from mod archives.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Car,
    Track,
}

impl Kind {
    pub fn folder(self) -> &'static str {
        match self {
            Kind::Car => "cars",
            Kind::Track => "tracks",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub kind: Kind,
    pub id: String,
    pub prefix: String,
    pub target: PathBuf,
    pub exists: bool,
}

fn parent(p: &str) -> &str {
    p.rsplit_once('/').map(|(a, _)| a).unwrap_or("")
}

fn last(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}

fn detect(names: &[String], archive_stem: &str) -> BTreeSet<(Kind, String, String)> {
    let mut roots = BTreeSet::new();
    for name in names {
        let lower = name.to_lowercase();
        let file = last(&lower);
        let dir = parent(name);
        let root = match file {
            "ui_car.json" if last(&dir.to_lowercase()) == "ui" => Some((Kind::Car, parent(dir))),
            "data.acd" => Some((Kind::Car, dir)),
            "ui_track.json" => {
                let d = dir.to_lowercase();
                if last(&d) == "ui" {
                    Some((Kind::Track, parent(dir)))
                } else if last(parent(&d)) == "ui" {
                    Some((Kind::Track, parent(parent(dir))))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some((kind, prefix)) = root {
            let id = if prefix.is_empty() {
                archive_stem
            } else {
                last(prefix)
            };
            roots.insert((kind, id.to_owned(), prefix.to_owned()));
        }
    }
    let all: Vec<_> = roots.iter().cloned().collect();
    roots.retain(|(_, _, p)| {
        !all.iter().any(|(_, _, other)| {
            other != p && !other.is_empty() && p.starts_with(&format!("{other}/"))
        })
    });
    roots
}

fn archive_name(name: &str) -> Result<String> {
    let name = name.replace('\\', "/");
    if std::path::Path::new(&name).components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        bail!("unsafe archive path: {name}");
    }
    Ok(name)
}

fn archive_stem(archive: &Path) -> String {
    archive
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn list_entries(archive: &Path) -> Result<Vec<String>> {
    let ext = archive
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "zip" => {
            let mut zip = zip::ZipArchive::new(File::open(archive).context("opening zip")?)
                .context("not a zip archive")?;
            let mut names = Vec::new();
            for i in 0..zip.len() {
                names.push(archive_name(zip.by_index(i)?.name())?);
            }
            Ok(names)
        }
        "7z" => {
            let arc = sevenz_rust::Archive::open(archive).context("opening 7z")?;
            Ok(arc.files.iter().map(|e| archive_name(&e.name.replace('\\', "/"))).collect::<Result<Vec<_>>>()?)
        }
        "rar" => list_rar(archive),
        "tar" | "tgz" | "gz" => list_tar(archive),
        _ => {
            // Try zip as fallback.
            let mut zip = zip::ZipArchive::new(File::open(archive).context("opening archive")?)
                .context("unsupported archive type")?;
            let mut names = Vec::new();
            for i in 0..zip.len() {
                names.push(archive_name(zip.by_index(i)?.name())?);
            }
            Ok(names)
        }
    }
}

fn list_rar(archive: &Path) -> Result<Vec<String>> {
    // Prefer bsdtar, then 7z.
    if which("bsdtar") {
        let out = Command::new("bsdtar").arg("-tf").arg(archive).output().context("running bsdtar")?;
        if !out.status.success() {
            bail!("bsdtar failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        return Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(archive_name)
            .collect::<Result<Vec<_>>>()?);
    }
    if which("7z") {
        let out = Command::new("7z").arg("l").arg("-slt").arg(archive).output().context("running 7z")?;
        if !out.status.success() {
            bail!("7z failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        let mut names = Vec::new();
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if let Some(rest) = line.strip_prefix("Path = ") {
                let n = rest.trim();
                if !n.is_empty() {
                    names.push(archive_name(&n.replace('\\', "/"))?);
                }
            }
        }
        return Ok(names);
    }
    bail!("no tool to read .rar archives: install bsdtar or p7zip")
}

fn list_tar(archive: &Path) -> Result<Vec<String>> {
    if which("tar") {
        let out = Command::new("tar").arg("-tf").arg(archive).output().context("running tar")?;
        if !out.status.success() {
            bail!("tar failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        return Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(archive_name)
            .collect::<Result<Vec<_>>>()?);
    }
    bail!("tar not found")
}

fn which(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn plan(archive: &Path, ac_root: &Path) -> Result<Vec<Item>> {
    let names = list_entries(archive)?;
    let stem = archive_stem(archive);
    let items: Vec<Item> = detect(&names, &stem)
        .into_iter()
        .map(|(kind, id, prefix)| {
            let mut components = Path::new(&id).components();
            if !matches!(components.next(), Some(std::path::Component::Normal(_)))
                || components.next().is_some()
            {
                bail!("unsafe content identifier: {id}");
            }
            let target = ac_root.join("content").join(kind.folder()).join(&id);
            Ok(Item {
                exists: target.exists(),
                kind,
                id,
                prefix,
                target,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if items.is_empty() {
        bail!("no cars or tracks found in {}", archive.display());
    }
    Ok(items)
}

/// Extracts the planned items. Existing files are overwritten.
pub fn install(archive: &Path, items: &[Item]) -> Result<usize> {
    let ext = archive
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext == "zip" {
        return install_zip(archive, items);
    }
    // For other formats, extract whole archive to temp then copy matching prefixes.
    let tmp = tempfile::tempdir().context("creating temp dir")?;
    extract_to_temp(archive, tmp.path())?;
    let mut written = 0;
    for item in items {
        let src_root = if item.prefix.is_empty() {
            tmp.path().to_path_buf()
        } else {
            tmp.path().join(&item.prefix)
        };
        if !src_root.is_dir() {
            continue;
        }
        written += copy_dir(&src_root, &item.target)?;
    }
    Ok(written)
}

fn install_zip(archive: &Path, items: &[Item]) -> Result<usize> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    let mut written = 0;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let name = archive_name(f.name())?;
        for item in items {
            let rel = if item.prefix.is_empty() {
                Some(name.as_str())
            } else {
                name.strip_prefix(&item.prefix)
                    .and_then(|r| r.strip_prefix('/'))
            };
            let Some(rel) = rel.filter(|r| !r.is_empty()) else {
                continue;
            };
            let out = item.target.join(rel);
            let mut checked = item.target.clone();
            for component in Path::new(rel).components() {
                checked.push(component);
                if fs::symlink_metadata(&checked)
                    .is_ok_and(|m| m.file_type().is_symlink())
                {
                    bail!("refusing to overwrite symlink destination: {}", checked.display());
                }
            }
            if f.is_dir() {
                fs::create_dir_all(&out)?;
            } else {
                if let Some(p) = out.parent() {
                    fs::create_dir_all(p)?;
                }
                io::copy(&mut f, &mut File::create(&out)?)?;
                written += 1;
            }
            break;
        }
    }
    Ok(written)
}

fn extract_to_temp(archive: &Path, dest: &Path) -> Result<()> {
    let ext = archive
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "7z" => {
            sevenz_rust::decompress_file(archive, dest).map_err(|e| anyhow::anyhow!("7z extract failed: {e}"))?;
            Ok(())
        }
        "rar" => {
            if which("bsdtar") {
                let status = Command::new("bsdtar").arg("-xf").arg(archive).arg("-C").arg(dest).status().context("running bsdtar")?;
                if !status.success() {
                    bail!("bsdtar extraction failed");
                }
                return Ok(());
            }
            if which("7z") {
                let status = Command::new("7z").arg("x").arg(format!("-o{}", dest.display())).arg(archive).arg("-y").status().context("running 7z")?;
                if !status.success() {
                    bail!("7z extraction failed");
                }
                return Ok(());
            }
            if which("unrar") {
                let status = Command::new("unrar").arg("x").arg("-y").arg(archive).arg(dest).status().context("running unrar")?;
                if !status.success() {
                    bail!("unrar extraction failed");
                }
                return Ok(());
            }
            bail!("no tool to extract .rar archives: install bsdtar, p7zip or unrar")
        }
        "tar" | "tgz" | "gz" => {
            if which("tar") {
                let status = Command::new("tar").arg("-xf").arg(archive).arg("-C").arg(dest).status().context("running tar")?;
                if !status.success() {
                    bail!("tar extraction failed");
                }
                return Ok(());
            }
            bail!("tar not found")
        }
        _ => bail!("unsupported archive type: {ext}"),
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<usize> {
    fs::create_dir_all(dst)?;
    let mut written = 0;
    for entry in walkdir::WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = path.strip_prefix(src).unwrap();
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
            fs::copy(path, &target)?;
            written += 1;
        }
    }
    Ok(written)
}