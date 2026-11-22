use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::state::{AppMsg, AppState, Nav, StatusTarget};
use crate::views;

pub struct App {
    pub state: AppState,
    shutdown: Arc<AtomicBool>,
}

impl Drop for App {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
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
        let shutdown = Arc::new(AtomicBool::new(false));
        // File watcher: poll content dirs and trigger rescan on change.
        if let Some(root) = s.settings.ac_root() {
            let tx = s.tx.clone();
            let stop = shutdown.clone();
            std::thread::spawn(move || {
                use std::collections::BTreeSet;
                use std::fs;
                use std::time::Duration;

                fn modified_millis(path: &std::path::Path) -> u128 {
                    fs::metadata(path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis())
                        .unwrap_or(0)
                }

                fn snapshot(root: &std::path::Path) -> BTreeSet<(String, u128)> {
                    let mut sig = BTreeSet::new();
                    for sub in ["cars", "tracks", "weather"] {
                        let dir = root.join("content").join(sub);
                        sig.insert((format!("{sub}/."), modified_millis(&dir)));
                        if let Ok(entries) = fs::read_dir(&dir) {
                            for e in entries.flatten() {
                                let path = e.path();
                                sig.insert((
                                    format!("{}/{}", sub, e.file_name().to_string_lossy()),
                                    modified_millis(&path),
                                ));
                            }
                        }
                    }
                    sig
                }

                let mut last = snapshot(&root);
                while !stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_secs(5));
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let current = snapshot(&root);
                    if current != last {
                        let _ = tx.send(AppMsg::Rescan);
                        last = current;
                    }
                }
            });
        }
        s.trigger_scan();
        Self { state: s, shutdown }
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
                    // Default qd selections to the first available item when the
                    // restored selection is empty or stale.
                    let car_valid = self.state.cars.iter().any(|c| c.id == self.state.qd_car);
                    if !car_valid && let Some(c) = self.state.cars.first() {
                        self.state.qd_car = c.id.clone();
                        self.state.qd_skin =
                            c.skins.first().map(|s| s.id.clone()).unwrap_or_default();
                    }
                    let track_valid = !self.state.qd_track.is_empty()
                        && officina_core::content::track_entries(&self.state.tracks)
                            .iter()
                            .any(|e| e.track_id == self.state.qd_track);
                    if !track_valid {
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
                    self.state.confirm_overwrite = false;
                    self.state.install_status = "Plan ready. Click 'Install now'.".into();
                }
                AppMsg::InstallDone { written } => {
                    self.state.install_busy = false;
                    self.state.install_status = format!("Installed {} file(s).", written);
                    self.state.trigger_scan();
                }
                AppMsg::ToolDone(msg) => {
                    self.state.install_busy = false;
                    self.state.install_status = msg.clone();
                    self.state.trigger_scan();
                }
                AppMsg::CareerLoaded(cat, prog) => {
                    self.state.career_catalog = Some(cat);
                    self.state.career_progress = Some(prog);
                    self.state.career_loading = false;
                    self.state.career_status = "Loaded.".into();
                }
                AppMsg::CareerResultImported { progress, message } => {
                    self.state.career_progress = Some(progress);
                    self.state.career_status = message;
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
                AppMsg::Status { target, message } => match target {
                    StatusTarget::Settings => self.state.game_settings_status = message,
                    StatusTarget::CheckMods => self.state.check_mods_status = message,
                    StatusTarget::Career => {
                        self.state.career_loading = false;
                        if !message.starts_with("Launched") {
                            self.state.career_auto_import_pending = false;
                        }
                        self.state.career_status = message;
                    }
                    StatusTarget::Setups => self.state.status = message,
                },
                AppMsg::Warning(w) => {
                    self.state.status = w;
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
                ("Results", Nav::Results),
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
            Nav::Results => views::results::show(&mut self.state, ui),
        });

        if self.state.scanning || self.state.install_busy {
            ctx.request_repaint();
        }
    }
}
