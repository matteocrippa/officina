use egui::Ui;

use officina_core::content;
use officina_core::launch::{self, LaunchOptions};
use officina_core::race::{self, Conditions, Mode, QuickDrive};

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Quick Drive");

    // Car picker
    ui.horizontal(|ui| {
        ui.label("Car:");
        let car_ids: Vec<String> = state.cars.iter().map(|c| c.id.clone()).collect();
        egui::ComboBox::from_id_salt("qd_car")
            .selected_text(if state.qd_car.is_empty() { "select car" } else { &state.qd_car })
            .show_ui(ui, |ui| {
                for id in &car_ids {
                    if ui.selectable_label(state.qd_car == *id, id).clicked() {
                        state.qd_car = id.clone();
                        // reset skin
                        state.qd_skin.clear();
                    }
                }
            });
    });

    // Skin picker
    let skins: Vec<String> = state
        .cars
        .iter()
        .find(|c| c.id == state.qd_car)
        .map(|c| c.skins.iter().map(|s| s.id.clone()).collect())
        .unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Skin:");
        egui::ComboBox::from_id_salt("qd_skin")
            .selected_text(if state.qd_skin.is_empty() { "select skin" } else { &state.qd_skin })
            .show_ui(ui, |ui| {
                for s in &skins {
                    if ui.selectable_label(state.qd_skin == *s, s).clicked() {
                        state.qd_skin = s.clone();
                    }
                }
            });
    });

    // Track + layout picker
    let entries = content::track_entries(&state.tracks);
    ui.horizontal(|ui| {
        ui.label("Track:");
        let sel = if state.qd_track.is_empty() {
            "select track".to_string()
        } else {
            match &state.qd_layout {
                Some(l) => format!("{}/{}", state.qd_track, l),
                None => state.qd_track.clone(),
            }
        };
        egui::ComboBox::from_id_salt("qd_track")
            .selected_text(sel)
            .show_ui(ui, |ui| {
                for e in &entries {
                    let key = e.key();
                    let is_sel = state.qd_track == e.track_id && state.qd_layout == e.layout.layout_id;
                    if ui.selectable_label(is_sel, &key).clicked() {
                        state.qd_track = e.track_id.clone();
                        state.qd_layout = e.layout.layout_id.clone();
                    }
                }
            });
    });

    ui.separator();

    // Session mode
    ui.horizontal(|ui| {
        ui.label("Session:");
        ui.radio_value(&mut state.qd_mode, 0, "Practice");
        ui.radio_value(&mut state.qd_mode, 1, "Hotlap");
        ui.radio_value(&mut state.qd_mode, 2, "Race");
    });
    match state.qd_mode {
        0 => {
            ui.horizontal(|ui| {
                ui.label("Minutes:");
                ui.add(egui::DragValue::new(&mut state.qd_minutes).range(0..=180));
            });
        }
        2 => {
            ui.horizontal(|ui| {
                ui.label("Laps:");
                ui.add(egui::DragValue::new(&mut state.qd_laps).range(1..=999));
                ui.label("Opponents:");
                ui.add(egui::DragValue::new(&mut state.qd_opponents).range(0..=30));
                ui.label("AI level:");
                ui.add(egui::DragValue::new(&mut state.qd_ai_level).range(0..=100));
            });
        }
        _ => {}
    }

    ui.separator();
    ui.label("Conditions");
    ui.horizontal(|ui| {
        ui.label("Time (s since midnight):");
        ui.add(egui::Slider::new(&mut state.qd_time_seconds, race::TIME_MIN..=race::TIME_MAX));
    });
    ui.horizontal(|ui| {
        ui.label("Ambient C:");
        ui.add(egui::Slider::new(&mut state.qd_ambient, -10.0..=50.0));
    });
    ui.horizontal(|ui| {
        ui.label("Weather:");
        let sel = state
            .weather
            .iter()
            .find(|w| w.id == state.qd_weather)
            .map(|w| format!("{} ({})", w.name, w.id))
            .unwrap_or_else(|| if state.qd_weather.is_empty() { "select weather".to_string() } else { state.qd_weather.clone() });
        egui::ComboBox::from_id_salt("qd_weather")
            .selected_text(sel)
            .show_ui(ui, |ui| {
                for w in &state.weather {
                    let label = format!("{} ({})", w.name, w.id);
                    if ui.selectable_label(state.qd_weather == w.id, &label).clicked() {
                        state.qd_weather = w.id.clone();
                    }
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Time multiplier:");
        ui.add(egui::Slider::new(&mut state.qd_time_mult, 0.1..=10.0));
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.qd_penalties, "Penalties");
        ui.checkbox(&mut state.qd_fixed_setup, "Fixed setup");
        ui.checkbox(&mut state.qd_use_mph, "Use mph");
    });

    ui.separator();
    if ui.button("Go!").clicked() {
        launch_quick_drive(state);
    }
    if !state.qd_status.is_empty() {
        ui.label(&state.qd_status);
    }
}

fn launch_quick_drive(state: &mut AppState) {
    let ac_root = match state.settings.ac_root() {
        Some(r) => r,
        None => {
            state.qd_status = "AC root not found".into();
            return;
        }
    };
    let car_id = state.qd_car.clone();
    if car_id.is_empty() {
        state.qd_status = "Select a car".into();
        return;
    }
    let track_id = state.qd_track.clone();
    if track_id.is_empty() {
        state.qd_status = "Select a track".into();
        return;
    }
    let skin_id = if state.qd_skin.is_empty() {
        state
            .cars
            .iter()
            .find(|c| c.id == car_id)
            .and_then(|c| c.skins.first())
            .map(|s| s.id.clone())
            .unwrap_or_default()
    } else {
        state.qd_skin.clone()
    };

    let skins: Vec<String> = state
        .cars
        .iter()
        .find(|c| c.id == car_id)
        .map(|c| c.skins.iter().map(|s| s.id.clone()).collect())
        .unwrap_or_default();

    let mode = match state.qd_mode {
        0 => Mode::Practice { minutes: state.qd_minutes },
        1 => Mode::Hotlap,
        _ => Mode::Race {
            laps: state.qd_laps,
            opponents: state.qd_opponents,
            ai_level: state.qd_ai_level,
            starting_position: 1,
        },
    };

    let mut qd = QuickDrive {
        car_id: car_id.clone(),
        skin_id: skin_id.clone(),
        track_id: track_id.clone(),
        layout_id: state.qd_layout.clone(),
        driver_name: "Player".into(),
        nationality: "ITA".into(),
        mode,
        conditions: Conditions {
            time_seconds: state.qd_time_seconds,
            ambient_c: state.qd_ambient,
            weather_id: state.qd_weather.clone(),
            time_multiplier: state.qd_time_mult,
        },
        penalties: state.qd_penalties,
        fixed_setup: state.qd_fixed_setup,
        use_mph: state.qd_use_mph,
        opponents: Vec::new(),
    };

    // Generate opponents for race mode.
    if matches!(qd.mode, Mode::Race { .. }) {
        let count = match qd.mode {
            Mode::Race { opponents, .. } => opponents,
            _ => 0,
        };
        qd.opponents = race::pick_opponents(&car_id, &skin_id, &skins, count);
    }

    // Save settings.
    state.settings.last_car = Some(car_id.clone());
    state.settings.last_track = Some(track_id.clone());
    state.settings.last_skin = Some(skin_id.clone());
    state.settings.last_session = Some(qd.clone());
    let _ = state.settings.save();

    // Write race.ini and launch.
    match launch::write_race_ini(&ac_root, &qd) {
        Ok(path) => {
            state.qd_status = format!("race.ini written: {}", path.display());
        }
        Err(e) => {
            state.qd_status = format!("write_race_ini failed: {e:#}");
            return;
        }
    }

    let opts = LaunchOptions {
        proton: state.settings.proton.clone(),
    };
    match launch::start(&ac_root, &opts) {
        Ok(_child) => {
            state.qd_status = "Launched!".into();
        }
        Err(e) => {
            state.qd_status = format!("launch failed: {e:#}");
        }
    }
}