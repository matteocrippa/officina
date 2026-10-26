use crate::state::{AppMsg, AppState, Nav};
use crate::views;

pub struct App {
    pub state: AppState,
}

impl Default for App {
    fn default() -> Self {
        let mut s = AppState::new();
        if s.settings.auto_update_csp {
            let tx = s.tx.clone();
            if let Some(root) = s.settings.ac_root() {
                std::thread::spawn(move || {
                    if let Ok(n) = officina_core::csp_config::update_csp_configs(&root) {
                        let _ = tx.send(AppMsg::PatchesDone(format!(
                            "Auto-updated CSP configs: {n} file(s)."
                        )));
                    }
                });
            }
        }
        // File watcher: poll content dirs and trigger rescan on change.
        if let Some(root) = s.settings.ac_root() {
            let tx = s.tx.clone();
            std::thread::spawn(move || {
                use std::collections::BTreeSet;
                use std::fs;
                use std::time::Duration;
                let mut last: Option<BTreeSet<(String, u64)>> = None;
                loop {
                    let mut sig = BTreeSet::new();
                    for sub in ["cars", "tracks", "weather"] {
                        let dir = root.join("content").join(sub);
                        if let Ok(meta) = fs::metadata(&dir) {
                            let mt = meta
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                            sig.insert((format!("{sub}/."), mt));
                        }
                        if let Ok(entries) = fs::read_dir(&dir) {
                            for e in entries.flatten() {
                                if let Ok(m) = fs::metadata(e.path()) {
                                    let mt = m
                                        .modified()
                                        .ok()
                                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                        .map(|d| d.as_secs())
                                        .unwrap_or(0);
                                    sig.insert((
                                        format!("{}/{}", sub, e.file_name().to_string_lossy()),
                                        mt,
                                    ));
                                }
                            }
                        }
                    }
                    if let Some(prev) = &last
                        && *prev != sig
                    {
                        let _ = tx.send(AppMsg::Rescan);
                    }
                    last = Some(sig);
                    std::thread::sleep(Duration::from_secs(5));
                }
            });
        }
        s.trigger_scan();
        Self { state: s }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let visuals = if self.state.settings.theme == "light" {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        ctx.set_visuals(visuals);
        self.state.gamepad.poll(ctx);
        // Poll async messages.
        let mut msgs = Vec::new();
        if let Some(rx) = &self.state.rx {
            while let Ok(msg) = rx.try_recv() {
                msgs.push(msg);
            }
        }
        for msg in msgs {
            match msg {
                AppMsg::ScanDone {
                    cars,
                    tracks,
                    weather,
                } => {
                    self.state.cars = cars;
                    self.state.tracks = tracks;
                    self.state.weather = weather;
                    self.state.scanning = false;
                    self.state.status = format!(
                        "{} cars, {} tracks, {} weather",
                        self.state.cars.len(),
                        self.state.tracks.len(),
                        self.state.weather.len()
                    );
                    // Default qd selections.
                    if self.state.qd_car.is_empty()
                        && let Some(c) = self.state.cars.first()
                    {
                        self.state.qd_car = c.id.clone();
                        if let Some(s) = c.skins.first() {
                            self.state.qd_skin = s.id.clone();
                        }
                    }
                    if self.state.qd_track.is_empty() {
                        let entries = officina_core::content::track_entries(&self.state.tracks);
                        if let Some(e) = entries.first() {
                            self.state.qd_track = e.track_id.clone();
                            self.state.qd_layout = e.layout.layout_id.clone();
                        }
                    }
                    if self.state.qd_weather.is_empty() {
                        if let Some(w) = self.state.weather.first() {
                            self.state.qd_weather = w.id.clone();
                        }
                    } else if !self
                        .state
                        .weather
                        .iter()
                        .any(|w| w.id == self.state.qd_weather)
                        && let Some(w) = self.state.weather.first()
                    {
                        self.state.qd_weather = w.id.clone();
                    }
                }
                AppMsg::InstallPlan { archive, items } => {
                    self.state.install_archive = Some(archive);
                    self.state.install_plan = Some(items);
                    self.state.install_busy = false;
                    self.state.install_status = "Plan ready. Click 'Install now'.".into();
                }
                AppMsg::InstallDone { written } => {
                    self.state.install_busy = false;
                    self.state.install_status = format!("Installed {} file(s).", written);
                    self.state.trigger_scan();
                }
                AppMsg::CareerLoaded(cat, prog) => {
                    self.state.career_catalog = Some(cat);
                    self.state.career_progress = Some(prog);
                    self.state.career_loading = false;
                    self.state.career_status = "Loaded.".into();
                }
                AppMsg::GameConfigLoaded(gc) => {
                    self.state.game_config = Some(gc);
                    self.state.game_settings_loading = false;
                    self.state.game_settings_status = "Loaded.".into();
                }
                AppMsg::PatchesDone(msg) => {
                    self.state.install_busy = false;
                    self.state.patches_status = msg;
                }
                AppMsg::Rescan => {
                    self.state.trigger_scan();
                }
                AppMsg::Error(e) => {
                    self.state.scanning = false;
                    self.state.install_busy = false;
                    self.state.status = e.clone();
                    self.state.install_status = e;
                }
            }
        }

        egui::SidePanel::left("nav").show(ctx, |ui| {
            ui.heading("Officina");
            ui.separator();
            for (label, nav) in [
                ("Cars", Nav::Cars),
                ("Tracks", Nav::Tracks),
                ("Weather", Nav::Weather),
                ("Check Mods", Nav::CheckMods),
                ("Patches", Nav::Patches),
                ("Career", Nav::Career),
                ("Game Settings", Nav::GameSettings),
                ("Quick Drive", Nav::QuickDrive),
                ("Install", Nav::Install),
                ("Settings", Nav::Settings),
                ("About", Nav::About),
                ("Setups", Nav::Setups),
            ] {
                let entry = format!("{}  {}", crate::icons::nav(nav), label);
                if ui.selectable_label(self.state.nav == nav, entry).clicked() {
                    self.state.nav = nav;
                }
            }
            ui.separator();
            ui.label(&self.state.status);
            if self.state.scanning || self.state.install_busy {
                ui.spinner();
            }
            ui.small(format!("gamepad: {}", self.state.gamepad.status));
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.state.nav {
            Nav::Cars => views::cars::show(&mut self.state, ui),
            Nav::Tracks => views::tracks::show(&mut self.state, ui),
            Nav::Weather => views::weather::show(&mut self.state, ui),
            Nav::CheckMods => views::check_mods::show(&mut self.state, ui),
            Nav::Patches => views::patches::show(&mut self.state, ui, ctx),
            Nav::Career => views::career::show(&mut self.state, ui),
            Nav::GameSettings => views::game_settings::show(&mut self.state, ui),
            Nav::QuickDrive => views::quick_drive::show(&mut self.state, ui),
            Nav::Install => views::install::show(&mut self.state, ui, ctx),
            Nav::Settings => views::settings::show(&mut self.state, ui),
            Nav::About => views::about::show(&mut self.state, ui),
            Nav::Setups => views::setups::show(&mut self.state, ui),
        });

        if self.state.scanning || self.state.install_busy {
            ctx.request_repaint();
        }
    }
}
