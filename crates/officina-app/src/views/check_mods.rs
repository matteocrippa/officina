use egui::Ui;

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Check Mods");
    if ui.button("Run check").clicked() {
        if let Some(root) = state.settings.ac_root() {
            let issues = officina_core::validate::check(&root);
            if issues.is_empty() {
                state.check_mods_status = "No issues found.".into();
            } else {
                state.check_mods_status = issues
                    .iter()
                    .map(|i| format!("[{}] {} — {}", i.kind, i.path.display(), i.message))
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        } else {
            state.check_mods_status = "AC root not found.".into();
        }
    }
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for line in state.check_mods_status.lines() {
            ui.label(line);
        }
    });
}