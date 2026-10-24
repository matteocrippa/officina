//! Validation of installed AC content: missing files, layouts without an AI
//! line, and liveries that are likely to crash the game.

use std::path::{Path, PathBuf};

use crate::content::{scan_cars, scan_tracks};

#[derive(Debug, Clone)]
pub struct Issue {
    pub kind: &'static str,
    pub path: PathBuf,
    pub message: String,
}

pub fn check(ac_root: &Path) -> Vec<Issue> {
    let mut issues = Vec::new();

    // Cars
    for car in scan_cars(ac_root) {
        let ui_car = car.dir.join("ui").join("ui_car.json");
        if !ui_car.is_file() {
            issues.push(Issue {
                kind: "car",
                path: car.dir.clone(),
                message: format!("missing ui_car.json ({})", car.id),
            });
        }
        if car.skins.is_empty() {
            issues.push(Issue {
                kind: "car",
                path: car.dir.clone(),
                message: format!("no skins ({})", car.id),
            });
        }
        for skin in &car.skins {
            let ui_skin = skin.dir.join("ui_skin.json");
            if !ui_skin.is_file() {
                issues.push(Issue {
                    kind: "livery",
                    path: skin.dir.clone(),
                    message: format!("missing ui_skin.json ({}/{})", car.id, skin.id),
                });
            }
            // A livery that crashes AC often has a livery.png but no preview,
            // or a missing car model reference. Check for preview presence as a
            // weak signal.
            if skin.preview.is_none() {
                issues.push(Issue {
                    kind: "livery",
                    path: skin.dir.clone(),
                    message: format!("missing preview image ({}/{})", car.id, skin.id),
                });
            }
        }
    }

    // Tracks
    for track in scan_tracks(ac_root) {
        for layout in &track.layouts {
            let layout_dir = if layout.layout_id.is_some() {
                track.dir.join(layout.layout_id.as_deref().unwrap())
            } else {
                track.dir.clone()
            };
            // AI line check: look for ai folder or fast_lane.ai / spline
            let has_ai = layout_dir.join("ai").is_dir()
                || layout_dir.join("fast_lane.ai").is_file()
                || layout_dir.join("models.ini").is_file();
            if !has_ai {
                issues.push(Issue {
                    kind: "track",
                    path: layout_dir.clone(),
                    message: format!(
                        "layout without AI line: {}/{}",
                        track.id,
                        layout.layout_id.as_deref().unwrap_or("(default)")
                    ),
                });
            }
            // ui_track.json missing?
            let ui_track = if layout.layout_id.is_some() {
                layout_dir.join("ui").join("ui_track.json")
            } else {
                track.dir.join("ui").join("ui_track.json")
            };
            if !ui_track.is_file() {
                issues.push(Issue {
                    kind: "track",
                    path: layout_dir.clone(),
                    message: format!("missing ui_track.json ({})", track.id),
                });
            }
        }
    }

    // Missing content folders
    for sub in ["cars", "tracks"] {
        let d = ac_root.join("content").join(sub);
        if !d.is_dir() {
            issues.push(Issue {
                kind: "content",
                path: d.clone(),
                message: format!("missing content/{sub} folder"),
            });
        }
    }

    issues
}