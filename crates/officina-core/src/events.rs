use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

use crate::content::{natural_cmp, scan_cars};
use crate::ini::IniFile;
use crate::paths::{documents_ac_dir, race_ini_path};
use crate::race;

#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub series: Vec<Series>,
    pub special: Vec<Event>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Series {
    pub id: String,
    pub name: String,
    pub description: String,
    pub preview: Option<PathBuf>,
    pub requires: Vec<String>,
    pub requires_any: bool,
    pub points: Vec<u32>,
    pub goal_points: u32,
    pub goal_ranking: u32,
    pub goal_medals: [u32; 3],
    pub events: Vec<Event>,
    pub opponents: Vec<Opponent>,
}

#[derive(Debug, Clone)]
pub struct Opponent {
    pub id: String,
    pub name: String,
    pub car: String,
    pub skin: String,
    pub level: u32,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub name: String,
    pub description: String,
    pub preview: Option<PathBuf>,
    pub car: String,
    pub track: String,
    pub layout: String,
    pub weather: String,
    pub ambient: Option<f64>,
    pub road: Option<f64>,
    pub penalties: bool,
    pub fixed_setup: bool,
    pub ai_level: u32,
    pub sessions: Vec<Session>,
    pub goals: Vec<Goal>,
    pub ini: IniFile,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub kind: u32,
    pub laps: u32,
    pub minutes: u32,
}

#[derive(Debug, Clone)]
pub struct Goal {
    pub kind: String,
    pub objective: f64,
}

impl Series {
    pub fn championship(&self) -> bool {
        self.goal_points > 0 || self.goal_ranking > 0
    }

    pub fn unlocked(&self, p: &Progress) -> bool {
        if self.requires.is_empty() {
            return true;
        }
        if self.requires_any {
            self.requires
                .iter()
                .any(|req| p.completed_series.contains(req))
        } else {
            self.requires
                .iter()
                .all(|req| p.completed_series.contains(req))
        }
    }

    pub fn completed(&self, p: &Progress) -> bool {
        if p.completed_series.contains(&self.id) {
            return true;
        }
        if self.championship() {
            let pts = p
                .championship_points
                .get(&self.id)
                .and_then(|pts_map| pts_map.get("player"))
                .copied()
                .unwrap_or(0);
            if self.goal_points > 0 && pts >= self.goal_points {
                return true;
            }
            if self.goal_ranking > 0 {
                let all_done = !self.events.is_empty()
                    && self
                        .events
                        .iter()
                        .all(|e| p.get(&e.id).last_position.is_some());
                if all_done {
                    let st = self.standings(p);
                    if let Some(rank) = st.iter().position(|(id, _)| id == "player")
                        && (rank as u32 + 1) <= self.goal_ranking
                    {
                        return true;
                    }
                }
            }
            false
        } else {
            if self.events.is_empty() {
                return false;
            }
            let counts = self.medal_counts(p);
            let g = self.goal_medals;
            let mult = self.events.len() as i64;

            let mut diff3 = counts[2] as i64 - g[2] as i64;
            if diff3 < 0 {
                diff3 *= mult;
            }
            let mut diff2 = counts[1] as i64 + diff3 - g[1] as i64;
            if diff2 < 0 {
                diff2 *= mult;
            }
            let mut diff1 = counts[0] as i64 + diff2 - g[0] as i64;
            if diff1 < 0 {
                diff1 *= mult;
            }

            diff3 + diff2 + diff1 >= 0
        }
    }

    pub fn medal_counts(&self, p: &Progress) -> [u32; 3] {
        let mut b = 0;
        let mut s = 0;
        let mut g = 0;
        for ev in &self.events {
            match p.get(&ev.id).medal {
                1 => b += 1,
                2 => s += 1,
                3 => g += 1,
                _ => {}
            }
        }
        [b, s, g]
    }

    pub fn standings(&self, p: &Progress) -> Vec<(String, u32)> {
        let default_empty = BTreeMap::new();
        let series_pts = p
            .championship_points
            .get(&self.id)
            .unwrap_or(&default_empty);
        let mut list: Vec<(String, u32)> = Vec::new();
        list.push((
            "player".to_string(),
            series_pts.get("player").copied().unwrap_or(0),
        ));
        for opp in &self.opponents {
            let pts = series_pts.get(&opp.id).copied().unwrap_or(0);
            list.push((opp.id.clone(), pts));
        }
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| natural_cmp(&a.0, &b.0)));
        list
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventProgress {
    pub medal: u8,
    pub attempts: u32,
    pub best_score: Option<f64>,
    pub last_score: Option<f64>,
    pub last_position: Option<u32>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    pub events: BTreeMap<String, EventProgress>,
    pub championship_points: BTreeMap<String, BTreeMap<String, u32>>,
    pub completed_series: BTreeSet<String>,
}

