use std::path::PathBuf;
use std::sync::mpsc;
use std::time::SystemTime;

use officina_core::{content, install, settings::Settings};

pub enum AppMsg {
    ScanDone {
        cars: Vec<content::Car>,
        tracks: Vec<content::Track>,
        weather: Vec<content::Weather>,
    },
    InstallPlan {
        archive: PathBuf,
        items: Vec<install::Item>,
    },
    InstallDone {
        written: usize,
    },
    ToolDone(String),
    CareerLoaded(
        officina_core::events::Catalog,
        officina_core::events::Progress,
    ),
    GameConfigLoaded(officina_core::game_cfg::GameConfig),
    CareerResultImported {
        progress: officina_core::events::Progress,
        message: String,
    },
    PatchesDone(String),
    Rescan,
    Status {
        target: StatusTarget,
        message: String,
    },
    Warning(String),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTarget {
    Settings,
    CheckMods,
    Career,
    Setups,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    #[default]
    Cars,
    Tracks,
    Weather,
    CheckMods,
    Patches,
    Career,
    GameSettings,
    QuickDrive,
    Install,
    Settings,
    About,
    Setups,
    Results,
}

pub struct AppState {
    pub settings: Settings,
    pub nav: Nav,
    pub cars: Vec<content::Car>,
    pub tracks: Vec<content::Track>,
    pub weather: Vec<content::Weather>,
    pub scanning: bool,
    pub status: String,

    pub car_filter: String,
    pub selected_car: Option<String>,
    pub track_filter: String,

    // Install view state
    pub install_archive: Option<PathBuf>,
    pub install_plan: Option<Vec<install::Item>>,
    pub install_busy: bool,
    pub install_status: String,
    pub confirm_overwrite: bool,

    // GitHub tool install state
    pub tool_repo: String,

    // Quick drive state
    pub qd_car: String,
    pub qd_skin: String,
    pub qd_track: String,
    pub qd_layout: Option<String>,
    pub qd_mode: u8, // 0 practice,1 hotlap,2 race
    pub qd_laps: u32,
    pub qd_opponents: u32,
    pub qd_ai_level: u32,
    pub qd_minutes: u32,
    pub qd_time_seconds: u32,
    pub qd_ambient: f64,
    pub qd_weather: String,
    pub qd_time_mult: f64,
    pub qd_penalties: bool,
    pub qd_fixed_setup: bool,
    pub qd_use_mph: bool,
    pub qd_setup: String,
    pub qd_status: String,

    // Game settings view state
    pub game_config: Option<officina_core::game_cfg::GameConfig>,
    pub game_settings_loading: bool,
    pub game_settings_tab: usize,
    pub game_settings_status: String,

    // Results view state
    pub results_paths: Option<Vec<PathBuf>>,

    // Check mods view state
    pub check_mods_status: String,
    // Patches view state
    pub patches_status: String,

    // Gamepad state
    pub gamepad: crate::gamepad::GamepadState,

    // Career view state
    pub career_catalog: Option<officina_core::events::Catalog>,
    pub career_progress: Option<officina_core::events::Progress>,
    pub career_loading: bool,
    pub career_status: String,
    pub career_pending: Option<(String, Option<String>)>,
    pub career_last_launched: Option<String>,
    pub career_launch_time: Option<SystemTime>,
    pub career_auto_import_pending: bool,

