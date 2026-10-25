use egui::Ui;
use officina_core::content;

use crate::icons;
use crate::state::AppState;

fn file_uri(p: &std::path::Path) -> String {
    format!("file://{}", p.display().to_string().replace(' ', "%20"))
}

const ROW_HEIGHT: f32 = 72.0;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading(format!("{} Tracks", icons::nav(crate::state::Nav::Tracks)));
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(&mut state.track_filter);
        if ui.button(format!("{} Rescan", icons::RESCAN)).clicked() {
            state.trigger_scan();
        }
    });
    let filter = state.track_filter.to_lowercase();
    let entries = officina_core::content::track_entries(&state.tracks);
    let filtered: Vec<&content::TrackEntry> = entries
        .iter()
        .filter(|e| filter.is_empty() || e.search_text().contains(&filter))
        .collect();
    ui.label(format!("{} / {} layouts", filtered.len(), entries.len()));

    let mut uninstall_target: Option<std::path::PathBuf> = None;
    let mut disable_target: Option<(std::path::PathBuf, String)> = None;

    let num_rows = filtered.len();
    egui::ScrollArea::vertical().show_rows(ui, ROW_HEIGHT, num_rows, |ui, row_range| {
        for i in row_range {
            let e = filtered[i];
            ui.set_height(ROW_HEIGHT);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if let Some(p) = &e.layout.preview {
                        ui.add(
                            egui::Image::new(egui::ImageSource::Uri(file_uri(p).into()))
                                .fit_to_exact_size(egui::vec2(96.0, 64.0)),
                        );
                    } else {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(96.0, 64.0), egui::Sense::hover());
                        ui.painter().rect_stroke(
                            rect,
                            4.0,
                            egui::Stroke::new(1.0_f32, egui::Color32::DARK_GRAY),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if let Some(o) = &e.layout.outline {
                        ui.add(
                            egui::Image::new(egui::ImageSource::Uri(file_uri(o).into()))
                                .fit_to_exact_size(egui::vec2(64.0, 64.0)),
                        );
                    }
                    ui.vertical(|ui| {
                        ui.label(e.layout.name.to_string());
                        ui.small(format!(
                            "track: {}  layout: {:?}",
                            e.track_id, e.layout.layout_id
                        ));
                        if let Some(len) = &e.layout.length {
                            ui.small(format!("length: {len}"));
                        }
                        if let Some(pb) = e.layout.pitboxes {
                            ui.small(format!("pitboxes: {pb}"));
                        }
                        if ui.button("Uninstall track (to trash)").clicked() {
                            // find track dir
                            if let Some(t) = state.tracks.iter().find(|t| t.id == e.track_id) {
                                uninstall_target = Some(t.dir.clone());
                            }
                        }
                        if ui.button("Disable track").clicked()
                            && let Some(t) = state.tracks.iter().find(|t| t.id == e.track_id)
                        {
                            disable_target = Some((t.dir.clone(), e.track_id.clone()));
                        }
                    });
                });
            });
        }
    });

    if let Some(p) = uninstall_target {
        match officina_core::trash::move_to_trash(&p) {
            Ok(_) => {
                state.status = "Moved to trash.".into();
                state.trigger_scan();
            }
            Err(e) => state.status = format!("Uninstall failed: {e}"),
        }
    }
    if let Some((dir, id)) = disable_target
        && let Some(root) = state.settings.ac_root()
    {
        let _ = dir;
        match officina_core::mods::disable(&root, "tracks", &id) {
            Ok(_) => {
                state.status = "Disabled.".into();
                state.trigger_scan();
            }
            Err(e) => state.status = format!("Disable failed: {e}"),
        }
    }
    // Disabled tracks section
    let ac_root = state.settings.ac_root();
    let disabled = ac_root
        .as_ref()
        .map(|r| officina_core::mods::list_disabled(r, "tracks"))
        .unwrap_or_default();
    if !disabled.is_empty() {
        ui.separator();
        ui.label(format!("Disabled tracks ({}):", disabled.len()));
        for id in &disabled {
            ui.horizontal(|ui| {
                ui.label(id);
                if ui.button("Enable").clicked()
                    && let Some(root) = state.settings.ac_root()
                {
                    match officina_core::mods::enable(&root, "tracks", id) {
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