impl Progress {
    pub fn load(root: &Path) -> Result<Self> {
        let mut p = Self::default();
        if let Some(doc_dir) = documents_ac_dir(root) {
            let json_path = doc_dir.join("launcherdata").join("cm-rs-progress.json");
            if json_path.exists() {
                let bytes = fs::read(&json_path)
                    .with_context(|| format!("reading {}", json_path.display()))?;
                return serde_json::from_slice(&bytes).map_err(|e| {
                    let detail = match crate::quarantine_corrupt(&json_path) {
                        Some(q) => format!(
                            "invalid progress in {} (moved to {}): {e}",
                            json_path.display(),
                            q.display()
                        ),
                        None => format!(
                            "invalid progress in {} (could not preserve the original): {e}",
                            json_path.display()
                        ),
                    };
                    anyhow!(detail)
                });
            }

            let career_ini_path = doc_dir
                .join("launcherdata")
                .join("filestore")
                .join("career.ini");
            if career_ini_path.is_file() {
                p.import_career_ini(&career_ini_path, root)?;
            }
        }
        Ok(p)
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        let doc_dir = documents_ac_dir(root)
            .ok_or_else(|| anyhow!("Cannot resolve Documents/Assetto Corsa directory"))?;
        let launcher_dir = doc_dir.join("launcherdata");
        fs::create_dir_all(&launcher_dir)
            .with_context(|| format!("creating directory {}", launcher_dir.display()))?;

        let target_path = launcher_dir.join("cm-rs-progress.json");
        let tmp_file = tempfile::NamedTempFile::new_in(&launcher_dir)
            .with_context(|| format!("creating temp file in {}", launcher_dir.display()))?;

        let data = serde_json::to_vec_pretty(self)?;
        fs::write(tmp_file.path(), data)
            .with_context(|| format!("writing temp file {}", tmp_file.path().display()))?;

        tmp_file.as_file().sync_all()?;
        tmp_file
            .persist(&target_path)
            .with_context(|| format!("persisting {}", target_path.display()))?;

        Ok(())
    }

    pub fn get(&self, id: &str) -> EventProgress {
        self.events.get(id).cloned().unwrap_or_default()
    }

    /// Updates this progress with the outcome of a finished event.
    pub fn record(
        &mut self,
        event: &Event,
        series: Option<&Series>,
        result: &serde_json::Value,
    ) -> Result<String> {
        let position = result_position(result);
        let total_time = result_number(result, &["totalTime", "TotalTime"]);
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut medal = match position {
            Some(1) => 3,
            Some(2..=3) => 2,
            Some(_) => 1,
            None => 0,
        };
        let mut points = series
            .filter(|s| s.championship())
            .and_then(|s| position.and_then(|p| s.points.get(p.saturating_sub(1) as usize)))
            .copied()
            .unwrap_or(0);

        if let Some(goal) = event.goals.first() {
            let kind = goal.kind.to_uppercase();
            if kind.contains("POSITION") && goal.objective > 0.0 {
                medal = match position {
                    Some(p) if p as f64 <= goal.objective => 3,
                    Some(p) if p as f64 <= goal.objective + 1.0 => 2,
                    Some(_) => 1,
                    None => 0,
                };
            } else if kind.contains("TIME") && goal.objective > 0.0 {
                medal = match total_time {
                    Some(t) if t <= goal.objective => 3,
                    Some(t) if t <= goal.objective * 1.05 => 2,
                    Some(_) => 1,
                    None => 0,
                };
            } else if kind.contains("POINTS") && goal.objective > 0.0 && points == 0 {
                points = goal.objective as u32;
            }
        }

        let score = points as f64 + medal as f64 * 10.0;
        let mut ep = self.get(&event.id);
        ep.attempts += 1;
        ep.updated_at = now;
        ep.last_position = position;
        ep.last_score = Some(score);
        ep.best_score = Some(ep.best_score.unwrap_or_default().max(score));
        ep.medal = ep.medal.max(medal);
        self.events.insert(event.id.clone(), ep);

        if let Some(ser) = series
            && ser.championship()
        {
            let table = self.championship_points.entry(ser.id.clone()).or_default();
            let total = table.entry("player".to_string()).or_default();
            *total = total.saturating_add(points);
        }
        if let Some(ser) = series
            && ser.completed(self)
        {
            self.completed_series.insert(ser.id.clone());
        }
        Ok(format!(
            "Result recorded: position {}, medal {}, points {}.",
            position.map_or("?".to_string(), |p| p.to_string()),
            medal,
            points
        ))
    }
}

