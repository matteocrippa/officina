use std::path::PathBuf;

use ac_core::{content, install, launch, paths, settings::Settings};
use anyhow::{Context, Result};

pub fn where_cmd() -> Result<()> {
    let s = Settings::load();
    let ac_root = s.ac_root();
    println!("AC root:        {}", ac_root.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<not found>".into()));
    if let Some(r) = &ac_root {
        println!("compatdata:     {}", paths::compat_data_dir(r).map(|p| p.display().to_string()).unwrap_or_else(|| "<none>".into()));
        println!("documents:      {}", paths::documents_ac_dir(r).map(|p| p.display().to_string()).unwrap_or_else(|| "<none>".into()));
        println!("race.ini:       {}", paths::race_ini_path(r).map(|p| p.display().to_string()).unwrap_or_else(|| "<none>".into()));
    }
    let proton = s
        .proton
        .clone()
        .or_else(|| ac_root.as_ref().and_then(|r| launch::find_proton(r)));
    println!("Proton:         {}", proton.map(|p| p.display().to_string()).unwrap_or_else(|| "<not found>".into()));
    println!("Steam roots:    {}", paths::steam_roots().iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", "));
    Ok(())
}

pub fn list_cmd(kind: &str) -> Result<()> {
    let s = Settings::load();
    let ac_root = s.ac_root().context("Assetto Corsa install not found")?;
    match kind {
        "cars" => {
            let cars = content::scan_cars(&ac_root);
            println!("{} cars:", cars.len());
            for c in cars {
                println!("  {}  ({})  [{} skins]", c.id, c.name, c.skins.len());
            }
        }
        "tracks" => {
            let tracks = content::scan_tracks(&ac_root);
            println!("{} tracks:", tracks.len());
            for t in tracks {
                println!("  {}  ({})", t.id, t.layouts.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(", "));
            }
        }
        other => anyhow::bail!("unknown kind: {other} (expected 'cars' or 'tracks')"),
    }
    Ok(())
}

pub fn install_cmd(archive: PathBuf) -> Result<()> {
    let s = Settings::load();
    let ac_root = s.ac_root().context("Assetto Corsa install not found")?;
    let items = install::plan(&archive, &ac_root).context("planning install")?;
    println!("Planned {} item(s):", items.len());
    for it in &items {
        println!("  {:?} {} -> {}  (exists: {})", it.kind, it.id, it.target.display(), it.exists);
    }
    let written = install::install(&archive, &items).context("installing")?;
    println!("Installed {} file(s).", written);
    Ok(())
}