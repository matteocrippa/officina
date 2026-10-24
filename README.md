# 🏎️ Officina

Mod manager & launcher for **Assetto Corsa** on Linux (Bazzite, Steam Deck).

Native Rust + egui. No Electron, no .NET — just a small binary.

![Officina Cars view](docs/screens/main.png)

## ✨ Features

- 🚗 **Cars / 🏁 Tracks** — browse, filter, view specs, skins, badges
- 📦 **Mod installer** — drag & drop `.zip`/`.7z`/`.rar`/`.tar.gz` (cars, tracks, CSP, Sol, Pure)
- ⚡ **Quick Drive** — car/skin/track, sessions, laps, opponents, weather/time/temp
- 🏆 **Career & Special Events** — series progression, medals, launch
- ⚙️ **Game Settings** — video/audio/assists/gameplay/controls INIs
- 🔧 **Patches** — CSP/Sol/Pure detection & install
- 🔍 **Check Mods** — missing files, AI lines, liveries
- 📂 **acd pack/unpack** — decrypt/encrypt `data.acd`
- 🎮 **Gamepad** navigation (gilrs)
- 🔄 **Self-update** from GitHub Releases
- 🗑️ **Uninstall to trash**, enable/disable mods (JSGME-style)
- 💾 **Settings backups** with restore
- 🖥️ **CLI** — `officina where|list|install|check|acd`

## 🚀 Install

```sh
curl -fsSL https://raw.githubusercontent.com/matteocrippa/officina/main/install.sh | sh
```

Or build from source:

```sh
cargo build --release -p officina-app
./target/release/officina
```

Deps (Debian/Ubuntu): `build-essential pkg-config libxkbcommon-dev libwayland-dev libx11-dev libfontconfig-dev libvulkan-dev`

## ⌨️ CLI

```sh
officina where
officina list cars|tracks|weather|events
officina install mod.zip
officina check
officina acd unpack data.acd [out_dir]
officina acd pack data_dir [data.acd]
```

## ⚙️ Config

`~/.config/officina/settings.toml` — AC root & Proton auto-detected, overridable.

## 📁 Structure

- `crates/officina-core` — domain logic (no UI)
- `crates/officina-app` — egui app + CLI

## 📜 License

MIT — see [LICENSE.md](LICENSE.md). AC formats follow AcTools (MS-PL, © gro-ove).