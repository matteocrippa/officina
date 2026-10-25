//! Lenient JSON helpers for AC's ui_*.json files.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

pub fn parse_file(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    // Strip BOM if present.
    let text = text.trim_start_matches('\u{feff}');
    serde_json::from_str(text).with_context(|| format!("parsing {}", path.display()))
}

pub fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| match x {
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() || t == "--" {
                None
            } else {
                Some(t.to_owned())
            }
        }
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}
