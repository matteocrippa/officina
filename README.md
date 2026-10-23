# ACM — Assetto Corsa Mod Manager & Launcher (Linux)

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
- **CLI** — `acm where`, `acm list cars|tracks`, `acm install <archive>`.
- **Steam Deck / Bazzite ready** — runs inside the Steam runtime, handles
  `steam-run` on NixOS, SDL gamepad routing, `PROTON_DISABLE_HIDRAW`.

## Screenshots

![ACM Cars view](docs/screens/main.png)

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
cargo build --release -p acm-app
./target/release/acm          # GUI
./target/release/acm where    # CLI
```

## CLI

```sh
acm where            # print detected paths
acm list cars         # list installed cars
acm list tracks       # list installed tracks
acm install mod.zip   # install a mod archive
```

## Configuration

Settings are stored in `~/.config/acm/settings.toml` (XDG). AC root and Proton
path are auto-detected but can be overridden in the Settings tab or directly in
the file.

## Project structure

- `crates/ac-core` — pure domain logic, file formats, Steam/Proton paths, content
  scanning, `race.ini`, mod installer. No UI dependencies.
- `crates/acm-app` — egui desktop app + CLI entrypoint.

## Provenance

File format logic and setup conventions follow Assetto Corsa's AcTools library
(MS-PL, © gro-ove). `data.acd` key derivation was reverse-engineered from
`AcTools.AcdEncryption` for file-format interoperability. No closed license
checks, proprietary keys or private API endpoints are included.

## License

MIT
