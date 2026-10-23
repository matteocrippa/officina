use std::path::PathBuf;
use std::sync::mpsc;

use ac_core::{content, install, settings::Settings};

#[derive(Debug, Clone)]
pub enum AppMsg {
    ScanDone {
        cars: Vec<content::Car>,
        tracks: Vec<content::Track>,
    },
    InstallPlan {
        archive: PathBuf,
        items: Vec<install::Item>,
    },
    InstallDone {
        written: usize,
    },
    Error(String),
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    #[default]
    Cars,
    Tracks,
    QuickDrive,
    Install,
    Settings,
}

pub struct AppState {
    pub settings: Settings,
    pub nav: Nav,
    pub cars: Vec<content::Car>,
    pub tracks: Vec<content::Track>,
    pub scanning: bool,
    pub status: String,

    pub car_filter: String,
    pub track_filter: String,

    // Install view state
    pub install_archive: Option<PathBuf>,
    pub install_plan: Option<Vec<install::Item>>,
    pub install_busy: bool,
    pub install_status: String,

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
    pub qd_status: String,

    pub rx: Option<mpsc::Receiver<AppMsg>>,
    pub tx: mpsc::Sender<AppMsg>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let settings = Settings::load();
        let mut s = Self {
            settings,
            nav: Nav::default(),
            cars: Vec::new(),
            tracks: Vec::new(),
            scanning: false,
            status: "Ready".into(),
            car_filter: String::new(),
            track_filter: String::new(),
            install_archive: None,
            install_plan: None,
            install_busy: false,
            install_status: String::new(),
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
            qd_status: String::new(),
            rx: Some(rx),
            tx,
        };
        // Restore last session if present.
        if let Some(qd) = s.settings.last_session.clone() {
            s.qd_car = qd.car_id.clone();
            s.qd_skin = qd.skin_id.clone();
            s.qd_track = qd.track_id.clone();
            s.qd_layout = qd.layout_id.clone();
            s.qd_penalties = qd.penalties;
            s.qd_fixed_setup = qd.fixed_setup;
            s.qd_use_mph = qd.use_mph;
            s.qd_time_seconds = qd.conditions.time_seconds;
            s.qd_ambient = qd.conditions.ambient_c;
            s.qd_weather = qd.conditions.weather_id.clone();
            s.qd_time_mult = qd.conditions.time_multiplier;
            if let ac_core::race::Mode::Practice { minutes } = qd.mode {
                s.qd_mode = 0;
                s.qd_minutes = minutes;
            } else if matches!(qd.mode, ac_core::race::Mode::Hotlap) {
                s.qd_mode = 1;
            } else if let ac_core::race::Mode::Race { laps, .. } = qd.mode {
                s.qd_mode = 2;
                s.qd_laps = laps;
            }
        }
        s
    }

    pub fn trigger_scan(&mut self) {
        if self.scanning {
            return;
        }
        self.scanning = true;
        self.status = "Scanning...".into();
        let tx = self.tx.clone();
        let ac_root = self.settings.ac_root();
        std::thread::spawn(move || {
            match ac_root {
                Some(root) => {
                    let cars = content::scan_cars(&root);
                    let tracks = content::scan_tracks(&root);
                    let _ = tx.send(AppMsg::ScanDone { cars, tracks });
                }
                None => {
                    let _ = tx.send(AppMsg::Error("Assetto Corsa not found".into()));
                }
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
        std::thread::spawn(move || {
            match ac_root {
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
            }
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
        std::thread::spawn(move || {
            match install::install(&archive, &items) {
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