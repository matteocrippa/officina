use egui::Ui;

use crate::icons;
use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading(format!("{} Weather", icons::nav(crate::state::Nav::Weather)));
    ui.horizontal(|ui| {
        if ui.button(format!("{} Rescan", icons::RESCAN)).clicked() {
            state.trigger_scan();
        }
    });
    ui.label(format!("{} weather presets", state.weather.len()));
    egui::ScrollArea::vertical().show(ui, |ui| {
        for w in &state.weather {
            let (ic, color) = icons::weather(w);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(ic).size(20.0).color(color));
                    ui.label(format!("{}  —  {}", w.name, w.id));
                });
            });
        }
    });
}