    pub rx: Option<mpsc::Receiver<AppMsg>>,
    pub tx: mpsc::Sender<AppMsg>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let (settings, settings_warning) = Settings::load();
        let mut s = Self {
            settings,
            nav: Nav::default(),
            cars: Vec::new(),
            tracks: Vec::new(),
            weather: Vec::new(),
            scanning: false,
            status: "Ready".into(),
            car_filter: String::new(),
            selected_car: None,
            track_filter: String::new(),
            install_archive: None,
            install_plan: None,
            install_busy: false,
            install_status: String::new(),
            confirm_overwrite: false,
            tool_repo: "ac-custom-shaders-patch/app-paintshop".into(),
            qd_car: String::new(),
            qd_skin: String::new(),
            qd_track: String::new(),
            qd_layout: None,
            qd_mode: 0,
            qd_laps: 5,
            qd_opponents: 7,
            qd_ai_level: 90,
            qd_minutes: 10,
            qd_time_seconds: 13 * 3600,
            qd_ambient: 26.0,
            qd_weather: "3_clear".into(),
            qd_time_mult: 1.0,
            qd_penalties: true,
            qd_fixed_setup: false,
            qd_use_mph: false,
            qd_setup: String::new(),
            qd_status: String::new(),
            game_config: None,
            game_settings_loading: false,
            game_settings_tab: 0,
            game_settings_status: String::new(),
            results_paths: None,
            check_mods_status: String::new(),
            patches_status: String::new(),
            gamepad: crate::gamepad::GamepadState::new(),
            career_catalog: None,
            career_progress: None,
            career_loading: false,
            career_status: String::new(),
            career_pending: None,
            career_last_launched: None,
            career_launch_time: None,
            career_auto_import_pending: false,
            rx: Some(rx),
            tx,
        };
        if let Some(warning) = settings_warning {
            s.status = warning;
        }
        // Restore last session if present.
        if let Some(qd) = s.settings.last_session.clone() {
            s.qd_car = qd.car_id.clone();
            s.qd_skin = qd.skin_id.clone();
            s.qd_track = qd.track_id.clone();
            s.qd_layout = qd.layout_id.clone();
            s.qd_penalties = qd.penalties;
            s.qd_fixed_setup = qd.fixed_setup;
            s.qd_use_mph = qd.use_mph;
            s.qd_setup = qd.setup.clone();
            s.qd_time_seconds = qd.conditions.time_seconds;
            s.qd_ambient = qd.conditions.ambient_c;
            s.qd_weather = qd.conditions.weather_id.clone();
            s.qd_time_mult = qd.conditions.time_multiplier;
            if let officina_core::race::Mode::Practice { minutes } = qd.mode {
                s.qd_mode = 0;
                s.qd_minutes = minutes;
            } else if matches!(qd.mode, officina_core::race::Mode::Hotlap) {
                s.qd_mode = 1;
            } else if let officina_core::race::Mode::Race { laps, .. } = qd.mode {
                s.qd_mode = 2;
                s.qd_laps = laps;
            }
        }
        if s.settings.last_session.is_none() {
            s.qd_car = s.settings.last_car.clone().unwrap_or_default();
            s.qd_track = s.settings.last_track.clone().unwrap_or_default();
            s.qd_skin = s.settings.last_skin.clone().unwrap_or_default();
        }
        s
    }

    pub fn load_game_config(&mut self) {
        if self.game_config.is_some() || self.game_settings_loading {
            return;
        }
        let ac_root = match self.settings.ac_root() {
            Some(r) => r,
            None => {
                self.game_settings_status = "AC root not found".into();
                return;
            }
        };
        self.game_settings_loading = true;
        self.game_settings_status = "Loading...".into();
        let tx = self.tx.clone();
        std::thread::spawn(move || match officina_core::game_cfg::load(&ac_root) {
            Ok(gc) => {
                let _ = tx.send(AppMsg::GameConfigLoaded(gc));
            }
            Err(e) => {
                let _ = tx.send(AppMsg::Error(format!("game config load failed: {e:#}")));
            }
        });
    }

