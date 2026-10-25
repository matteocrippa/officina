use egui::Ui;
use officina_core::game_cfg::{self, Kind, Setting, Tab};

use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui) {
    ui.heading("Game Settings");
    state.load_game_config();

    if state.game_settings_loading {
        ui.spinner();
        ui.label(&state.game_settings_status);
        return;
    }
    let gc = match state.game_config.as_mut() {
        Some(g) => g,
        None => {
            ui.label(&state.game_settings_status);
            if ui.button("Retry").clicked() {
                state.game_config = None;
                state.game_settings_loading = false;
                state.load_game_config();
            }
            return;
        }
    };

    // Tab selector
    ui.horizontal(|ui| {
        for (i, tab) in Tab::ALL.iter().enumerate() {
            if ui
                .selectable_label(state.game_settings_tab == i, tab.label())
                .clicked()
            {
                state.game_settings_tab = i;
            }
        }
    });
    let selected_tab = Tab::ALL[state.game_settings_tab];

    let all = game_cfg::settings();
    // Group settings for selected tab, preserving group order.
    let mut groups: Vec<(&'static str, Vec<&Setting>)> = Vec::new();
    for s in &all {
        if s.tab != selected_tab {
            continue;
        }
        if let Some(g) = groups.iter_mut().find(|(name, _)| *name == s.group) {
            g.1.push(s);
        } else {
            groups.push((s.group, vec![s]));
        }
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (group, settings) in &groups {
            ui.collapsing(*group, |ui| {
                for s in settings {
                    render_setting(gc, s, ui);
                }
            });
        }
    });

    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            match gc.save() {
                Ok(dirty) => {
                    state.game_settings_status = if dirty.is_empty() {
                        "No changes.".into()
                    } else {
                        format!("Saved: {}", dirty.join(", "))
                    };
                }
                Err(e) => state.game_settings_status = format!("Save failed: {e:#}"),
            }
        }
        ui.label(&state.game_settings_status);
    });
}

fn render_setting(gc: &mut game_cfg::GameConfig, s: &Setting, ui: &mut Ui) {
    match &s.kind {
        Kind::Toggle => {
            let mut v = gc.get(s.file, s.section, s.key).unwrap_or("0") == "1";
            if ui.checkbox(&mut v, s.label).changed() {
                gc.set(s.file, s.section, s.key, if v { "1" } else { "0" });
            }
        }
        Kind::Choice(opts) => {
            let cur = gc.get(s.file, s.section, s.key).unwrap_or("").to_string();
            ui.horizontal(|ui| {
                ui.label(s.label);
                egui::ComboBox::from_id_salt(format!("{}-{}-{}", s.file, s.section, s.key))
                    .selected_text(
                        opts.iter()
                            .find(|(v, _)| *v == cur)
                            .map(|(_, l)| *l)
                            .unwrap_or(cur.as_str()),
                    )
                    .show_ui(ui, |ui| {
                        for (v, l) in *opts {
                            if ui.selectable_label(cur == *v, *l).clicked() {
                                gc.set(s.file, s.section, s.key, *v);
                            }
                        }
                    });
            });
        }
        Kind::Filter => {
            let cur = gc.get(s.file, s.section, s.key).unwrap_or("").to_string();
            ui.horizontal(|ui| {
                ui.label(s.label);
                egui::ComboBox::from_id_salt(format!("filt-{}-{}-{}", s.file, s.section, s.key))
                    .selected_text(if cur.is_empty() {
                        "none".into()
                    } else {
                        cur.clone()
                    })
                    .show_ui(ui, |ui| {
                        for f in gc.filters.clone() {
                            if ui.selectable_label(cur == f, &f).clicked() {
                                gc.set(s.file, s.section, s.key, &f);
                            }
                        }
                    });
            });
        }
        Kind::Number {
            min,
            max,
            step,
            scale,
            suffix,
        } => {
            let mut val = gc.number(s.file, s.section, s.key);
            if val == 0. && gc.get(s.file, s.section, s.key).is_none() {
                val = *min;
            }
            ui.horizontal(|ui| {
                ui.label(s.label);
                if ui
                    .add(egui::Slider::new(&mut val, *min..=*max).step_by(*step))
                    .changed()
                {
                    gc.set(s.file, s.section, s.key, game_cfg::fmt_number(val));
                }
                ui.label(format!("{:.0}{}", val * scale, suffix));
            });
        }
    }
}