fn result_number(result: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(v) = result.get(*key) {
            if let Some(n) = v.as_f64() {
                return Some(n);
            }
            if let Some(n) = v.as_str().and_then(|s| s.parse::<f64>().ok()) {
                return Some(n);
            }
        }
    }
    None
}

fn result_position(result: &serde_json::Value) -> Option<u32> {
    let drivers = result.get("result").and_then(|v| v.as_array())?;
    let player_name = result
        .get("playerName")
        .or_else(|| result.get("PlayerName"))
        .and_then(|v| v.as_str());
    let (index, entry) = drivers
        .iter()
        .enumerate()
        .find(|(_, d)| {
            d.get("isPlayer").and_then(|v| v.as_bool()).unwrap_or(false)
                || player_name
                    .is_some_and(|name| d.get("driverName").and_then(|v| v.as_str()) == Some(name))
        })
        .or_else(|| drivers.iter().enumerate().next())?;
    if let Some(p) = result_number(entry, &["position", "Position"]) {
        return Some(p as u32);
    }
    Some(index as u32 + 1)
}

/// Finds the newest matching result file and records it, then saves progress.
pub fn import_result(
    root: &Path,
    progress: &mut Progress,
    event: &Event,
    series: Option<&Series>,
    result_path: &Path,
) -> Result<String> {
    let text = fs::read_to_string(result_path)
        .with_context(|| format!("reading {}", result_path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("parsing {}", result_path.display()))?;
    let message = progress.record(event, series, &value)?;
    progress.save(root)?;
    Ok(message)
}

pub fn latest_result_for(root: &Path, event: &Event) -> Result<PathBuf> {
    for path in crate::results::list_results(root) {
        let Ok(result) = crate::results::parse_result(&path) else {
            continue;
        };
        if !event.car.is_empty() && !result.car.eq_ignore_ascii_case(&event.car) {
            continue;
        }
        if !event.track.is_empty()
            && !result
                .track
                .to_lowercase()
                .contains(&event.track.to_lowercase())
        {
            continue;
        }
        return Ok(path);
    }
    bail!("no result file found for {} / {}", event.car, event.track)
}

fn normalize_ini_uppercase(file: &mut IniFile) {
    let mut new_ini = IniFile::default();
    for sec in file.sections() {
        let sec_name = sec.name.to_uppercase();
        let target_sec = new_ini.section_mut(&sec_name);
        for (k, v) in sec.entries() {
            target_sec.set(&k.to_uppercase(), v);
        }
    }
    *file = new_ini;
}

impl Progress {
    fn import_career_ini(&mut self, path: &Path, root: &Path) -> Result<()> {
        let mut ini = IniFile::load(path)?;
        normalize_ini_uppercase(&mut ini);
        if let Some(comp) = ini.get("CAREER", "COMPLETE") {
            self.completed_series.extend(
                comp.split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty()),
            );
        }
        let catalog = scan(root);
        for sec in ini.sections() {
            let id = sec.name.to_lowercase();
            let Some(series) = catalog.series.iter().find(|s| s.id == id) else {
                continue;
            };
            for (key, value) in sec.entries() {
                if let Some(index) = key
                    .strip_prefix("EVENT")
                    .and_then(|i| i.parse::<usize>().ok())
                {
                    let Some(event) = series.events.get(index) else {
                        continue;
                    };
                    let Ok(val) = value.parse::<u32>() else {
                        continue;
                    };
                    let ep = self.events.entry(event.id.clone()).or_default();
                    if series.championship() {
                        if val > 0 {
                            ep.last_position = Some(val);
                        }
                    } else {
                        ep.medal = val.min(3) as u8;
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn scan(root: &Path) -> Catalog {
    let mut cat = Catalog::default();
    let career_dir = root.join("content").join("career");
    if career_dir.is_dir()
        && let Ok(entries) = fs::read_dir(&career_dir)
    {
        let mut series_dirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|de| de.path()))
            .filter(|p| {
                p.is_dir()
                    && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                        n.starts_with("series") && n != "series0" && n != "series100"
                    })
            })
            .collect();
        series_dirs.sort_by(|a, b| {
            let na = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let nb = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
            natural_cmp(na, nb)
        });

        for sdir in series_dirs {
            match load_series(&sdir) {
                Ok(s) => cat.series.push(s),
                Err(e) => cat
                    .errors
                    .push(format!("series {}: {:#}", sdir.display(), e)),
            }
        }
    }

    let special_dir = root.join("content").join("specialevents");
    if special_dir.is_dir()
        && let Ok(entries) = fs::read_dir(&special_dir)
    {
        let mut event_dirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|de| de.path()))
            .filter(|p| {
                p.is_dir()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("SPECIAL_EVENT_"))
            })
            .collect();
        event_dirs.sort_by(|a, b| {
            let na = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let nb = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let num_a = na
                .trim_start_matches("SPECIAL_EVENT_")
                .parse::<u32>()
                .unwrap_or(u32::MAX);
            let num_b = nb
                .trim_start_matches("SPECIAL_EVENT_")
                .parse::<u32>()
                .unwrap_or(u32::MAX);
            num_a.cmp(&num_b).then_with(|| natural_cmp(na, nb))
        });

        for edir in event_dirs {
            let ev_id = edir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let ini_path = edir.join("event.ini");
            if ini_path.is_file() {
                match load_event(&ini_path, &ev_id, &edir) {
                    Ok(ev) => cat.special.push(ev),
                    Err(e) => cat
                        .errors
                        .push(format!("special event {}: {:#}", edir.display(), e)),
                }
            }
        }
    }

    cat
}

