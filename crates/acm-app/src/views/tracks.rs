use ac_core::content;
use egui::Ui;

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Tracks");
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(&mut state.track_filter);
        if ui.button("Rescan").clicked() {
            state.trigger_scan();
        }
    });
    let filter = state.track_filter.to_lowercase();
    let entries = ac_core::content::track_entries(&state.tracks);
    let filtered: Vec<&content::TrackEntry> = entries
        .iter()
        .filter(|e| filter.is_empty() || e.search_text().contains(&filter))
        .collect();
    ui.label(format!("{} / {} layouts", filtered.len(), entries.len()));
    egui::ScrollArea::vertical().show(ui, |ui| {
        for e in filtered {
            ui.group(|ui| {
                ui.label(format!("{}", e.layout.name));
                ui.small(format!("track: {}  layout: {:?}", e.track_id, e.layout.layout_id));
                if let Some(len) = &e.layout.length {
                    ui.small(format!("length: {len}"));
                }
                if let Some(pb) = e.layout.pitboxes {
                    ui.small(format!("pitboxes: {pb}"));
                }
            });
        }
    });
}