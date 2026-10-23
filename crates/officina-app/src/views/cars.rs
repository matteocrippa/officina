use officina_core::content;
use egui::Ui;

use crate::state::AppState;

fn file_uri(p: &std::path::Path) -> String {
    format!("file://{}", p.display().to_string().replace(' ', "%20"))
}

const ROW_HEIGHT: f32 = 88.0;

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
    let num_rows = cars.len();
    egui::ScrollArea::vertical().show_rows(ui, ROW_HEIGHT, num_rows, |ui, row_range| {
        for i in row_range {
            let car = cars[i];
            ui.set_height(ROW_HEIGHT);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if let Some(p) = car.preview() {
                        ui.add(
                            egui::Image::new(egui::ImageSource::Uri(file_uri(p).into()))
                                .fit_to_exact_size(egui::vec2(72.0, 72.0)),
                        );
                    } else {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(72.0, 72.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_stroke(
                            rect,
                            4.0,
                            egui::Stroke::new(1.0_f32, egui::Color32::DARK_GRAY),
                            egui::StrokeKind::Outside,
                        );
                    }
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}  —  {}", car.name, car.brand_name()));
                            if let Some(b) = &car.badge {
                                ui.add(
                                    egui::Image::new(egui::ImageSource::Uri(file_uri(b).into()))
                                        .fit_to_exact_size(egui::vec2(24.0, 24.0)),
                                );
                            }
                        });
                        ui.small(format!("id: {}  skins: {}", car.id, car.skins.len()));
                        if car.has_data_acd {
                            ui.small("data.acd");
                        }
                        if car.has_data_dir {
                            ui.small("data/");
                        }
                    });
                });
            });
        }
    });
}