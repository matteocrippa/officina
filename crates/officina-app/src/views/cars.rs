use egui::Ui;
use officina_core::content;

use crate::icons;
use crate::state::AppState;

fn file_uri(p: &std::path::Path) -> String {
    format!("file://{}", p.display().to_string().replace(' ', "%20"))
}

const ROW_HEIGHT: f32 = 88.0;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading(format!("{} Cars", icons::nav(crate::state::Nav::Cars)));
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(&mut state.car_filter);
        if ui.button(format!("{} Rescan", icons::RESCAN)).clicked() {
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

    let mut uninstall_target: Option<std::path::PathBuf> = None;
    let mut disable_target: Option<std::path::PathBuf> = None;
    let mut enable_target: Option<String> = None;

    ui.columns(2, |cols| {
        cols[0].vertical(|ui| {
            egui::ScrollArea::vertical().show_rows(ui, ROW_HEIGHT, cars.len(), |ui, row_range| {
                for i in row_range {
                    let car = cars[i];
                    let selected = state.selected_car.as_deref() == Some(car.id.as_str());
                    let mut clicked = false;
                    ui.horizontal(|ui| {
                        let thumb_resp = if let Some(p) = car.preview() {
                            ui.add(
                                egui::Image::new(egui::ImageSource::Uri(file_uri(p).into()))
                                    .fit_to_exact_size(egui::vec2(72.0, 72.0)),
                            )
                        } else {
                            let (rect, r) = ui
                                .allocate_exact_size(egui::vec2(72.0, 72.0), egui::Sense::click());
                            ui.painter().rect_stroke(
                                rect,
                                4.0,
                                egui::Stroke::new(1.0_f32, egui::Color32::DARK_GRAY),
                                egui::StrokeKind::Outside,
                            );
                            r
                        };
                        if thumb_resp.clicked() {
                            clicked = true;
                        }
                        let label_resp = ui.selectable_label(
                            selected,
                            format!("{}  —  {}", car.name, car.brand_name()),
                        );
                        if label_resp.clicked() {
                            clicked = true;
                        }
                        ui.vertical(|ui| {
                            ui.small(format!("id: {}  skins: {}", car.id, car.skins.len()));
                        });
                    });
                    if clicked {
                        if selected {
                            state.selected_car = None;
                        } else {
                            state.selected_car = Some(car.id.clone());
                        }
                    }
                }
            });
        });

        cols[1].vertical(|ui| {
            if let Some(id) = state.selected_car.clone() {
                if let Some(car) = state.cars.iter().find(|c| c.id == id) {
                    ui.heading(&car.name);
                    ui.label(format!("Brand: {}", car.brand_name()));
                    ui.label(format!("ID: {}", car.id));
                    if let Some(c) = &car.class {
                        ui.label(format!("Class: {c}"));
                    }
                    if let Some(y) = &car.year {
                        ui.label(format!("Year: {y}"));
                    }
                    if let Some(country) = &car.country {
                        ui.label(format!("Country: {country}"));
                    }
                    if let Some(author) = &car.author {
                        ui.label(format!("Author: {author}"));
                    }
                    if let Some(desc) = &car.description {
                        ui.separator();
                        ui.label("Description:");
                        ui.label(desc);
                    }
                    if !car.specs.is_empty() {
                        ui.separator();
                        ui.label("Specs:");
                        for (k, v) in &car.specs {
                            ui.label(format!("  {} : {}", content::spec_label(k), v));
                        }
                    }
                    if let Some(b) = &car.badge {
                        ui.separator();
                        ui.label("Badge:");
                        ui.add(
                            egui::Image::new(egui::ImageSource::Uri(file_uri(b).into()))
                                .fit_to_exact_size(egui::vec2(64.0, 64.0)),
                        );
                    }
                    if !car.skins.is_empty() {
                        ui.separator();
                        ui.label(format!("Skins ({}):", car.skins.len()));
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for skin in &car.skins {
                                ui.horizontal(|ui| {
                                    if let Some(p) = &skin.preview {
                                        ui.add(
                                            egui::Image::new(egui::ImageSource::Uri(
                                                file_uri(p).into(),
                                            ))
                                            .fit_to_exact_size(egui::vec2(48.0, 48.0)),
                                        );
                                    }
                                    ui.vertical(|ui| {
                                        ui.label(&skin.name);
                                        ui.small(format!("id: {}", skin.id));
                                        if let Some(n) = &skin.number {
                                            ui.small(format!("number: {n}"));
                                        }
                                    });
                                });
                            }
                        });
                    }
                    if let Some(err) = &car.ui_error {
                        ui.colored_label(egui::Color32::RED, format!("ui error: {err}"));
                    }
                    ui.separator();
                    if ui.button("Uninstall car (to trash)").clicked() {
                        uninstall_target = Some(car.dir.clone());
                    }
                    if ui.button("Disable car").clicked() {
                        disable_target = Some(car.dir.clone());
                    }
                } else {
                    ui.label("Car not found.");
                }
            } else {
                ui.label("Select a car from the list.");
            }
        });
    });
    // Disabled cars section
    let ac_root = state.settings.ac_root();
    let disabled = ac_root
        .as_ref()
        .map(|r| officina_core::mods::list_disabled(r, "cars"))
        .unwrap_or_default();
    if !disabled.is_empty() {
        ui.separator();
        ui.label(format!("Disabled cars ({}):", disabled.len()));
        for id in &disabled {
            ui.horizontal(|ui| {
                ui.label(id);
                if ui.button("Enable").clicked() {
                    enable_target = Some(id.clone());
                }
            });
        }
    }

    if let Some(p) = uninstall_target {
        match officina_core::trash::move_to_trash(&p) {
            Ok(_) => {
                state.selected_car = None;
                state.status = "Moved to trash.".into();
                state.trigger_scan();
            }
            Err(e) => state.status = format!("Uninstall failed: {e}"),
        }
    }
    if let Some(dir) = disable_target
        && let Some(root) = state.settings.ac_root()
    {
        let id = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match officina_core::mods::disable(&root, "cars", &id) {
            Ok(_) => {
                state.selected_car = None;
                state.status = "Disabled.".into();
                state.trigger_scan();
            }
            Err(e) => state.status = format!("Disable failed: {e}"),
        }
    }
    if let Some(id) = enable_target
        && let Some(root) = state.settings.ac_root()
    {
        match officina_core::mods::enable(&root, "cars", &id) {
            Ok(_) => {
                state.status = "Enabled.".into();
                state.trigger_scan();
            }
            Err(e) => state.status = format!("Enable failed: {e}"),
        }
    }
}
