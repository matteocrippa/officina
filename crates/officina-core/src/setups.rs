//! Car setup files stored under Documents/Assetto Corsa/cars/<car>/.

use std::fs;
use std::path::{Path, PathBuf};

use crate::paths;

pub fn setups_dir(ac_root: &Path, car_id: &str) -> Option<PathBuf> {
    let doc = paths::documents_ac_dir(ac_root)?;
    let dir = doc.join("cars").join(car_id);
    if dir.is_dir() { Some(dir) } else { None }
}

pub fn list_setups(ac_root: &Path, car_id: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = setups_dir(ac_root, car_id)
        && let Ok(entries) = fs::read_dir(&dir)
    {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file()
                && p.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| x.eq_ignore_ascii_case("ini"))
            {
                out.push(p);
            }
        }
    }
    // Also check car folder shipped setups.
    let car_setups = ac_root
        .join("content")
        .join("cars")
        .join(car_id)
        .join("setups");
    if car_setups.is_dir()
        && let Ok(entries) = fs::read_dir(&car_setups)
    {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file()
                && p.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| x.eq_ignore_ascii_case("ini"))
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn read_setup(path: &Path) -> std::io::Result<String> {
    fs::read_to_string(path)
}