    pub fn load_career(&mut self) {
        if self.career_catalog.is_some() || self.career_loading {
            return;
        }
        let ac_root = match self.settings.ac_root() {
            Some(r) => r,
            None => {
                self.career_status = "AC root not found".into();
                return;
            }
        };
        self.career_loading = true;
        self.career_status = "Loading...".into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let catalog = officina_core::events::scan(&ac_root);
            let progress = match officina_core::events::Progress::load(&ac_root) {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(AppMsg::Warning(format!("Career progress: {e:#}")));
                    officina_core::events::Progress::default()
                }
            };
            let _ = tx.send(AppMsg::CareerLoaded(catalog, progress));
        });
    }

    pub fn save_settings(&mut self) {
        if let Err(e) = self.settings.save() {
            self.status = format!("Settings save failed: {e:#}");
        }
    }

    pub fn trash_path(&mut self, path: PathBuf, success: &'static str) {
        match officina_core::trash::move_to_trash(&path) {
            Ok(_) => {
                self.status = success.into();
                self.trigger_scan();
            }
            Err(e) => self.status = format!("Delete failed: {e}"),
        }
    }

    pub fn run_status_task<F>(&self, target: StatusTarget, task: F)
    where
        F: FnOnce() -> String + Send + 'static,
    {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let message = task();
            let _ = tx.send(AppMsg::Status { target, message });
        });
    }

    pub fn try_auto_import_career_result(&mut self) {
        if !self.career_auto_import_pending {
            return;
        }
        let (Some(event_id), Some(since), Some(root)) = (
            self.career_last_launched.clone(),
            self.career_launch_time,
            self.settings.ac_root(),
        ) else {
            return;
        };
        let Some(newest) = officina_core::results::list_results(&root)
            .into_iter()
            .next()
        else {
            return;
        };
        let Ok(modified) = std::fs::metadata(&newest).and_then(|m| m.modified()) else {
            return;
        };
        if modified <= since {
            return;
        }
        self.career_auto_import_pending = false;
        self.import_career_result(event_id);
    }

    pub fn refresh_results(&mut self) {
        self.results_paths = self
            .settings
            .ac_root()
            .map(|root| officina_core::results::list_results(&root));
    }

    pub fn import_career_result(&mut self, event_id: String) {
        let Some(catalog) = self.career_catalog.clone() else {
            self.career_status = "Career data not loaded.".into();
            return;
        };
        let Some(root) = self.settings.ac_root() else {
            self.career_status = "AC root not found.".into();
            return;
        };
        let found = catalog
            .series
            .iter()
            .find_map(|s| {
                s.events
                    .iter()
                    .find(|e| e.id == event_id)
                    .map(|e| (e.clone(), Some(s.clone())))
            })
            .or_else(|| {
                catalog
                    .special
                    .iter()
                    .find(|e| e.id == event_id)
                    .map(|e| (e.clone(), None))
            });
        let Some((event, series)) = found else {
            self.career_status = "Event not found.".into();
            return;
        };
        let mut progress = self.career_progress.clone().unwrap_or_default();
        self.career_status = "Looking for result...".into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let outcome = (|| -> anyhow::Result<(officina_core::events::Progress, String)> {
                let path = officina_core::events::latest_result_for(&root, &event)?;
                let message = officina_core::events::import_result(
                    &root,
                    &mut progress,
                    &event,
                    series.as_ref(),
                    &path,
                )?;
                Ok((progress, message))
            })();
            match outcome {
                Ok((progress, message)) => {
                    let _ = tx.send(AppMsg::CareerResultImported { progress, message });
                }
                Err(e) => {
                    let _ = tx.send(AppMsg::Error(format!("import result failed: {e:#}")));
                }
            }
        });
    }

    pub fn trigger_scan(&mut self) {
        if self.scanning {
            return;
        }
        self.scanning = true;
        self.status = "Scanning...".into();
        let tx = self.tx.clone();
        let ac_root = self.settings.ac_root();
        std::thread::spawn(move || match ac_root {
            Some(root) => {
                let cars = content::scan_cars(&root);
                let tracks = content::scan_tracks(&root);
                let weather = content::scan_weather(&root);
                let _ = tx.send(AppMsg::ScanDone {
                    cars,
                    tracks,
                    weather,
                });
            }
            None => {
                let _ = tx.send(AppMsg::Error("Assetto Corsa not found".into()));
            }
        });
    }

    pub fn plan_install(&mut self, archive: PathBuf) {
        if self.install_busy {
            return;
        }
        self.install_busy = true;
        self.install_status = "Planning...".into();
        let tx = self.tx.clone();
        let ac_root = self.settings.ac_root();
        std::thread::spawn(move || match ac_root {
            Some(root) => match install::plan(&archive, &root) {
                Ok(items) => {
                    let _ = tx.send(AppMsg::InstallPlan { archive, items });
                }
                Err(e) => {
                    let _ = tx.send(AppMsg::Error(format!("plan failed: {e:#}")));
                }
            },
            None => {
                let _ = tx.send(AppMsg::Error("Assetto Corsa not found".into()));
            }
        });
    }

    pub fn install_tool(&mut self, source: String) {
        if self.install_busy {
            return;
        }
        self.tool_repo = source.clone();
        self.install_busy = true;
        self.install_status = "Downloading tool...".into();
        let tx = self.tx.clone();
        let ac_root = self.settings.ac_root();
        std::thread::spawn(move || {
            let msg = match ac_root {
                Some(root) => match officina_core::install::install_tool(&root, &source) {
                    Ok(written) => format!("Tool installed: {written} file(s)."),
                    Err(e) => format!("Tool install failed: {e:#}"),
                },
                None => "AC root not found".into(),
            };
            let _ = tx.send(AppMsg::ToolDone(msg));
        });
    }

    pub fn run_install(&mut self) {
        let archive = match &self.install_archive {
            Some(a) => a.clone(),
            None => return,
        };
        let items = match &self.install_plan {
            Some(i) => i.clone(),
            None => return,
        };
        if self.install_busy {
            return;
        }
        self.install_busy = true;
        self.install_status = "Installing...".into();
        let tx = self.tx.clone();
        let options = install::InstallOptions {
            allow_overwrite: true,
            allow_root_overwrite: true,
        };
        std::thread::spawn(move || {
            match install::install_with_options(&archive, &items, options) {
                Ok(written) => {
                    let _ = tx.send(AppMsg::InstallDone { written });
                }
                Err(e) => {
                    let _ = tx.send(AppMsg::Error(format!("install failed: {e:#}")));
                }
            }
        });
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
