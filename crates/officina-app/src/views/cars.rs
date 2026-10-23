use officina_core::content;
use egui::Ui;

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Cars");
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(&mut state.car_filter);
        if ui.button("Rescan").clicked() {
            state.trigger_scan();
        }
    });
    let filter = state.car_filter.to_lowercase();
    let cars: Vec<&content::Car> = state
        .cars
        .iter()
        .filter(|c| filter.is_empty() || c.search_text().contains(&filter))
        .collect();
    ui.label(format!("{} / {} cars", cars.len(), state.cars.len()));
    egui::ScrollArea::vertical().show(ui, |ui| {
        for car in cars {
            ui.group(|ui| {
                ui.label(format!("{}  —  {}", car.name, car.brand_name()));
                ui.small(format!("id: {}  skins: {}", car.id, car.skins.len()));
                if let Some(b) = &car.badge {
                    let _ = b;
                }
                if car.has_data_acd {
                    ui.small("data.acd");
                }
                if car.has_data_dir {
                    ui.small("data/");
                }
            });
        }
    });
}