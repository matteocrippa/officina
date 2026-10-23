//! In-game options Assetto Corsa reads from `Documents/Assetto Corsa/cfg`:
//! video, audio, assists, gameplay and controls (keyboard bindings).
//!
//! AC prefers the Documents copy of each file and only falls back to the
//! defaults in `<ac_root>/cfg`. Users' Documents files are often partial, so
//! a file is loaded as the defaults with the Documents values on top, and
//! saved back to Documents in full (like Content Manager does).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::ini::IniFile;
use crate::paths;

pub const FILES: [&str; 5] = ["video.ini", "audio.ini", "assists.ini", "gameplay.ini", "controls.ini"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Video,
    Audio,
    Assists,
    Gameplay,
    Controls,
    Keys,
}

impl Tab {
    pub const ALL: [Tab; 6] = [Tab::Video, Tab::Audio, Tab::Assists, Tab::Gameplay, Tab::Controls, Tab::Keys];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Video => "Video",
            Tab::Audio => "Audio",
            Tab::Assists => "Assists",
            Tab::Gameplay => "Gameplay",
            Tab::Controls => "Controls",
            Tab::Keys => "Keys",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Kind {
    Toggle,
    Choice(&'static [(&'static str, &'static str)]),
    Filter,
    Number { min: f64, max: f64, step: f64, scale: f64, suffix: &'static str },
}

#[derive(Clone, Debug)]
pub struct Setting {
    pub tab: Tab,
    pub group: &'static str,
    pub file: &'static str,
    pub section: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub kind: Kind,
}

const fn num(min: f64, max: f64, step: f64) -> Kind {
    Kind::Number { min, max, step, scale: 1., suffix: "" }
}
const fn pct(max: f64, step: f64) -> Kind {
    Kind::Number { min: 0., max, step, scale: 100., suffix: "%" }
}
const fn pct_int(step: f64) -> Kind {
    Kind::Number { min: 0., max: 100., step, scale: 1., suffix: "%" }
}

const OFF_2_4_8: &[(&str, &str)] = &[("1", "Off"), ("2", "2×"), ("4", "4×"), ("8", "8×")];
const ANISO: &[(&str, &str)] = &[("0", "Off"), ("2", "2×"), ("4", "4×"), ("8", "8×"), ("16", "16×")];
const SHADOWS: &[(&str, &str)] = &[("512", "512"), ("1024", "1024"), ("2048", "2048"), ("4096", "4096")];
const CUBEMAP: &[(&str, &str)] = &[("0", "Off"), ("256", "256"), ("512", "512"), ("1024", "1024"), ("2048", "2048")];
const MIRROR: &[(&str, &str)] = &[("256", "256"), ("512", "512"), ("1024", "1024")];
const ASSIST_LEVEL: &[(&str, &str)] = &[("0", "Off"), ("1", "Factory"), ("2", "On")];
const INPUT: &[(&str, &str)] = &[("KEYBOARD", "Keyboard"), ("X360", "Gamepad"), ("WHEEL", "Wheel")];

macro_rules! s {
    ($tab:ident, $group:literal, $file:literal, $section:literal, $key:literal, $label:literal, $kind:expr) => {
        Setting { tab: Tab::$tab, group: $group, file: $file, section: $section, key: $key, label: $label, kind: $kind }
    };
}

pub fn settings() -> Vec<Setting> {
    vec![
        s!(Video, "Display", "video.ini", "VIDEO", "FULLSCREEN", "Fullscreen", Kind::Toggle),
        s!(Video, "Display", "video.ini", "VIDEO", "VSYNC", "V-sync", Kind::Toggle),
        s!(Video, "Quality", "video.ini", "VIDEO", "AASAMPLES", "Anti-aliasing", Kind::Choice(OFF_2_4_8)),
        s!(Video, "Quality", "video.ini", "VIDEO", "ANISOTROPIC", "Anisotropic", Kind::Choice(ANISO)),
        s!(Video, "Quality", "video.ini", "VIDEO", "SHADOW_MAP_SIZE", "Shadows", Kind::Choice(SHADOWS)),
        s!(Video, "Quality", "video.ini", "ASSETTOCORSA", "WORLD_DETAIL", "World detail", num(0., 5., 1.)),
        s!(Video, "Quality", "video.ini", "EFFECTS", "SMOKE", "Smoke", num(0., 5., 1.)),
        s!(Video, "Quality", "video.ini", "EFFECTS", "MOTION_BLUR", "Motion blur", num(0., 12., 1.)),
        s!(Video, "Quality", "video.ini", "EFFECTS", "RENDER_SMOKE_IN_MIRROR", "Smoke in mirrors", Kind::Toggle),
        s!(Video, "Reflections", "video.ini", "CUBEMAP", "SIZE", "Resolution", Kind::Choice(CUBEMAP)),
        s!(Video, "Reflections", "video.ini", "CUBEMAP", "FACES_PER_FRAME", "Faces per frame", num(0., 6., 1.)),
        s!(Video, "Reflections", "video.ini", "CUBEMAP", "FARPLANE", "Render distance", Kind::Number { min: 100., max: 2000., step: 100., scale: 1., suffix: " m" }),
        s!(Video, "Reflections", "video.ini", "MIRROR", "SIZE", "Mirror resolution", Kind::Choice(MIRROR)),
        s!(Video, "Reflections", "video.ini", "MIRROR", "HQ", "HQ mirrors", Kind::Toggle),
        s!(Video, "Post-processing", "video.ini", "POST_PROCESS", "ENABLED", "Enabled", Kind::Toggle),
        s!(Video, "Post-processing", "video.ini", "POST_PROCESS", "FILTER", "Filter", Kind::Filter),
        s!(Video, "Post-processing", "video.ini", "POST_PROCESS", "QUALITY", "Quality", num(0., 5., 1.)),
        s!(Video, "Post-processing", "video.ini", "POST_PROCESS", "GLARE", "Glare", num(0., 5., 1.)),
        s!(Video, "Post-processing", "video.ini", "POST_PROCESS", "DOF", "Depth of field", num(0., 5., 1.)),
        s!(Video, "Post-processing", "video.ini", "POST_PROCESS", "FXAA", "FXAA", Kind::Toggle),
        s!(Video, "Post-processing", "video.ini", "POST_PROCESS", "HEAT_SHIMMER", "Heat shimmer", Kind::Toggle),
        s!(Video, "Post-processing", "video.ini", "POST_PROCESS", "RAYS_OF_GOD", "Sun rays", Kind::Toggle),
        s!(Video, "Post-processing", "video.ini", "SATURATION", "LEVEL", "Saturation", Kind::Number { min: 0., max: 200., step: 10., scale: 1., suffix: "%" }),
        s!(Video, "Cockpit", "video.ini", "ASSETTOCORSA", "HIDE_ARMS", "Hide arms", Kind::Toggle),
        s!(Video, "Cockpit", "video.ini", "ASSETTOCORSA", "HIDE_STEER", "Hide wheel", Kind::Toggle),
        s!(Video, "Cockpit", "video.ini", "ASSETTOCORSA", "LOCK_STEER", "Lock wheel", Kind::Toggle),
        s!(Audio, "Levels", "audio.ini", "LEVELS", "MASTER", "Master", pct(1., 0.05)),
        s!(Audio, "Levels", "audio.ini", "LEVELS", "ENGINE", "Engine", pct(1., 0.05)),
        s!(Audio, "Levels", "audio.ini", "LEVELS", "TYRES", "Tyres", pct(1., 0.05)),
        s!(Audio, "Levels", "audio.ini", "LEVELS", "SURFACES", "Surfaces", pct(1., 0.05)),
        s!(Audio, "Levels", "audio.ini", "LEVELS", "DIRT_BOTTOM", "Dirt", pct(1., 0.05)),
        s!(Audio, "Levels", "audio.ini", "LEVELS", "WIND", "Wind", pct(1., 0.05)),
        s!(Audio, "Levels", "audio.ini", "LEVELS", "OPPONENTS", "Opponents", pct(1., 0.05)),
        s!(Audio, "Levels", "audio.ini", "LEVELS", "UISOUNDS", "UI", pct(1., 0.05)),
        s!(Audio, "Levels", "audio.ini", "LEVELS", "MUSIC", "Music", pct(1., 0.05)),
        s!(Assists, "Driving aids", "assists.ini", "ASSISTS", "ABS", "ABS", Kind::Choice(ASSIST_LEVEL)),
        s!(Assists, "Driving aids", "assists.ini", "ASSISTS", "TRACTION_CONTROL", "Traction control", Kind::Choice(ASSIST_LEVEL)),
        s!(Assists, "Driving aids", "assists.ini", "ASSISTS", "STABILITY_CONTROL", "Stability control", pct_int(10.)),
        s!(Assists, "Driving aids", "assists.ini", "ASSISTS", "AUTO_SHIFTER", "Auto shifter", Kind::Toggle),
        s!(Assists, "Driving aids", "assists.ini", "ASSISTS", "AUTO_CLUTCH", "Auto clutch", Kind::Toggle),
        s!(Assists, "Driving aids", "assists.ini", "ASSISTS", "AUTO_BLIP", "Auto blip", Kind::Toggle),
        s!(Assists, "Driving aids", "assists.ini", "ASSISTS", "AUTO_BRAKE", "Auto brake", Kind::Toggle),
        s!(Assists, "Driving aids", "assists.ini", "ASSISTS", "IDEAL_LINE", "Ideal line", Kind::Toggle),
        s!(Assists, "Realism", "assists.ini", "ASSISTS", "DAMAGE", "Damage", pct_int(10.)),
        s!(Assists, "Realism", "assists.ini", "ASSISTS", "VISUALDAMAGE", "Visual damage", Kind::Toggle),
        s!(Assists, "Realism", "assists.ini", "ASSISTS", "FUEL_RATE", "Fuel rate", Kind::Number { min: 0., max: 5., step: 1., scale: 1., suffix: "×" }),
        s!(Assists, "Realism", "assists.ini", "ASSISTS", "TYRE_WEAR", "Tyre wear", Kind::Number { min: 0., max: 5., step: 1., scale: 1., suffix: "×" }),
        s!(Assists, "Realism", "assists.ini", "ASSISTS", "TYRE_BLANKETS", "Tyre blankets", Kind::Toggle),
        s!(Assists, "Realism", "assists.ini", "ASSISTS", "SLIPSTREAM", "Slipstream", Kind::Number { min: 0., max: 5., step: 1., scale: 1., suffix: "×" }),
        s!(Gameplay, "HUD", "gameplay.ini", "GUI", "SHOW_SECTORS", "Sector times", Kind::Toggle),
        s!(Gameplay, "HUD", "gameplay.ini", "TIME_DIFFERENCE", "IS_ACTIVE", "Time difference", Kind::Toggle),
        s!(Gameplay, "HUD", "gameplay.ini", "DRIVER_NAME_DISPLAYER", "IS_ACTIVE", "Driver names", Kind::Toggle),
        s!(Gameplay, "HUD", "gameplay.ini", "DAMAGE_DISPLAYER", "IS_ACTIVE", "Damage display", Kind::Toggle),
        s!(Gameplay, "HUD", "gameplay.ini", "VIRTUAL_MIRROR", "ACTIVE", "Virtual mirror", Kind::Toggle),
        s!(Gameplay, "HUD", "gameplay.ini", "PIT_INDICATOR", "ONLY_IN_PITLINE", "Pit marker only in pit lane", Kind::Toggle),
        s!(Gameplay, "Other", "gameplay.ini", "PYTHON", "ENABLE_PYTHON", "Python apps", Kind::Toggle),
        s!(Gameplay, "Other", "gameplay.ini", "PHYSICS", "ENABLE_CAR_COLLISIONS", "Car collisions", Kind::Toggle),
        s!(Controls, "Input", "controls.ini", "HEADER", "INPUT_METHOD", "Input method", Kind::Choice(INPUT)),
        s!(Controls, "Input", "controls.ini", "ADVANCED", "COMBINE_WITH_KEYBOARD_CONTROL", "Also use keyboard", Kind::Toggle),
        s!(Controls, "Keyboard", "controls.ini", "KEYBOARD", "STEERING_SPEED", "Steering speed", num(0.1, 5., 0.05)),
        s!(Controls, "Keyboard", "controls.ini", "KEYBOARD", "STEERING_OPPOSITE_DIRECTION_SPEED", "Counter-steer speed", num(0.1, 5., 0.05)),
        s!(Controls, "Keyboard", "controls.ini", "KEYBOARD", "STEER_RESET_SPEED", "Return speed", num(0.1, 5., 0.05)),
        s!(Controls, "Keyboard", "controls.ini", "KEYBOARD", "MOUSE_STEER", "Mouse steering", Kind::Toggle),
        s!(Controls, "Gamepad", "controls.ini", "X360", "STEER_SPEED", "Steering speed", num(0., 1., 0.05)),
        s!(Controls, "Gamepad", "controls.ini", "X360", "STEER_GAMMA", "Steering gamma", num(1., 5., 0.1)),
        s!(Controls, "Gamepad", "controls.ini", "X360", "STEER_DEADZONE", "Deadzone", pct(0.5, 0.01)),
        s!(Controls, "Gamepad", "controls.ini", "X360", "RUMBLE_INTENSITY", "Rumble", pct(1., 0.05)),
        s!(Controls, "Wheel", "controls.ini", "STEER", "LOCK", "Steering lock", Kind::Number { min: 180., max: 1080., step: 10., scale: 1., suffix: "°" }),
        s!(Controls, "Wheel", "controls.ini", "STEER", "FF_GAIN", "Force feedback", pct(2., 0.05)),
        s!(Controls, "Wheel", "controls.ini", "STEER", "FILTER_FF", "FF filter", pct(1., 0.05)),
        s!(Controls, "Wheel", "controls.ini", "FF_TWEAKS", "MIN_FF", "Minimum FF", pct(0.5, 0.01)),
        s!(Controls, "Wheel", "controls.ini", "FF_ENHANCEMENT", "CURBS", "Kerb effects", pct(1., 0.05)),
        s!(Controls, "Wheel", "controls.ini", "FF_ENHANCEMENT", "ROAD", "Road effects", pct(1., 0.05)),
        s!(Controls, "Wheel", "controls.ini", "FF_ENHANCEMENT", "SLIPS", "Slip effects", pct(1., 0.05)),
    ]
}

pub fn fmt_number(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}
pub fn documents_cfg_dir(ac_root: &Path) -> Option<PathBuf> {
    Some(paths::documents_ac_dir(ac_root)?.join("cfg"))
}

pub fn load(ac_root: &Path) -> Result<GameConfig> {
    let defaults = ac_root.join("cfg");
    let dir = documents_cfg_dir(ac_root).context("Documents/Assetto Corsa not found (run the game from Steam once)")?;
    let ppfilters = ac_root.join("system/cfg/ppfilters");
    GameConfig::load_from(&defaults, &dir, &ppfilters)
}

pub struct GameConfig {
    pub dir: PathBuf,
    pub files: BTreeMap<String, IniFile>,
    pub saved: BTreeMap<String, String>,
    pub filters: Vec<String>,
}

impl GameConfig {
    pub fn load_from(defaults: &Path, dir: &Path, ppfilters: &Path) -> Result<Self> {
        let mut files = BTreeMap::new();
        let mut saved = BTreeMap::new();
        for name in FILES {
            let mut ini = IniFile::load_or_default(&defaults.join(name))?;
            // overlay documents on top
            if let Ok(over) = IniFile::load(&dir.join(name)) {
                for s in over.sections() {
                    let dst = ini.section_mut(&s.name);
                    for (k, v) in s.entries() {
                        dst.set(k, v);
                    }
                }
            }
            saved.insert(name.to_string(), ini.to_string());
            files.insert(name.to_string(), ini);
        }
        let mut filters: Vec<String> = fs::read_dir(ppfilters)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension()?.to_str()? != "ini" {
                    return None;
                }
                Some(p.file_stem()?.to_string_lossy().into_owned())
            })
            .collect();
        filters.sort_by(|a, b| crate::content::natural_cmp(a, b));
        Ok(Self { dir: dir.to_path_buf(), files, saved, filters })
    }

    pub fn get(&self, file: &str, section: &str, key: &str) -> Option<&str> {
        self.files.get(file)?.get(section, key)
    }

    pub fn set(&mut self, file: &str, section: &str, key: &str, value: impl ToString) {
        self.files.entry(file.to_string()).or_default().section_mut(section).set(key, value);
    }

    pub fn number(&self, file: &str, section: &str, key: &str) -> f64 {
        self.get(file, section, key).and_then(|v| v.parse().ok()).unwrap_or(0.)
    }

    pub fn dirty_files(&self) -> Vec<&str> {
        self.files.iter().filter(|(n, f)| self.saved.get(*n) != Some(&f.to_string())).map(|(n, _)| n.as_str()).collect()
    }

    pub fn save(&mut self) -> Result<Vec<String>> {
        use std::io::Write;
        let dirty: Vec<String> = self.dirty_files().iter().map(|s| s.to_string()).collect();
        fs::create_dir_all(&self.dir).with_context(|| format!("creating {}", self.dir.display()))?;
        for name in &dirty {
            let text = self.files[name].to_string();
            let path = self.dir.join(name);
            let mut temp = tempfile::NamedTempFile::new_in(&self.dir)?;
            temp.write_all(text.replace('\n', "\r\n").as_bytes())?;
            temp.as_file().sync_all()?;
            temp.persist(&path).with_context(|| format!("writing {}", path.display()))?;
            self.saved.insert(name.to_string(), text);
        }
        Ok(dirty)
    }
}