fn load_series(dir: &Path) -> Result<Series> {
    let id = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let ini_path = dir.join("series.ini");
    let ini =
        IniFile::load(&ini_path).with_context(|| format!("loading {}", ini_path.display()))?;

    let name = ini.get("SERIES", "NAME").unwrap_or(&id).to_string();
    let description = crate::content::plain_text(ini.get("SERIES", "DESCRIPTION").unwrap_or(""));
    let preview = {
        let p = dir.join("preview.png");
        if p.is_file() { Some(p) } else { None }
    };

    let requires: Vec<String> = ini
        .get("SERIES", "REQUIRES")
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let requires_any = ini
        .get("SERIES", "REQUIRESANY")
        .is_some_and(|v| v.trim() == "1");

    let points: Vec<u32> = ini
        .get("SERIES", "POINTS")
        .unwrap_or("")
        .split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .collect();

    let goal_points = ini
        .get("GOALS", "POINTS")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let goal_ranking = ini
        .get("GOALS", "RANKING")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(0);

    let t1 = ini
        .get("GOALS", "TIER1")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let t2 = ini
        .get("GOALS", "TIER2")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let t3 = ini
        .get("GOALS", "TIER3")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let goal_medals = [t1, t2, t3];

    let mut events = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        let mut ev_dirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|de| de.path()))
            .filter(|p| {
                p.is_dir()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("event"))
            })
            .collect();
        ev_dirs.sort_by(|a, b| {
            let na = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let nb = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
            natural_cmp(na, nb)
        });

        for ed in ev_dirs {
            let ev_num = ed.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let event_id = format!("{}/{}", id, ev_num);
            let ev_ini = ed.join("event.ini");
            if ev_ini.is_file() {
                events.push(load_event(&ev_ini, &event_id, &ed)?);
            }
        }
    }

    let mut opponents = Vec::new();
    let opp_path = dir.join("opponents.ini");
    if opp_path.is_file()
        && let Ok(opp_ini) = IniFile::load(&opp_path)
    {
        for sec in opp_ini.sections() {
            let name = sec.name.as_str();
            if name.starts_with("AI") {
                let opp_id = name.to_lowercase();
                let opp_name = sec
                    .get("DRIVER_NAME")
                    .or_else(|| sec.get("NAME"))
                    .unwrap_or("")
                    .to_string();
                let opp_car = sec
                    .get("MODEL")
                    .or_else(|| sec.get("CAR"))
                    .unwrap_or("")
                    .to_string();
                let opp_skin = sec.get("SKIN").unwrap_or("").to_string();
                let opp_level = sec
                    .get("AI_LEVEL")
                    .or_else(|| sec.get("LEVEL"))
                    .and_then(|v| v.trim().parse::<u32>().ok())
                    .unwrap_or(98);
                opponents.push(Opponent {
                    id: opp_id,
                    name: opp_name,
                    car: opp_car,
                    skin: opp_skin,
                    level: opp_level,
                });
            }
        }
    }

    opponents.sort_by(|a, b| natural_cmp(&a.id, &b.id));
    Ok(Series {
        id,
        name,
        description,
        preview,
        requires,
        requires_any,
        points,
        goal_points,
        goal_ranking,
        goal_medals,
        events,
        opponents,
    })
}

