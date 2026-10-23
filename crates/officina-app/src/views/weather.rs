use egui::Ui;

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Weather");
    ui.horizontal(|ui| {
        if ui.button("Rescan").clicked() {
            state.trigger_scan();
        }
    });
    ui.label(format!("{} weather presets", state.weather.len()));
    egui::ScrollArea::vertical().show(ui, |ui| {
        for w in &state.weather {
            ui.group(|ui| {
                ui.label(format!("{}  —  {}", w.name, w.id));
            });
        }
    });
}