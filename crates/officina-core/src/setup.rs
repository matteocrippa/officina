//! Environment diagnostics for running Assetto Corsa on Linux: Steam, AC install,
//! Proton prefix, Proton-GE, CSP and DXVK.

use std::fs;
use std::path::{Path, PathBuf};

use crate::paths;
use crate::settings::Settings;

#[derive(Debug, Clone)]
pub struct Diagnostics {
    pub steam_roots: Vec<PathBuf>,
    pub ac_root: Option<PathBuf>,
    pub ac_root_valid: bool,
    pub compatdata: Option<PathBuf>,
    pub proton: Option<PathBuf>,
    pub proton_ge: Vec<PathBuf>,
    pub csp: Option<String>,
    pub dxvk: bool,
    pub bazzite: bool,
    pub distro: String,
}

pub fn diagnose(settings: &Settings) -> Diagnostics {
    let steam_roots = paths::steam_roots();
    let ac_root = settings.ac_root();
    let ac_root_valid = ac_root
        .as_ref()
        .map(|r| paths::is_ac_root(r))
        .unwrap_or(false);
    let compatdata = ac_root.as_ref().and_then(|r| paths::compat_data_dir(r));
    let proton = settings
        .proton
        .clone()
        .or_else(|| ac_root.as_ref().and_then(|r| crate::launch::find_proton(r)));
    let proton_ge = find_proton_ge(&steam_roots);
    let csp = ac_root.as_ref().and_then(|r| csp_installed(r));
    let dxvk = dxvk_present(&steam_roots, compatdata.as_deref());
    let (bazzite, distro) = detect_distro();
    Diagnostics {
        steam_roots,
        ac_root,
        ac_root_valid,
        compatdata,
        proton,
        proton_ge,
        csp,
        dxvk,
        bazzite,
        distro,
    }
}

fn find_proton_ge(steam_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in steam_roots {
        for subdir in ["compatibilitytools.d", "steamapps/common"] {
            let dir = root.join(subdir);
            if let Ok(entries) = fs::read_dir(&dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        let name = p
                            .file_name()
                            .map(|n| n.to_string_lossy().to_lowercase())
                            .unwrap_or_default();
                        if name.contains("ge-proton") || name.contains("proton-ge") {
                            if p.join("proton").is_file() {
                                found.push(p.join("proton"));
                            } else {
                                found.push(p);
                            }
                        }
                    }
                }
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

fn csp_installed(ac_root: &Path) -> Option<String> {
    let ext = ac_root.join("extension");
    if !ext.is_dir() {
        return None;
    }
    for rel in ["config/version", "version.txt", "VERSION"] {
        let p = ext.join(rel);
        if let Ok(t) = fs::read_to_string(&p) {
            let t = t.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    Some("installed".to_string())
}

fn dxvk_present(steam_roots: &[PathBuf], compatdata: Option<&Path>) -> bool {
    if let Some(cd) = compatdata {
        let sys32 = cd.join("pfx/drive_c/windows/system32");
        if sys32.join("dxgi.dll").is_file() || sys32.join("d3d11.dll").is_file() {
            // Heuristic: DXVK renames; presence of these in prefix is not definitive.
            // Fall through to directory check.
        }
    }
    for root in steam_roots {
        if root.join("dxvk").is_dir() {
            return true;
        }
    }
    // Check common DXVK install location under a steam root's ubuntu runtime.
    false
}

pub fn linux_setup_command() -> String {
    "curl -Os https://raw.githubusercontent.com/sihawido/assettocorsa-linux-setup/main/assettocorsa-linux-setup.sh && bash assettocorsa-linux-setup.sh".to_string()
}

pub fn proton_ge_install_hint(bazzite: bool) -> String {
    if bazzite {
        "Run: ujust install-proton-ge  (Bazzite helper)".to_string()
    } else {
        linux_setup_command()
    }
}

fn detect_distro() -> (bool, String) {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut name = String::new();
    let mut bazzite = false;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("NAME=") {
            name = v.trim_matches('"').to_string();
        }
        if line.to_lowercase().contains("bazzite") {
            bazzite = true;
        }
        if let Some(v) = line.strip_prefix("ID=") {
            let id = v.trim_matches('"').to_lowercase();
            if id == "bazzite" {
                bazzite = true;
            }
            if name.is_empty() {
                name = id;
            }
        }
    }
    if name.is_empty() {
        name = "unknown".to_string();
    }
    (bazzite, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn diagnose_without_ac_root_does_not_panic() {
        let s = Settings::default();
        let d = diagnose(&s);
        assert!(d.ac_root.is_none());
        assert!(!d.ac_root_valid);
        assert!(d.compatdata.is_none());
    }

    #[test]
    fn linux_setup_command_nonempty() {
        assert!(!linux_setup_command().is_empty());
    }
}