fn load_event(path: &Path, id: &str, event_dir: &Path) -> Result<Event> {
    let mut ini = IniFile::load(path)?;
    normalize_ini_uppercase(&mut ini);

    let name = ini
        .get("EVENT", "NAME")
        .unwrap_or_else(|| ini.get("SESSION_0", "NAME").unwrap_or(id))
        .to_string();
    let description = crate::content::plain_text(ini.get("EVENT", "DESCRIPTION").unwrap_or(""));
    let preview = {
        let p = event_dir.join("preview.png");
        if p.is_file() { Some(p) } else { None }
    };

    let car = ini
        .get("RACE", "MODEL")
        .or_else(|| ini.get("CAR_0", "MODEL"))
        .filter(|m| *m != "-")
        .unwrap_or("")
        .to_string();
    let track = ini.get("RACE", "TRACK").unwrap_or("").to_string();
    let layout = ini.get("RACE", "CONFIG_TRACK").unwrap_or("").to_string();
    let weather = ini.get("WEATHER", "NAME").unwrap_or("3_clear").to_string();
    let ambient = ini
        .get("TEMPERATURE", "AMBIENT")
        .and_then(|v| v.trim().parse::<f64>().ok());
    let road = ini
        .get("TEMPERATURE", "ROAD")
        .and_then(|v| v.trim().parse::<f64>().ok());
    let penalties = ini.get("RACE", "PENALTIES").is_none_or(|v| v.trim() != "0");
    let fixed_setup = ini
        .get("RACE", "FIXED_SETUP")
        .is_some_and(|v| v.trim() == "1");
    let ai_level = ini
        .get("RACE", "AI_LEVEL")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(98);

    let mut sessions = Vec::new();
    for i in 0..10 {
        let sname = format!("SESSION_{}", i);
        if let Some(sec) = ini.section(&sname) {
            let name = sec.get("NAME").unwrap_or("Session").to_string();
            let kind = sec
                .get("TYPE")
                .and_then(|v| v.trim().parse::<u32>().ok())
                .unwrap_or(1);
            let laps = sec
                .get("LAPS")
                .and_then(|v| v.trim().parse::<u32>().ok())
                .unwrap_or(0);
            let minutes = sec
                .get("DURATION_MINUTES")
                .and_then(|v| v.trim().parse::<u32>().ok())
                .unwrap_or(0);
            sessions.push(Session {
                name,
                kind,
                laps,
                minutes,
            });
        }
    }

    let cond0_type = ini
        .get("CONDITION_0", "TYPE")
        .map(|t| t.trim().to_uppercase());

    let mut goals = Vec::new();
    for i in 0..10 {
        let cname = format!("CONDITION_{}", i);
        if let Some(sec) = ini.section(&cname) {
            let mut k = sec
                .get("TYPE")
                .map(|t| t.trim().to_uppercase())
                .or_else(|| cond0_type.clone())
                .unwrap_or_else(|| "POINTS".to_string());
            if k.is_empty() {
                k = cond0_type.clone().unwrap_or_else(|| "POINTS".to_string());
            }
            let obj = sec
                .get("OBJECTIVE")
                .and_then(|v| v.trim().parse::<f64>().ok())
                .unwrap_or(0.0);
            goals.push(Goal {
                kind: k,
                objective: obj,
            });
        }
    }

    Ok(Event {
        id: id.to_string(),
        name,
        description,
        preview,
        car,
        track,
        layout,
        weather,
        ambient,
        road,
        penalties,
        fixed_setup,
        ai_level,
        sessions,
        goals,
        ini,
    })
}

