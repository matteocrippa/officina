use egui::Ui;

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Race Results");

    if state.results_paths.is_none() {
        state.refresh_results();
    }
    let results = state.results_paths.clone().unwrap_or_default();

    ui.label(format!("{} result file(s)", results.len()));

    if ui.button("Rescan").clicked() {
        state.refresh_results();
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for r in results.iter().take(50) {
            match officina_core::results::parse_result(r) {
                Ok(res) => {
                    ui.group(|ui| {
                        ui.label(format!(
                            "{}  —  {}  ({})",
                            if res.track.is_empty() {
                                r.file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default()
                            } else {
                                res.track.clone()
                            },
                            res.car,
                            res.session
                        ));
                        if !res.player.is_empty() {
                            ui.small(format!("Driver: {}", res.player));
                        }
                        if let Some(b) = res.best_lap {
                            ui.small(format!(
                                "Best lap: {}",
                                officina_core::results::format_time(b)
                            ));
                        }
                        if let Some(t) = res.total_time {
                            ui.small(format!("Total: {}", officina_core::results::format_time(t)));
                        }
                        if !res.laps.is_empty() {
                            ui.small(format!("Laps: {}", res.laps.len()));
                            egui::CollapsingHeader::new("Lap times").show(ui, |ui| {
                                for (i, lap) in res.laps.iter().enumerate() {
                                    ui.label(format!(
                                        "  Lap {}: {} {}",
                                        i + 1,
                                        officina_core::results::format_time(lap.time),
                                        if lap.valid { "" } else { "(invalid)" }
                                    ));
                                }
                            });
                        }
                    });
                }
                Err(_) => {
                    ui.label(format!(
                        "Unparsed: {}",
                        r.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    ));
                }
            }
        }
    });
}
