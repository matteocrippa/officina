use std::path::PathBuf;

use egui::Ui;

use crate::icons;
use crate::state::AppState;

pub fn show(state: &mut AppState, ui: &mut Ui, ctx: &egui::Context) {
    ui.heading(format!(
        "{} Install",
        icons::nav(crate::state::Nav::Install)
    ));
    ui.label("Drop a .zip/.7z/.rar archive here, or pick a file.");

    // Handle drag-and-drop.
    let dropped: Vec<PathBuf> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .filter_map(|f| f.path.clone())
            .collect()
    });
    if !dropped.is_empty() && state.install_archive.is_none() && !state.install_busy {
        state.plan_install(dropped[0].clone());
    }

    ui.horizontal(|ui| {
        if ui.button("Pick file...").clicked()
            && !state.install_busy
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Archives", &["zip", "7z", "rar", "tar", "gz", "tgz"])
                .pick_file()
        {
            state.plan_install(path);
        }
        if state.install_archive.is_some()
            && ui.button("Install now").clicked()
            && !state.install_busy
        {
            state.run_install();
        }
        if state.install_archive.is_some() && ui.button("Clear").clicked() && !state.install_busy {
            state.install_archive = None;
            state.install_plan = None;
            state.install_status.clear();
        }
    });

    if state.install_busy {
        ui.spinner();
    }
    if !state.install_status.is_empty() {
        ui.label(&state.install_status);
    }

    if let Some(archive) = &state.install_archive {
        ui.label(format!("Archive: {}", archive.display()));
    }
    if let Some(plan) = &state.install_plan {
        ui.label(format!("Planned {} item(s):", plan.len()));
        for it in plan {
            ui.label(format!(
                "  {} {}  (exists: {})",
                it.kind.label(),
                it.id,
                it.exists
            ));
            ui.small(format!("    -> {}", it.target.display()));
        }
    }

    ui.separator();
    ui.group(|ui| {
        ui.label("GitHub tools");
        ui.small("Install community tools (e.g. Paintshop, Helicorsa) from GitHub releases or a direct archive URL.");
        ui.text_edit_singleline(&mut state.tool_repo);
        if ui.button("Install latest release").clicked() && !state.install_busy {
            state.install_tool(state.tool_repo.clone());
        }
    });

    let _ = ctx;
}