pub fn available(root: &Path, event: &Event, series: Option<&Series>) -> Result<()> {
    let cars = root.join("content/cars");
    anyhow::ensure!(
        !event.car.is_empty() && cars.join(&event.car).is_dir(),
        "Car '{}' is not installed",
        event.car
    );
    let track_dir = root.join("content/tracks").join(&event.track);
    anyhow::ensure!(
        track_dir.is_dir(),
        "Track '{}' is not installed",
        event.track
    );
    let track = crate::content::load_track(&track_dir);
    anyhow::ensure!(
        track
            .layouts
            .iter()
            .any(|l| l.layout_id.as_deref().unwrap_or("") == event.layout),
        "Track layout '{}' is not installed",
        event.layout
    );
    anyhow::ensure!(
        root.join("content/weather").join(&event.weather).is_dir(),
        "Weather '{}' is not installed",
        event.weather
    );
    if let Some(series) = series.filter(|s| s.championship()) {
        for opponent in &series.opponents {
            anyhow::ensure!(
                !opponent.car.is_empty() && cars.join(&opponent.car).is_dir(),
                "Opponent car '{}' is not installed",
                opponent.car
            );
        }
    } else {
        for section in event
            .ini
            .sections()
            .iter()
            .filter(|s| s.name.starts_with("CAR_") && s.name != "CAR_0")
        {
            if let Some(model) = section.get("MODEL").filter(|m| *m != "-" && !m.is_empty()) {
                anyhow::ensure!(
                    cars.join(model).is_dir(),
                    "Opponent car '{}' is not installed",
                    model
                );
            }
        }
    }
    Ok(())
}

