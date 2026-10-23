use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod app;
mod cli;
mod state;
mod views;

#[derive(Parser)]
#[command(name = "acm", version, about = "Assetto Corsa mod manager & launcher (Linux)")]
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
        /// 'cars' or 'tracks'.
        kind: String,
    },
    /// Install a mod archive.
    Install {
        archive: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Where) => cli::where_cmd(),
        Some(Commands::List { kind }) => cli::list_cmd(&kind),
        Some(Commands::Install { archive }) => cli::install_cmd(archive),
        None => run_gui(),
    }
}

fn run_gui() -> anyhow::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "ACM - Assetto Corsa Manager",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::App::default()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("gui error: {e}"))
}