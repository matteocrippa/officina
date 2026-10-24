use std::path::Path;

use egui::Ui;

use crate::state::AppState;

fn csp_installed(ac_root: &Path) -> Option<String> {
    let ext = ac_root.join("extension");
    if !ext.is_dir() {
        return None;
    }
    // Try common version files.
    for rel in ["config/version", "version.txt", "VERSION"] {
        let p = ext.join(rel);
        if let Ok(t) = std::fs::read_to_string(&p) {
            let t = t.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    Some("installed".to_string())
}

pub fn show(state: &mut AppState, ui: &mut Ui, ctx: &egui::Context) {
    ui.heading("Patches & Weather (CSP / Sol / Pure)");
    let ac_root = state.settings.ac_root();

    // CSP section
    ui.group(|ui| {
        ui.label("Custom Shaders Patch (CSP)");
        match &ac_root {
            Some(root) => match csp_installed(root) {
                Some(v) => ui.label(format!("Status: installed ({v})")),
                None => ui.label("Status: not installed"),
            },
            None => ui.label("AC root not found"),
        };
        ui.small("CSP settings are edited in extension/config or via CSP's own in-game UI.");
        if ui.button("Install/update CSP from archive...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Archives", &["zip", "7z", "rar"])
                .pick_file()
            {
                state.plan_install(path);
                state.nav = crate::state::Nav::Install;
            }
        }
    });

    // Sol / Pure section
    ui.group(|ui| {
        ui.label("Sol / Pure (weather mods)");
        let sol_pure: Vec<&officina_core::content::Weather> = state
            .weather
            .iter()
            .filter(|w| {
                let n = format!("{} {}", w.name, w.id).to_lowercase();
                n.contains("sol") || n.contains("pure")
            })
            .collect();
        if sol_pure.is_empty() {
            ui.label("None detected in content/weather.");
        } else {
            for w in sol_pure {
                ui.label(format!("  {} ({})", w.name, w.id));
            }
        }
        if ui.button("Install Sol/Pure from archive...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Archives", &["zip", "7z", "rar"])
                .pick_file()
            {
                state.plan_install(path);
                state.nav = crate::state::Nav::Install;
            }
        }
    });

    ui.small("Tip: you can also drag-and-drop any archive onto the Install tab.");
    let _ = ctx;
}