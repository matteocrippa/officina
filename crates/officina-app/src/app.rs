use crate::state::{AppState, AppMsg, Nav};
use crate::views;

pub struct App {
    pub state: AppState,
}

impl Default for App {
    fn default() -> Self {
        let mut s = AppState::new();
        s.trigger_scan();
        Self { state: s }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    AppMsg::ScanDone { cars, tracks, weather } => {
                        self.state.cars = cars;
                        self.state.tracks = tracks;
                        self.state.weather = weather;
                        self.state.scanning = false;
                        self.state.status = format!("{} cars, {} tracks, {} weather", self.state.cars.len(), self.state.tracks.len(), self.state.weather.len());
                        // Default qd selections.
                        if self.state.qd_car.is_empty() {
                            if let Some(c) = self.state.cars.first() {
                                self.state.qd_car = c.id.clone();
                                if let Some(s) = c.skins.first() {
                                    self.state.qd_skin = s.id.clone();
                                }
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
                        } else if !self.state.weather.iter().any(|w| w.id == self.state.qd_weather) {
                            if let Some(w) = self.state.weather.first() {
                                self.state.qd_weather = w.id.clone();
                            }
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
        });

        if self.state.scanning || self.state.install_busy {
            ctx.request_repaint();
        }
    }
}