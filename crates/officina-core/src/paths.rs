//! Locating Steam, the Assetto Corsa install and its Documents folder on
//! Windows and on Linux (Steam + Proton).

use std::fs;
use std::path::{Path, PathBuf};

use crate::AC_APP_ID;

pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam") {
            if let Ok(p) = k.get_value::<String, _>("SteamPath") {
                roots.push(PathBuf::from(p.replace('/', "\\")));
            }
        }
        if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam") {
            if let Ok(p) = k.get_value::<String, _>("InstallPath") {
                roots.push(PathBuf::from(p));
            }
        }
        roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    }

    #[cfg(not(windows))]
    if let Some(home) = dirs::home_dir() {
        for rel in [
            ".steam/steam",
            ".steam/root",
            ".local/share/Steam",
            ".var/app/com.valvesoftware.Steam/.local/share/Steam",
            "snap/steam/common/.local/share/Steam",
        ] {
            roots.push(home.join(rel));
        }
    }

    let mut seen = Vec::<PathBuf>::new();
    roots
        .into_iter()
        .filter(|p| p.join("steamapps").is_dir())
        .filter(|p| {
            let canon = fs::canonicalize(p).unwrap_or_else(|_| p.clone());
            if seen.contains(&canon) {
                false
            } else {
                seen.push(canon);
                true
            }
        })
        .collect()
}

pub fn steam_libraries() -> Vec<PathBuf> {
    let mut libs: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        let canon = fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
        if p.join("steamapps").is_dir() && !libs.iter().any(|l| fs::canonicalize(l).unwrap_or_else(|_| l.clone()) == canon) {
            libs.push(p);
        }
    };
    for root in steam_roots() {
        push(root.clone());
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(&vdf) {
            for path in parse_library_paths(&text) {
                push(path);
            }
        }
    }
    libs
}

pub fn steam_sdl_lib_dirs() -> Vec<PathBuf> {
    #[cfg(windows)]
    return Vec::new();
    #[cfg(not(windows))]
    {
        let Some(stage) = dirs::cache_dir().map(|d| d.join("officina").join("steam-sdl")) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (arch, sub) in [("x86_64-linux-gnu", "64"), ("i386-linux-gnu", "32")] {
            let Some(src) = steam_roots()
                .into_iter()
                .map(|r| r.join("ubuntu12_32/steam-runtime/usr/lib").join(arch))
                .find(|p| p.join("libSDL2-2.0.so.0").exists())
            else {
                continue;
            };
            let dst = stage.join(sub);
            if fs::create_dir_all(&dst).is_err() {
                continue;
            }
            for name in ["libSDL2-2.0.so.0"] {
                let link = dst.join(name);
                let _ = fs::remove_file(&link);
                if std::os::unix::fs::symlink(src.join(name), &link).is_err() {
                    continue;
                }
            }
            out.push(dst);
        }
        out
    }
}

pub fn parse_library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('"').collect();
            (parts.len() >= 5 && parts[1].eq_ignore_ascii_case("path"))
                .then(|| PathBuf::from(parts[3].replace("\\\\", "\\")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_library_paths_extracts() {
        let line = "\t\t\"path\"\t\t\"/home/user/.steam/steam\"";
        let paths = parse_library_paths(line);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], PathBuf::from("/home/user/.steam/steam"));
    }

    #[test]
    fn parse_library_paths_empty() {
        assert!(parse_library_paths("").is_empty());
        assert!(parse_library_paths("no paths here").is_empty());
    }

    #[test]
    fn is_ac_root_requires_files() {
        let tmp = std::env::temp_dir().join("officina_test_ac");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("content/cars")).unwrap();
        std::fs::write(tmp.join("acs.exe"), b"").unwrap();
        assert!(is_ac_root(&tmp));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

pub fn is_ac_root(p: &Path) -> bool {
    p.join("acs.exe").is_file() && p.join("content").join("cars").is_dir()
}

pub fn find_ac_root() -> Option<PathBuf> {
    steam_libraries()
        .into_iter()
        .map(|lib| lib.join("steamapps").join("common").join("assettocorsa"))
        .find(|p| is_ac_root(p))
}

pub fn steamapps_of(ac_root: &Path) -> Option<PathBuf> {
    let common = ac_root.parent()?;
    let steamapps = common.parent()?;
    (common.file_name()?.eq_ignore_ascii_case("common")).then(|| steamapps.to_path_buf())
}

pub fn compat_data_dir(ac_root: &Path) -> Option<PathBuf> {
    let id = AC_APP_ID.to_string();
    let near = steamapps_of(ac_root).map(|s| s.join("compatdata").join(&id));
    near.into_iter()
        .chain(steam_libraries().into_iter().map(|l| l.join("steamapps").join("compatdata").join(&id)))
        .find(|p| p.join("pfx").is_dir())
}

pub fn documents_ac_dir(ac_root: &Path) -> Option<PathBuf> {
    if cfg!(windows) {
        let _ = ac_root;
        Some(dirs::document_dir()?.join("Assetto Corsa"))
    } else {
        Some(
            compat_data_dir(ac_root)?
                .join("pfx/drive_c/users/steamuser/Documents/Assetto Corsa"),
        )
    }
}

pub fn race_ini_path(ac_root: &Path) -> Option<PathBuf> {
    Some(documents_ac_dir(ac_root)?.join("cfg").join("race.ini"))
}