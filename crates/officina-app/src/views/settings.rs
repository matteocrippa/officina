use egui::Ui;

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Settings");
    ui.horizontal(|ui| {
        ui.label("Theme:");
        let mut theme = state.settings.theme.clone();
        egui::ComboBox::from_id_salt("theme")
            .selected_text(&theme)
            .show_ui(ui, |ui| {
                for t in ["dark", "light"] {
                    if ui.selectable_label(theme == t, t).clicked() {
                        theme = t.to_string();
                    }
                }
            });
        if theme != state.settings.theme {
            state.settings.theme = theme;
            let _ = state.settings.save();
        }
    });
    ui.horizontal(|ui| {
        ui.label("AC root:");
        let mut s = state
            .settings
            .ac_root
            .clone()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        if ui.text_edit_singleline(&mut s).lost_focus() {
            state.settings.ac_root = if s.is_empty() {
                None
            } else {
                Some(std::path::PathBuf::from(s))
            };
            let _ = state.settings.save();
        }
    });
    ui.horizontal(|ui| {
        ui.label("Proton:");
        let mut s = state
            .settings
            .proton
            .clone()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        if ui.text_edit_singleline(&mut s).lost_focus() {
            state.settings.proton = if s.is_empty() {
                None
            } else {
                Some(std::path::PathBuf::from(s))
            };
            let _ = state.settings.save();
        }
    });
    let root = state
        .settings
        .ac_root()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<not found>".into());
    let compat = state
        .settings
        .ac_root()
        .and_then(|r| officina_core::paths::compat_data_dir(&r))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<none>".into());
    ui.label(format!("Resolved AC root: {root}"));
    ui.label(format!("Compatdata: {compat}"));
    ui.separator();
    ui.heading("Settings backups");
    if ui.button("Create backup now").clicked() {
        if let Some(root) = state.settings.ac_root() {
            match officina_core::backup::create_backup(&root) {
                Ok(p) => state.game_settings_status = format!("Backup created: {}", p.display()),
                Err(e) => state.game_settings_status = format!("Backup failed: {e:#}"),
            }
        } else {
            state.game_settings_status = "AC root not found.".into();
        }
    }
    let backups = officina_core::backup::list_backups();
    if !backups.is_empty() {
        ui.label(format!("{} backup(s):", backups.len()));
        for b in &backups {
            ui.horizontal(|ui| {
                ui.label(
                    b.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                );
                if ui.button("Restore").clicked()
                    && let Some(root) = state.settings.ac_root()
                {
                    match officina_core::backup::restore_backup(b, &root) {
                        Ok(_) => state.game_settings_status = "Restored.".into(),
                        Err(e) => state.game_settings_status = format!("Restore failed: {e:#}"),
                    }
                }
            });
        }
    }
    ui.separator();
    ui.heading("Setup / Diagnostics");
    let diag = officina_core::setup::diagnose(&state.settings);
    let check = |ok: bool| if ok { "✅" } else { "❌" };
    ui.label(format!(
        "{} Steam roots: {}",
        check(!diag.steam_roots.is_empty()),
        diag.steam_roots.len()
    ));
    ui.label(format!(
        "{} AC root: {}",
        check(diag.ac_root_valid),
        diag.ac_root
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<not found>".into())
    ));
    ui.label(format!(
        "{} Compatdata: {}",
        check(diag.compatdata.is_some()),
        diag.compatdata
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".into())
    ));
    ui.label(format!(
        "{} Proton: {}",
        check(diag.proton.is_some()),
        diag.proton
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<not found>".into())
    ));
    ui.label(format!(
        "{} Proton-GE: {}",
        check(!diag.proton_ge.is_empty()),
        if diag.proton_ge.is_empty() {
            "none found".into()
        } else {
            diag.proton_ge
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    ui.label(format!(
        "{} CSP: {}",
        check(diag.csp.is_some()),
        diag.csp.clone().unwrap_or_else(|| "not installed".into())
    ));
    ui.label(format!(
        "{} DXVK (heuristic): {}",
        check(diag.dxvk),
        if diag.dxvk {
            "detected"
        } else {
            "not detected"
        }
    ));
    if diag.ac_root.is_none() || !diag.ac_root_valid {
        ui.small("Install Assetto Corsa on Steam first.");
    }
    if diag.compatdata.is_none() && diag.ac_root.is_some() {
        ui.small("Run Assetto Corsa from Steam once to create the Proton prefix.");
    }
    if diag.proton_ge.is_empty() {
        ui.small("Proton-GE not found. Recommended for best compatibility.");
        if ui.button("Open Proton-GE releases").clicked() {
            let _ = std::process::Command::new("xdg-open")
                .arg("https://github.com/GloriousEggroll/proton-ge-custom/releases")
                .spawn();
        }
        ui.small("Or run the upstream Linux setup script:");
        ui.monospace(officina_core::setup::linux_setup_command());
    }
    if diag.csp.is_none() && diag.ac_root.is_some() {
        ui.small("CSP not installed. Use the Patches tab to install it.");
    }
    ui.separator();
    ui.heading("Launch");
    if ui.button("Launch Assetto Corsa").clicked() {
        if let Some(root) = state.settings.ac_root() {
            let qd = state.settings.last_session.clone().unwrap_or_default();
            match officina_core::launch::write_race_ini(&root, &qd) {
                Ok(_) => {
                    let opts = officina_core::launch::LaunchOptions {
                        proton: state.settings.proton.clone(),
                    };
                    match officina_core::launch::start(&root, &opts) {
                        Ok(_) => state.game_settings_status = "Launched!".into(),
                        Err(e) => state.game_settings_status = format!("Launch failed: {e:#}"),
                    }
                }
                Err(e) => state.game_settings_status = format!("write_race_ini failed: {e:#}"),
            }
        } else {
            state.game_settings_status = "AC root not found.".into();
        }
    }
    ui.label(&state.game_settings_status);
    ui.separator();
    ui.heading("Updates");
    ui.horizontal(|ui| {
        ui.label("Repo (owner/name):");
        let mut r = state.settings.update_repo.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut r).lost_focus() {
            state.settings.update_repo = if r.is_empty() { None } else { Some(r) };
            let _ = state.settings.save();
        }
    });
    ui.label(format!(
        "Current version: {}",
        crate::updater::current_version()
    ));
    if ui.button("Check for updates").clicked() {
        if let Some(repo) = state.settings.update_repo.clone() {
            match crate::updater::check_latest(&repo) {
                Ok(tag) => state.game_settings_status = format!("Latest: {tag}"),
                Err(e) => state.game_settings_status = format!("Check failed: {e}"),
            }
        } else {
            state.game_settings_status = "Set update_repo first.".into();
        }
    }
    if ui.button("Download & install update").clicked() {
        if let Some(repo) = state.settings.update_repo.clone() {
            state.game_settings_status = "Updating...".into();
            match crate::updater::download_and_install(&repo) {
                Ok(msg) => state.game_settings_status = msg,
                Err(e) => state.game_settings_status = format!("Update failed: {e}"),
            }
        } else {
            state.game_settings_status = "Set update_repo first.".into();
        }
    }
    ui.label(&state.game_settings_status);
    if ui.button("Rescan content").clicked() {
        state.trigger_scan();
    }
    if ui.button("Auto-detect AC root").clicked() {
        state.settings.ac_root = None;
        let _ = state.settings.save();
        state.trigger_scan();
    }
}
