//! Installing cars, tracks and community tools from mod archives.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Car,
    Track,
    Generic,
    Tool,
}

impl Kind {
    pub fn folder(self) -> &'static str {
        match self {
            Kind::Car => "cars",
            Kind::Track => "tracks",
            Kind::Generic => "",
            Kind::Tool => "apps",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Kind::Car => "Car",
            Kind::Track => "Track",
            Kind::Generic => "Generic/Patch",
            Kind::Tool => "Tool",
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
    ensure!(!name.contains('\0'), "unsafe archive path: {name:?}");
    let name = name.replace('\\', "/");
    let safe = std::path::Path::new(&name).components().all(|component| {
        matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    });
    ensure!(safe, "unsafe archive path: {name}");
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
            Ok(arc
                .files
                .iter()
                .map(|e| archive_name(&e.name.replace('\\', "/")))
                .collect::<Result<Vec<_>>>()?)
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
    if crate::command_exists("bsdtar") {
        let out = Command::new("bsdtar")
            .arg("-tf")
            .arg(archive)
            .output()
            .context("running bsdtar")?;
        if !out.status.success() {
            bail!("bsdtar failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        return String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(archive_name)
            .collect::<Result<Vec<_>>>();
    }
    if crate::command_exists("7z") {
        let out = Command::new("7z")
            .arg("l")
            .arg("-slt")
            .arg(archive)
            .output()
            .context("running 7z")?;
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
    if crate::command_exists("tar") {
        let out = Command::new("tar")
            .arg("-tf")
            .arg(archive)
            .output()
            .context("running tar")?;
        if !out.status.success() {
            bail!("tar failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        return String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(archive_name)
            .collect::<Result<Vec<_>>>();
    }
    bail!("tar not found")
}

pub fn plan(archive: &Path, ac_root: &Path) -> Result<Vec<Item>> {
    let names = list_entries(archive)?;
    let stem = archive_stem(archive);
    let mut items: Vec<Item> = detect(&names, &stem)
        .into_iter()
        .map(|(kind, id, prefix)| {
            let mut components = Path::new(&id).components();
            if !matches!(components.next(), Some(std::path::Component::Normal(_)))
                || components.next().is_some()
            {
                bail!("unsafe content identifier: {id}");
            }
            let target = if kind == Kind::Generic {
                ac_root.to_path_buf()
            } else {
                ac_root.join("content").join(kind.folder()).join(&id)
            };
            Ok(Item {
                exists: target.exists(),
                kind,
                id,
                prefix,
                target,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    // Tools can be bundled alongside cars or tracks, so always include them.
    items.extend(tool_items(&names, ac_root)?);
    if items.is_empty() {
        // Fall back to generic extraction into the AC root (CSP, Sol, Pure, ...).
        return Ok(vec![Item {
            exists: ac_root.exists(),
            kind: Kind::Generic,
            id: "patch".to_string(),
            prefix: String::new(),
            target: ac_root.to_path_buf(),
        }]);
    }
    Ok(items)
}

/// Plans installs for community tools (e.g. Paintshop) found under `apps/...`
/// in the archive: one item per app folder, installed into the AC root.
fn tool_items(names: &[String], ac_root: &Path) -> Result<Vec<Item>> {
    let mut roots = BTreeSet::<String>::new();
    for name in names {
        let segs: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
        if let Some(pos) = segs.iter().position(|s| s.eq_ignore_ascii_case("apps")) {
            let mut root_segs = segs[pos..].to_vec();
            if !name.ends_with('/') && root_segs.len() > 1 {
                // File entries install into their containing folder.
                root_segs.pop();
            }
            if root_segs.is_empty() {
                continue;
            }
            roots.insert(root_segs.join("/"));
        }
    }
    let mut items = Vec::new();
    for root in roots {
        let id = root.rsplit('/').next().unwrap_or(&root).to_string();
        let target = ac_root.join(&root);
        items.push(Item {
            kind: Kind::Tool,
            id,
            prefix: root.clone(),
            exists: target.exists(),
            target,
        });
    }
    Ok(items)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InstallOptions {
    pub allow_overwrite: bool,
    pub allow_root_overwrite: bool,
}

/// Extracts the planned items with conservative defaults: no overwrites and no
/// writes directly into the AC root.
pub fn install(archive: &Path, items: &[Item]) -> Result<usize> {
    install_with_options(archive, items, InstallOptions::default())
}

pub fn install_with_options(
    archive: &Path,
    items: &[Item],
    options: InstallOptions,
) -> Result<usize> {
    if !options.allow_root_overwrite && items.iter().any(|i| i.kind == Kind::Generic) {
        bail!("refusing to install a generic patch into the AC root without confirmation");
    }
    if !options.allow_overwrite && items.iter().any(|i| i.exists) {
        bail!("refusing to overwrite existing content without confirmation");
    }
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

/// Installs a community tool from a GitHub repo or a direct archive URL into
/// the AC root (`apps/...`).
pub fn install_tool(ac_root: &Path, source: &str) -> Result<usize> {
    let url = if source.starts_with("http://") || source.starts_with("https://") {
        source.to_string()
    } else {
        crate::github::latest_release_asset_url(source)?
    };
    install_tool_from_url(ac_root, &url).context("downloading tool")
}

fn archive_name_from_url(url: &str) -> Result<String> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let name = path.rsplit('/').next().unwrap_or("");
    ensure!(!name.is_empty(), "cannot determine archive name from URL");
    let lower = name.to_lowercase();
    ensure!(
        [".zip", ".7z", ".rar", ".tar.gz", ".tgz", ".tar"]
            .iter()
            .any(|ext| lower.ends_with(ext)),
        "URL does not point to a supported archive: {name}"
    );
    archive_name(name)
}

/// Installs a community tool from a direct archive URL into the AC root
/// (`apps/...`).
fn install_tool_from_url(ac_root: &Path, url: &str) -> Result<usize> {
    let tmp = tempfile::tempdir().context("creating temp dir")?;
    let archive = tmp.path().join(archive_name_from_url(url)?);
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
        .arg(&archive)
        .arg(url)
        .status()
        .context("running curl")?;
    if !status.success() {
        bail!("curl download failed");
    }
    let items = plan(&archive, ac_root).context("planning tool install")?;
    let written = install_with_options(
        &archive,
        &items,
        InstallOptions {
            allow_overwrite: true,
            ..InstallOptions::default()
        },
    )
    .context("installing tool")?;
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
            ensure_no_symlinks(&item.target, Path::new(rel))?;
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

pub fn extract_to_temp(archive: &Path, dest: &Path) -> Result<()> {
    list_entries(archive).context("validating archive entries")?;
    let ext = archive
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "zip" => {
            let mut zip = zip::ZipArchive::new(File::open(archive).context("opening zip")?)
                .context("reading zip")?;
            for i in 0..zip.len() {
                let mut f = zip.by_index(i)?;
                let name = archive_name(f.name())?;
                let out = dest.join(&name);
                if f.is_dir() {
                    fs::create_dir_all(&out)?;
                } else {
                    if let Some(p) = out.parent() {
                        fs::create_dir_all(p)?;
                    }
                    io::copy(&mut f, &mut File::create(&out)?)?;
                }
            }
            Ok(())
        }
        "7z" => sevenz_rust::decompress_file(archive, dest)
            .map_err(|e| anyhow::anyhow!("7z extract failed: {e}")),
        "rar" => {
            if crate::command_exists("bsdtar") {
                let status = Command::new("bsdtar")
                    .arg("-xf")
                    .arg(archive)
                    .arg("-C")
                    .arg(dest)
                    .status()
                    .context("running bsdtar")?;
                if !status.success() {
                    bail!("bsdtar extraction failed");
                }
                Ok(())
            } else if crate::command_exists("7z") {
                let status = Command::new("7z")
                    .arg("x")
                    .arg(format!("-o{}", dest.display()))
                    .arg(archive)
                    .arg("-y")
                    .status()
                    .context("running 7z")?;
                if !status.success() {
                    bail!("7z extraction failed");
                }
                Ok(())
            } else if crate::command_exists("unrar") {
                let status = Command::new("unrar")
                    .arg("x")
                    .arg("-y")
                    .arg(archive)
                    .arg(dest)
                    .status()
                    .context("running unrar")?;
                if !status.success() {
                    bail!("unrar extraction failed");
                }
                Ok(())
            } else {
                bail!("no tool to extract .rar archives: install bsdtar, p7zip or unrar")
            }
        }
        "tar" | "tgz" | "gz" => {
            if !crate::command_exists("tar") {
                bail!("tar not found");
            }
            let status = Command::new("tar")
                .arg("-xf")
                .arg(archive)
                .arg("-C")
                .arg(dest)
                .status()
                .context("running tar")?;
            if !status.success() {
                bail!("tar extraction failed");
            }
            Ok(())
        }
        _ => bail!("unsupported archive type: {ext}"),
    }?;
    reject_unsafe_tree(dest)
}

fn reject_unsafe_tree(root: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.with_context(|| format!("checking {}", root.display()))?;
        if entry.file_type().is_symlink() {
            bail!("refusing archive symlink: {}", entry.path().display());
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .context("entry outside extraction directory")?;
        if !rel.as_os_str().is_empty() {
            archive_name(&rel.to_string_lossy().replace('\\', "/"))?;
        }
    }
    Ok(())
}

fn ensure_no_symlinks(base: &Path, rel: &Path) -> Result<()> {
    let mut checked = base.to_path_buf();
    for component in rel.components() {
        checked.push(component);
        if fs::symlink_metadata(&checked).is_ok_and(|m| m.file_type().is_symlink()) {
            bail!(
                "refusing to overwrite symlink destination: {}",
                checked.display()
            );
        }
    }
    Ok(())
}

/// Recursively copies `src` into `dst`, validating relative paths and refusing
/// symlinks on either side.
pub fn copy_dir(src: &Path, dst: &Path) -> Result<usize> {
    fs::create_dir_all(dst).with_context(|| format!("creating {}", dst.display()))?;
    let mut written = 0;
    for entry in walkdir::WalkDir::new(src).follow_links(false) {
        let entry = entry.with_context(|| format!("walking {}", src.display()))?;
        let path = entry.path();
        let rel = path
            .strip_prefix(src)
            .context("entry outside source tree")?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        archive_name(&rel.to_string_lossy().replace('\\', "/"))?;
        if entry.file_type().is_symlink() {
            bail!("refusing to copy symlink: {}", path.display());
        }
        ensure_no_symlinks(dst, rel)?;
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target).with_context(|| format!("copying {}", path.display()))?;
            written += 1;
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_name_rejects_unsafe_paths() {
        assert!(archive_name("../evil").is_err());
        assert!(archive_name("/etc/passwd").is_err());
        assert!(archive_name("a/../../b").is_err());
        assert!(archive_name("bad\0name").is_err());
        assert!(archive_name("cars/mycar/ui/ui_car.json").is_ok());
    }

    #[test]
    fn extract_rejects_traversal_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("evil.zip");
        {
            let file = File::create(&archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("../evil.txt", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.finish().unwrap();
        }
        let dest = tmp.path().join("out");
        assert!(extract_to_temp(&archive, &dest).is_err());
        assert!(!tmp.path().join("evil.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn copy_dir_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        symlink("/etc/passwd", src.join("leak")).unwrap();
        assert!(copy_dir(&src, &tmp.path().join("dst")).is_err());
    }

    #[test]
    fn tool_url_keeps_archive_extension() {
        assert_eq!(
            archive_name_from_url("https://example.com/tool.tar.gz?download=1").unwrap(),
            "tool.tar.gz"
        );
        assert_eq!(
            archive_name_from_url("https://example.com/tool.7z").unwrap(),
            "tool.7z"
        );
        assert!(archive_name_from_url("https://example.com/tool.exe").is_err());
    }

    #[test]
    fn plan_keeps_tools_alongside_cars() {
        let tmp = tempfile::tempdir().unwrap();
        let ac_root = tmp.path().join("ac");
        std::fs::create_dir_all(ac_root.join("content/cars")).unwrap();
        let archive = tmp.path().join("bundle.zip");
        {
            let file = std::fs::File::create(&archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("cars/mycar/data.acd", options).unwrap();
            zip.start_file("apps/lua/Tool/tool.lua", options).unwrap();
            zip.finish().unwrap();
        }
        let items = plan(&archive, &ac_root).unwrap();
        assert!(items.iter().any(|i| i.kind == Kind::Car));
        assert!(items.iter().any(|i| i.kind == Kind::Tool));
    }

    #[test]
    fn install_refuses_unconfirmed_generic_and_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("patch.zip");
        std::fs::write(&archive, b"not a real zip").unwrap();
        let ac_root = tmp.path().join("ac");
        let generic = Item {
            kind: Kind::Generic,
            id: "patch".into(),
            prefix: String::new(),
            target: ac_root.clone(),
            exists: true,
        };
        assert!(install(&archive, std::slice::from_ref(&generic)).is_err());
        assert!(
            install_with_options(
                &archive,
                &[generic],
                InstallOptions {
                    allow_overwrite: true,
                    allow_root_overwrite: true,
                }
            )
            .is_err()
        );
    }

    #[test]
    fn plan_installs_tool_from_apps_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let ac_root = tmp.path().join("ac");
        std::fs::create_dir_all(ac_root.join("content/cars")).unwrap();

        let archive = tmp.path().join("Paintshop.zip");
        {
            let file = std::fs::File::create(&archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.add_directory("apps/lua/Paintshop", options).unwrap();
            zip.start_file("apps/lua/Paintshop/Paintshop.lua", options)
                .unwrap();
            zip.finish().unwrap();
        }

        let items = plan(&archive, &ac_root).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, Kind::Tool);
        assert_eq!(items[0].id, "Paintshop");
        assert_eq!(items[0].prefix, "apps/lua/Paintshop");
        assert_eq!(items[0].target, ac_root.join("apps/lua/Paintshop"));
    }
}
