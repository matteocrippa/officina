use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod app;
mod cli;
mod gamepad;
mod icons;
mod state;
mod updater;
mod views;

#[derive(Parser)]
#[command(
    name = "officina",
    version,
    about = "Officina — mod manager & launcher for Assetto Corsa (Linux)"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Print detected paths (AC root, Proton, compatdata, ...).
    Where,
    /// List installed content.
    List {
        /// 'cars', 'tracks', 'weather', or 'events'.
        kind: String,
    },
    /// Install a mod archive.
    Install { archive: PathBuf },
    /// Validate installed content (missing files, AI lines, liveries).
    Check,
    /// Pack/unpack data.acd files.
    Acd {
        #[command(subcommand)]
        action: AcdAction,
    },
}

#[derive(Subcommand)]
enum AcdAction {
    /// Decrypt data.acd into a folder (default: <car>/data).
    Unpack { file: PathBuf, out: Option<PathBuf> },
    /// Encrypt a data folder into data.acd (default: <car>/data.acd).
    Pack { dir: PathBuf, out: Option<PathBuf> },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Where) => cli::where_cmd(),
        Some(Commands::List { kind }) => cli::list_cmd(&kind),
        Some(Commands::Install { archive }) => cli::install_cmd(archive),
        Some(Commands::Check) => cli::check_cmd(),
        Some(Commands::Acd { action }) => match action {
            AcdAction::Unpack { file, out } => cli::acd_unpack_cmd(file, out),
            AcdAction::Pack { dir, out } => cli::acd_pack_cmd(dir, out),
        },
        None => run_gui(),
    }
}

fn run_gui() -> anyhow::Result<()> {
    let icon_bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.png"));
    let icon_image = image::load_from_memory(icon_bytes)
        .map_err(|e| anyhow::anyhow!("icon decode error: {e}"))?;
    let icon_rgba = icon_image.to_rgba8();
    let (width, height) = icon_rgba.dimensions();
    let icon_data = egui::IconData {
        rgba: icon_rgba.into_raw(),
        width,
        height,
    };

    let mut options = eframe::NativeOptions::default();
    options.viewport = options.viewport.with_icon(std::sync::Arc::new(icon_data));
    options.viewport = options.viewport.with_app_id("officina");
    eframe::run_native(
        "Officina",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::App::default()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("gui error: {e}"))
}
