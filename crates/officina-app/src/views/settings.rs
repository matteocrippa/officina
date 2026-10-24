use egui::Ui;

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Settings");
    ui.horizontal(|ui| {
        ui.label("AC root:");
        let mut s = state.settings.ac_root.clone().map(|p| p.display().to_string()).unwrap_or_default();
        if ui.text_edit_singleline(&mut s).lost_focus() {
            state.settings.ac_root = if s.is_empty() { None } else { Some(std::path::PathBuf::from(s)) };
            let _ = state.settings.save();
        }
    });
    ui.horizontal(|ui| {
        ui.label("Proton:");
        let mut s = state.settings.proton.clone().map(|p| p.display().to_string()).unwrap_or_default();
        if ui.text_edit_singleline(&mut s).lost_focus() {
            state.settings.proton = if s.is_empty() { None } else { Some(std::path::PathBuf::from(s)) };
            let _ = state.settings.save();
        }
    });
    let root = state.settings.ac_root().map(|p| p.display().to_string()).unwrap_or_else(|| "<not found>".into());
    let compat = state.settings.ac_root().and_then(|r| officina_core::paths::compat_data_dir(&r)).map(|p| p.display().to_string()).unwrap_or_else(|| "<none>".into());
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
                ui.label(b.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
                if ui.button("Restore").clicked() {
                    if let Some(root) = state.settings.ac_root() {
                        match officina_core::backup::restore_backup(b, &root) {
                            Ok(_) => state.game_settings_status = "Restored.".into(),
                            Err(e) => state.game_settings_status = format!("Restore failed: {e:#}"),
                        }
                    }
                }
            });
        }
    }
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
    ui.label(format!("Current version: {}", crate::updater::current_version()));
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