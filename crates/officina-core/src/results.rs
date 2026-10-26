//! Race result files written by Assetto Corsa under Documents/Assetto Corsa/results.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::paths;

#[derive(Debug, Clone, Default)]
pub struct Lap {
    pub time: f64,
    pub valid: bool,
    pub sectors: Vec<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct RaceResult {
    pub file: PathBuf,
    pub track: String,
    pub car: String,
    pub player: String,
    pub session: String,
    pub best_lap: Option<f64>,
    pub total_time: Option<f64>,
    pub laps: Vec<Lap>,
}

pub fn results_dir(ac_root: &Path) -> Option<PathBuf> {
    let doc = paths::documents_ac_dir(ac_root)?;
    let dir = doc.join("results");
    if dir.is_dir() { Some(dir) } else { None }
}

pub fn list_results(ac_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = results_dir(ac_root)
        && let Ok(entries) = fs::read_dir(&dir)
    {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file()
                && p.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| x.eq_ignore_ascii_case("json"))
            {
                out.push(p);
            }
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse(fs::metadata(p).and_then(|m| m.modified()).ok()));
    out
}

fn as_f64(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_i64().map(|n| n as f64))
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

pub fn parse_result(path: &Path) -> Result<RaceResult> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let v: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;

    let mut r = RaceResult {
        file: path.to_path_buf(),
        ..Default::default()
    };

    r.track = v
        .get("track")
        .or_else(|| v.get("Track"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    r.car = v
        .get("car")
        .or_else(|| v.get("Car"))
        .or_else(|| v.get("model"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    r.player = v
        .get("playerName")
        .or_else(|| v.get("PlayerName"))
        .or_else(|| v.get("driverName"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    r.session = v
        .get("sessionType")
        .or_else(|| v.get("SessionType"))
        .or_else(|| v.get("session"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    r.best_lap = v
        .get("bestLap")
        .or_else(|| v.get("BestLap"))
        .or_else(|| v.get("best_lap"))
        .and_then(as_f64);
    r.total_time = v
        .get("totalTime")
        .or_else(|| v.get("TotalTime"))
        .and_then(as_f64);

    // laps
    let laps_val = v
        .get("laps")
        .or_else(|| v.get("Laps"))
        .and_then(|x| x.as_array());
    if let Some(arr) = laps_val {
        for l in arr {
            let time = l
                .get("lapTime")
                .or_else(|| l.get("LapTime"))
                .or_else(|| l.get("time"))
                .and_then(as_f64)
                .unwrap_or(0.0);
            let valid = l
                .get("valid")
                .or_else(|| l.get("Valid"))
                .and_then(|x| x.as_bool())
                .unwrap_or(true);
            let sectors = l
                .get("sectors")
                .or_else(|| l.get("Sectors"))
                .and_then(|x| x.as_array())
                .map(|secs| secs.iter().filter_map(as_f64).collect())
                .unwrap_or_default();
            r.laps.push(Lap {
                time,
                valid,
                sectors,
            });
        }
    }

    // Fallback best lap from laps if missing.
    if r.best_lap.is_none() {
        r.best_lap = r
            .laps
            .iter()
            .filter(|l| l.valid && l.time > 0.0)
            .map(|l| l.time)
            .reduce(f64::min);
    }

    Ok(r)
}

pub fn format_time(seconds: f64) -> String {
    if seconds <= 0.0 {
        return "--:--.---".to_string();
    }
    let mins = (seconds / 60.0).floor() as u64;
    let secs = seconds - mins as f64 * 60.0;
    format!("{}:{:06.3}", mins, secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_basic() {
        assert_eq!(format_time(90.5), "1:30.500");
        assert_eq!(format_time(0.0), "--:--.---");
    }

    #[test]
    fn parse_result_lenient() {
        let dir = std::env::temp_dir().join("officina_res_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("r.json");
        let json = r#"{"track":"ks_monza","car":"ferrari","playerName":"P","bestLap":91.2,"laps":[{"lapTime":92.0,"valid":true},{"lapTime":91.2,"valid":true,"sectors":[30.0,31.0,30.2]}]}"#;
        std::fs::write(&path, json).unwrap();
        let r = parse_result(&path).unwrap();
        assert_eq!(r.track, "ks_monza");
        assert_eq!(r.car, "ferrari");
        assert_eq!(r.laps.len(), 2);
        assert_eq!(r.best_lap, Some(91.2));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
