use egui::Ui;

use crate::icons;
use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading(format!(
        "{} Weather",
        icons::nav(crate::state::Nav::Weather)
    ));
    ui.horizontal(|ui| {
        if ui.button(format!("{} Rescan", icons::RESCAN)).clicked() {
            state.trigger_scan();
        }
    });
    ui.label(format!("{} weather presets", state.weather.len()));

    let mut uninstall_target: Option<std::path::PathBuf> = None;
    let mut disable_target: Option<String> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        for w in &state.weather {
            let (ic, color) = icons::weather(w);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(ic).size(20.0).color(color));
                    ui.label(format!("{}  —  {}", w.name, w.id));
                    if ui.button("Uninstall (to trash)").clicked()
                        && let Some(root) = state.settings.ac_root()
                    {
                        uninstall_target = Some(root.join("content").join("weather").join(&w.id));
                    }
                    if ui.button("Disable").clicked() {
                        disable_target = Some(w.id.clone());
                    }
                });
            });
        }
    });

    if let Some(p) = uninstall_target {
        state.trash_path(p, "Moved to trash.");
    }
    if let Some(id) = disable_target
        && let Some(root) = state.settings.ac_root()
    {
        match officina_core::mods::disable(&root, "weather", &id) {
            Ok(_) => {
                state.status = "Disabled.".into();
                state.trigger_scan();
            }
            Err(e) => state.status = format!("Disable failed: {e}"),
        }
    }

    let ac_root = state.settings.ac_root();
    let disabled = ac_root
        .as_ref()
        .map(|r| officina_core::mods::list_disabled(r, "weather"))
        .unwrap_or_default();
    if !disabled.is_empty() {
        ui.separator();
        ui.label(format!("Disabled weather ({}):", disabled.len()));
        for id in &disabled {
            ui.horizontal(|ui| {
                ui.label(id);
                if ui.button("Enable").clicked()
                    && let Some(root) = state.settings.ac_root()
                {
                    match officina_core::mods::enable(&root, "weather", id) {
                        Ok(_) => {
                            state.status = "Enabled.".into();
                            state.trigger_scan();
                        }
                        Err(e) => state.status = format!("Enable failed: {e}"),
                    }
                }
            });
        }
    }
}
