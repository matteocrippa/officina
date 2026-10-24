#!/bin/sh
set -e
PREFIX="${PREFIX:-$HOME/.local}"
install -Dm755 target/release/officina "$PREFIX/bin/officina"
install -Dm644 packaging/officina.desktop "$PREFIX/share/applications/officina.desktop"
install -Dm644 crates/officina-app/assets/icon.png "$PREFIX/share/icons/hicolor/512x512/apps/officina.png"
# update icon cache if available
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
fi
echo "Installed to $PREFIX"
