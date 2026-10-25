use egui::Ui;

use crate::state::AppState;

pub fn show(_state: &mut AppState, ui: &mut Ui) {
    ui.heading("About Officina");
    ui.label(format!("Version: {}", crate::updater::current_version()));
    ui.label("Mod manager & launcher for Assetto Corsa on Linux (Bazzite, Steam Deck).");
    ui.label("Native Rust + egui. No Electron, no .NET.");
    ui.separator();
    ui.label("Links:");
    ui.hyperlink_to(
        "GitHub repository",
        "https://github.com/matteocrippa/officina",
    );
    ui.hyperlink_to(
        "Install script",
        "https://raw.githubusercontent.com/matteocrippa/officina/main/install.sh",
    );
    ui.separator();
    ui.label("AC formats follow AcTools (MS-PL, (c) gro-ove).");
    ui.label("License: MIT");
}
