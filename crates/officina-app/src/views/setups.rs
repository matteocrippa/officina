use egui::Ui;
use std::path::PathBuf;

use crate::state::{AppState, StatusTarget};

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Car Setups");

    // Car picker
    let car_ids: Vec<String> = state.cars.iter().map(|c| c.id.clone()).collect();
    ui.horizontal(|ui| {
        ui.label("Car:");
        egui::ComboBox::from_id_salt("setup_car")
            .selected_text(if state.qd_car.is_empty() {
                "select car"
            } else {
                &state.qd_car
            })
            .show_ui(ui, |ui| {
                for id in &car_ids {
                    if ui.selectable_label(state.qd_car == *id, id).clicked() {
                        state.qd_car = id.clone();
                    }
                }
            });
    });

    let car_id = state.qd_car.clone();
    if car_id.is_empty() {
        ui.label("Select a car.");
        return;
    }

    let ac_root = state.settings.ac_root();
    let setups = ac_root
        .as_ref()
        .map(|r| officina_core::setups::list_setups(r, &car_id))
        .unwrap_or_default();

    ui.label(format!("{} setup(s)", setups.len()));

    let mut to_delete: Option<PathBuf> = None;
    let mut to_view: Option<PathBuf> = None;
    let mut view_target: String = state.qd_setup.clone();

    for s in &setups {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    s.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                );
                if ui.button("View").clicked() {
                    to_view = Some(s.clone());
                }
                if ui.button("Use").clicked() {
                    view_target = s.display().to_string();
                }
                if ui.button("Delete").clicked() {
                    to_delete = Some(s.clone());
                }
            });
        });
    }

    if let Some(p) = to_delete {
        state.run_status_task(StatusTarget::Setups, move || {
            match officina_core::trash::move_to_trash(&p) {
                Ok(_) => "Moved to trash.".into(),
                Err(e) => format!("Delete failed: {e}"),
            }
        });
    }

    if let Some(p) = to_view
        && let Ok(text) = officina_core::setups::read_setup(&p)
    {
        ui.separator();
        ui.label(format!("Contents: {}", p.display()));
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                ui.monospace(text);
            });
    }

    // Save selected setup into qd_setup
    if view_target != state.qd_setup {
        state.qd_setup = view_target;
    }
}
