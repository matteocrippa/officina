use egui::Ui;
use officina_core::events;
use officina_core::launch::{self, LaunchOptions};

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Career");
    state.load_career();

    if state.career_loading {
        ui.spinner();
        ui.label(&state.career_status);
        return;
    }
    let catalog = match &state.career_catalog {
        Some(c) => c,
        None => {
            ui.label(&state.career_status);
            if ui.button("Retry").clicked() {
                state.career_catalog = None;
                state.career_loading = false;
                state.load_career();
            }
            return;
        }
    };
    let progress = state.career_progress.get_or_insert_with(|| events::Progress::default()).clone();

    ui.label(format!(
        "{} series, {} special events",
        catalog.series.len(),
        catalog.special.len()
    ));
    if !catalog.errors.is_empty() {
        for e in &catalog.errors {
            ui.colored_label(egui::Color32::RED, e);
        }
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for series in &catalog.series {
            let unlocked = series.unlocked(&progress);
            let completed = series.completed(&progress);
            ui.group(|ui| {
                ui.label(format!(
                    "{}  —  {}  [{}]",
                    series.name,
                    series.id,
                    if completed {
                        "complete"
                    } else if unlocked {
                        "unlocked"
                    } else {
                        "locked"
                    }
                ));
                if !series.description.is_empty() {
                    ui.small(&series.description);
                }
                ui.small(format!(
                    "medals: B{} S{} G{}  (goal B{} S{} G{})",
                    series.medal_counts(&progress)[0],
                    series.medal_counts(&progress)[1],
                    series.medal_counts(&progress)[2],
                    series.goal_medals[0],
                    series.goal_medals[1],
                    series.goal_medals[2],
                ));
                for ev in &series.events {
                    let p = progress.get(&ev.id);
                    ui.horizontal(|ui| {
                        ui.label(format!("  {} ({})", ev.name, ev.id));
                        ui.small(format!("medal {}", p.medal));
                        if unlocked && ui.button("Launch").clicked() {
                            state.career_pending = Some((ev.id.clone(), Some(series.id.clone())));
                        }
                    });
                }
            });
        }
        ui.separator();
        ui.heading("Special Events");
        for ev in &catalog.special {
            let p = progress.get(&ev.id);
            ui.group(|ui| {
                ui.label(format!("{}  —  {}", ev.name, ev.id));
                ui.small(format!("car: {}  track: {}  medal: {}", ev.car, ev.track, p.medal));
                if ui.button("Launch").clicked() {
                    state.career_pending = Some((ev.id.clone(), None));
                }
            });
        }
    });

    // Handle pending launch (deferred to avoid borrow conflicts in scroll closure)
    if let Some((eid, _sid)) = state.career_pending.take() {
        let ac_root = state.settings.ac_root();
        let found = state.career_catalog.as_ref().and_then(|c| {
            c.series
                .iter()
                .flat_map(|s| s.events.iter().map(move |e| (Some(s.clone()), e.clone())))
                .find(|(_, e)| e.id == eid)
                .or_else(|| {
                    c.special
                        .iter()
                        .find(|e| e.id == eid)
                        .map(|e| (None, e.clone()))
                })
        });
        if let Some((series, event)) = found {
            let series_ref = series.as_ref();
            if let Some(root) = ac_root.clone() {
                if let Err(e) = events::available(&root, &event, series_ref) {
                    state.career_status = format!("unavailable: {e:#}");
                } else {
                    match events::prepare(&root, &event, series_ref, 0, "Player", "ITA") {
                        Ok(path) => state.career_status = format!("race.ini written: {}", path.display()),
                        Err(e) => {
                            state.career_status = format!("prepare failed: {e:#}");
                        }
                    }
                    let opts = LaunchOptions {
                        proton: state.settings.proton.clone(),
                    };
                    match launch::start(&root, &opts) {
                        Ok(_) => state.career_status = "Launched!".into(),
                        Err(e) => state.career_status = format!("launch failed: {e:#}"),
                    }
                }
            } else {
                state.career_status = "AC root not found".into();
            }
        } else {
            state.career_status = "Event not found".into();
        }
    }

    ui.label(&state.career_status);
}
