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
    if ui.button("Rescan content").clicked() {
        state.trigger_scan();
    }
    if ui.button("Auto-detect AC root").clicked() {
        state.settings.ac_root = None;
        let _ = state.settings.save();
        state.trigger_scan();
    }
}