pub fn prepare(
    root: &Path,
    event: &Event,
    series: Option<&Series>,
    difficulty: usize,
    driver: &str,
    nationality: &str,
) -> Result<PathBuf> {
    available(root, event, series)?;

    let target_path =
        race_ini_path(root).ok_or_else(|| anyhow!("Cannot locate AC race.ini path"))?;

    let mut ini = IniFile::load_or_default(&root.join("cfg/race.ini"))?;
    if ini.sections().is_empty() {
        ini = IniFile::load_or_default(&target_path)?;
    }
    ini.remove_section("CAR_0");
    race::clear_up(&mut ini);

    let cond0_type = event
        .ini
        .get("CONDITION_0", "TYPE")
        .map(|t| t.trim().to_uppercase());

    for sec in event.ini.sections() {
        let sec_name = sec.name.to_uppercase();
        if sec_name == "EVENT" || sec_name == "ASSISTS" {
            continue;
        }

        let target_sec = ini.section_mut(&sec_name);
        for (k, v) in sec.entries() {
            let key = k.to_uppercase();
            if sec_name.starts_with("CONDITION_")
                && key == "TYPE"
                && v.trim().is_empty()
                && let Some(c0) = &cond0_type
            {
                target_sec.set("TYPE", c0);
                continue;
            }
            target_sec.set(&key, v);
        }

        if sec_name.starts_with("CONDITION_")
            && !sec.entries().any(|(k, _)| k.eq_ignore_ascii_case("TYPE"))
            && let Some(c0) = &cond0_type
        {
            target_sec.set("TYPE", c0);
        }
    }

    let is_special_ai = event
        .goals
        .first()
        .is_some_and(|g| g.kind.to_uppercase() == "AI");
    if is_special_ai {
        let diff_idx = difficulty.clamp(0, 2);
        let goal_obj = event
            .goals
            .get(diff_idx)
            .map(|g| g.objective)
            .or_else(|| event.goals.last().map(|g| g.objective))
            .unwrap_or(98.0) as u32;

        ini.section_mut("RACE").set("AI_LEVEL", goal_obj);
        let names: Vec<_> = ini.sections().iter().map(|s| s.name.clone()).collect();
        for sname in names {
            if sname.starts_with("CAR_") && sname != "CAR_0" {
                ini.section_mut(&sname).set("AI_LEVEL", goal_obj);
            }
        }
    }

    if let Some(ser) = series
        && ser.championship()
    {
        ini.remove_numbered("CAR", 1);
        let mut index = 1;
        for opp in &ser.opponents {
            let car_sec = format!("CAR_{}", index);
            let c = ini.section_mut(&car_sec);
            c.set("DRIVER_NAME", &opp.name);
            c.set("NATIONALITY", "");
            c.set("NATION_CODE", "");
            c.set("AI_LEVEL", opp.level);
            c.set("MODEL", opp.car.to_lowercase());
            c.set("MODEL_CONFIG", "");
            c.set("SKIN", opp.skin.to_lowercase());
            c.set("SETUP", "");
            index += 1;
        }
        ini.section_mut("RACE").set("CARS", index);
    }

    let race = ini.section_mut("RACE");
    if !event.car.is_empty() {
        race.set("MODEL", event.car.to_lowercase());
    }
    race.set("TRACK", event.track.to_lowercase());
    race.set("CONFIG_TRACK", event.layout.to_lowercase());
    race.set_bool("PENALTIES", event.penalties);
    race.set_bool("FIXED_SETUP", event.fixed_setup);

    if event.ini.get("WEATHER", "NAME").is_none() || event.ini.get("WEATHER", "NAME") == Some("") {
        let weather = if !event.weather.is_empty() {
            &event.weather
        } else {
            "3_clear"
        };
        ini.section_mut("WEATHER").set("NAME", weather);
    }

    let cars = scan_cars(root);
    let car_model = event.car.to_lowercase();
    let car_info = cars.iter().find(|c| c.id.eq_ignore_ascii_case(&car_model));

    let car0 = ini.section_mut("CAR_0");
    car0.set("MODEL", "-");
    car0.set("DRIVER_NAME", driver);
    car0.set("NATIONALITY", nationality);
    car0.set("NATION_CODE", nationality);
    car0.set("SETUP", "");

    let existing_skin = car0.get("SKIN").unwrap_or("").to_string();
    let skin_is_valid = if let Some(ci) = car_info {
        ci.skins
            .iter()
            .any(|s| s.id.eq_ignore_ascii_case(&existing_skin))
    } else {
        !existing_skin.is_empty()
    };

    if !skin_is_valid {
        let fallback_skin = car_info
            .and_then(|ci| ci.skins.first())
            .map(|s| s.id.clone())
            .unwrap_or_default();
        car0.set("SKIN", fallback_skin);
    }

    ini.save(&target_path)?;
    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn event(goals: Vec<Goal>) -> Event {
        Event {
            id: "ev1".into(),
            name: "Test event".into(),
            description: String::new(),
            preview: None,
            car: "car".into(),
            track: "track".into(),
            layout: String::new(),
            weather: String::new(),
            ambient: None,
            road: None,
            penalties: false,
            fixed_setup: false,
            ai_level: 90,
            sessions: Vec::new(),
            goals,
            ini: IniFile::default(),
        }
    }

    fn championship(event: Event) -> Series {
        Series {
            id: "s1".into(),
            name: "Test series".into(),
            description: String::new(),
            preview: None,
            requires: Vec::new(),
            requires_any: false,
            points: vec![10, 6, 3],
            goal_points: 10,
            goal_ranking: 0,
            goal_medals: [0, 0, 0],
            events: vec![event],
            opponents: Vec::new(),
        }
    }

    #[test]
    fn record_updates_medal_and_attempts() {
        let mut progress = Progress::default();
        let ev = event(vec![Goal {
            kind: "POSITION".into(),
            objective: 1.0,
        }]);
        let result = json!({
            "playerName": "Player",
            "totalTime": 100.0,
            "result": [
                {"driverName": "Other", "position": 1},
                {"driverName": "Player", "position": 2, "isPlayer": true}
            ]
        });
        let message = progress.record(&ev, None, &result).unwrap();
        assert!(message.contains("position 2"));
        let p = progress.get("ev1");
        assert_eq!(p.attempts, 1);
        assert_eq!(p.medal, 2);
        assert_eq!(p.last_position, Some(2));
    }

    #[test]
    fn record_awards_championship_points_and_completes_series() {
        let mut progress = Progress::default();
        let ev = event(Vec::new());
        let series = championship(ev.clone());
        let result = json!({
            "playerName": "Player",
            "result": [{"driverName": "Player", "position": 1, "isPlayer": true}]
        });
        progress.record(&ev, Some(&series), &result).unwrap();
        assert_eq!(
            progress.championship_points["s1"]["player"], 10,
            "first place should award the configured points"
        );
        assert!(series.completed(&progress));
    }
}
