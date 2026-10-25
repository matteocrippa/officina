//! Order-preserving INI files in the dialect Assetto Corsa uses.
//!
//! Section and key names are matched case-sensitively (AC writes them in
//! upper case). Comments after `;` or `//` are dropped on read.

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result};

#[derive(Debug, Clone, Default)]
pub struct Section {
    pub name: String,
    entries: Vec<(String, String)>,
}

impl Section {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entries: Vec::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn set(&mut self, key: &str, value: impl ToString) -> &mut Self {
        let value = value.to_string();
        match self.entries.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = value,
            None => self.entries.push((key.to_owned(), value)),
        }
        self
    }

    pub fn set_bool(&mut self, key: &str, value: bool) -> &mut Self {
        self.set(key, if value { 1 } else { 0 })
    }

    pub fn remove(&mut self, key: &str) {
        self.entries.retain(|(k, _)| k != key);
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

#[derive(Debug, Clone, Default)]
pub struct IniFile {
    sections: Vec<Section>,
}

fn strip_comment(line: &str) -> &str {
    let semi = line.find(';');
    let slashes = line.find("//");
    match (semi, slashes) {
        (Some(a), Some(b)) => &line[..a.min(b)],
        (Some(a), None) | (None, Some(a)) => &line[..a],
        (None, None) => line,
    }
}

impl IniFile {
    pub fn parse(text: &str) -> Self {
        let mut file = IniFile::default();
        let mut current: Option<usize> = None;
        for raw in text.trim_start_matches('\u{feff}').lines() {
            let line = raw.trim();
            if line.starts_with('[')
                && let Some(end) = line.find(']')
            {
                let name = line[1..end].trim().to_owned();
                current = Some(file.section_index(&name));
                continue;
            }
            let line = strip_comment(line).trim();
            if line.is_empty() {
                continue;
            }
            if let (Some(idx), Some(eq)) = (current, line.find('=')) {
                let key = line[..eq].trim();
                let value = line[eq + 1..].trim();
                file.sections[idx].set(key, value);
            }
        }
        file
    }

    pub fn load(path: &std::path::Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Ok(Self::parse(&String::from_utf8_lossy(&bytes)))
    }

    pub fn load_or_default(path: &std::path::Path) -> Result<Self> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::default())
        }
    }

    fn section_index(&mut self, name: &str) -> usize {
        match self.sections.iter().position(|s| s.name == name) {
            Some(i) => i,
            None => {
                self.sections.push(Section::new(name));
                self.sections.len() - 1
            }
        }
    }

    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }

    pub fn section_mut(&mut self, name: &str) -> &mut Section {
        let i = self.section_index(name);
        &mut self.sections[i]
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.section(section)?.get(key)
    }

    pub fn remove_section(&mut self, name: &str) {
        self.sections.retain(|s| s.name != name);
    }

    pub fn remove_numbered(&mut self, prefix: &str, from: u32) {
        self.sections.retain(|s| {
            s.name
                .strip_prefix(prefix)
                .and_then(|r| r.strip_prefix('_'))
                .and_then(|n| n.parse::<u32>().ok())
                .is_none_or(|n| n < from)
        });
    }

    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_string()).with_context(|| format!("writing {}", path.display()))
    }
}

impl std::fmt::Display for IniFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        for (i, s) in self.sections.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            let _ = writeln!(out, "[{}]", s.name);
            for (k, v) in &s.entries {
                let _ = writeln!(out, "{k}={v}");
            }
        }
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let text = "[SECTION]\nKEY=value\nFOO=bar\n";
        let ini = IniFile::parse(text);
        assert_eq!(ini.get("SECTION", "KEY"), Some("value"));
        assert_eq!(ini.get("SECTION", "FOO"), Some("bar"));
        assert_eq!(ini.get("MISSING", "KEY"), None);
    }

    #[test]
    fn parse_strips_comments() {
        let text = "[S]\nKEY=value ; comment\nOTHER=2 // note\n";
        let ini = IniFile::parse(text);
        assert_eq!(ini.get("S", "KEY"), Some("value"));
        assert_eq!(ini.get("S", "OTHER"), Some("2"));
    }

    #[test]
    fn set_updates_or_appends() {
        let mut ini = IniFile::default();
        ini.section_mut("S").set("K", "1");
        ini.section_mut("S").set("K", "2");
        assert_eq!(ini.get("S", "K"), Some("2"));
        let entries: Vec<_> = ini.section("S").unwrap().entries().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn roundtrip_to_string() {
        let text = "[A]\nX=1\n\n[B]\nY=2\n";
        let ini = IniFile::parse(text);
        let out = ini.to_string();
        let ini2 = IniFile::parse(&out);
        assert_eq!(ini2.get("A", "X"), Some("1"));
        assert_eq!(ini2.get("B", "Y"), Some("2"));
    }

    #[test]
    fn remove_numbered() {
        let mut ini = IniFile::default();
        ini.section_mut("CAR_0");
        ini.section_mut("CAR_1");
        ini.section_mut("CAR_2");
        ini.remove_numbered("CAR", 1);
        assert!(ini.section("CAR_0").is_some());
        assert!(ini.section("CAR_1").is_none());
        assert!(ini.section("CAR_2").is_none());
    }
}
