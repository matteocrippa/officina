use egui::Color32;

use crate::state::Nav;
use officina_core::content::Weather;

pub fn weather(w: &Weather) -> (&'static str, Color32) {
    let t = format!("{} {}", w.id, w.name).to_lowercase();
    if t.contains("thunder") || t.contains("storm") {
        ("⚡", Color32::from_rgb(180, 140, 255))
    } else if t.contains("snow") || t.contains("ice") || t.contains("blizzard") {
        ("❄", Color32::from_rgb(180, 220, 255))
    } else if t.contains("fog") || t.contains("mist") || t.contains("haze") {
        ("☁", Color32::GRAY)
    } else if t.contains("rain") || t.contains("drizzle") || t.contains("shower") {
        ("☔", Color32::from_rgb(120, 180, 255))
    } else if t.contains("overcast") || t.contains("heavy cloud") {
        ("☁", Color32::GRAY)
    } else if t.contains("cloud") || t.contains("scattered") {
        ("⛅", Color32::LIGHT_GRAY)
    } else if t.contains("wind") {
        ("💨", Color32::LIGHT_GRAY)
    } else if t.contains("clear") || t.contains("sunny") || t.contains("fair") {
        ("☀", Color32::from_rgb(255, 200, 60))
    } else {
        ("☁", Color32::LIGHT_GRAY)
    }
}

pub fn nav(n: Nav) -> &'static str {
    match n {
        Nav::Cars => "🚗",
        Nav::Tracks => "🏁",
        Nav::Weather => "☁",
        Nav::CheckMods => "🔍",
        Nav::Patches => "🔧",
        Nav::Career => "🏆",
        Nav::GameSettings => "⚙",
        Nav::QuickDrive => "⚡",
        Nav::Install => "📦",
        Nav::Settings => "⚙",
        Nav::About => "ℹ",
        Nav::Setups => "🔧",
        Nav::Results => "📊",
    }
}

pub const RESCAN: &str = "↻";
