# Officina — Mod Manager & Launcher for Assetto Corsa (Linux)

A native, lightweight desktop client for **Assetto Corsa** that runs on **Linux
(Bazzite, Steam Deck)**. It browses installed cars/tracks/skins, installs mods
from local archives, and launches the game through Proton with a full Quick
Drive setup.

Built in Rust with **egui/eframe** (no webview, Steam Deck-friendly). No
Electron, no .NET, no DirectX — just a small native binary.

## Features

- **Cars / Tracks browsers** — searchable, filterable lists with brand grouping,
  specs, skins, badges, track layouts, lengths, pit boxes.
- **Mod installer** — drag-and-drop or file-pick `.zip` / `.7z` / `.rar` /
  `.tar.gz` archives. Auto-detects car and track structures and extracts them
  into your Assetto Corsa directory (with overwrite confirmation).
- **Quick Drive** — pick car, skin, track and layout; choose Practice, Hotlap
  or Race (laps, opponents, AI level, starting grid); full conditions (time of
  day, ambient/road temperature, weather, time multiplier, wind, penalties,
  fixed setup, mph). Writes `race.ini` and launches `acs.exe` through Proton.
- **AC settings discovery** — auto-detects Steam libraries, AC install, Proton
  prefix and compatdata.
- **CLI** — `officina where`, `officina list cars|tracks`, `officina install <archive>`.
- **Steam Deck / Bazzite ready** — runs inside the Steam runtime, handles
  `steam-run` on NixOS, SDL gamepad routing, `PROTON_DISABLE_HIDRAW`.

## Screenshots

![Officina Cars view](docs/screens/main.png)

## Requirements

- Assetto Corsa installed via Steam (run it once from Steam so the Proton
  prefix is created).
- Proton enabled for Assetto Corsa (Proton-GE 9-20+ recommended).
- Rust 1.80+ to build.

Build dependencies (Debian/Ubuntu):

```sh
sudo apt install build-essential pkg-config \
  libxkbcommon-dev libwayland-dev libx11-dev libfontconfig-dev libvulkan-dev
```

## Build & run

```sh
cargo build --release -p officina-app
./target/release/officina          # GUI
./target/release/officina where    # CLI
```

## CLI

```sh
officina where            # print detected paths
officina list cars         # list installed cars
officina list tracks       # list installed tracks
officina install mod.zip   # install a mod archive
```

## Configuration

Settings are stored in `~/.config/officina/settings.toml` (XDG). AC root and Proton
path are auto-detected but can be overridden in the Settings tab or directly in
the file.

## Project structure

- `crates/officina-core` — pure domain logic, file formats, Steam/Proton paths, content
  scanning, `race.ini`, mod installer. No UI dependencies.
- `crates/officina-app` — egui desktop app + CLI entrypoint.

## Provenance

File format logic and setup conventions follow Assetto Corsa's AcTools library
(MS-PL, © gro-ove). `data.acd` key derivation was reverse-engineered from
`AcTools.AcdEncryption` for file-format interoperability. No closed license
checks, proprietary keys or private API endpoints are included.

## License

MIT
