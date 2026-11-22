#!/usr/bin/env sh
# Officina installer: downloads a verified release binary, or builds from
# source when no release is available.
# Usage:  curl -fsSL https://raw.githubusercontent.com/matteocrippa/officina/main/install.sh | sh
# Local build:  sh install.sh --local
set -eu

PREFIX="${PREFIX:-$HOME/.local}"
if [ "${1:-}" = "--local" ]; then
    SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
    PREFIX="$PREFIX" exec sh "$SCRIPT_DIR/packaging/install.sh"
fi

REPO="matteocrippa/officina"
BIN_DIR="$PREFIX/bin"
APP_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/512x512/apps"
ASSET="officina-linux-x86_64.tar.gz"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

# Try to download a prebuilt release binary.
download_ok=0
if command -v curl >/dev/null 2>&1; then
    LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
    if [ -n "$LATEST" ]; then
        URL="https://github.com/$REPO/releases/download/$LATEST/$ASSET"
        TMP=$(mktemp -d)
        if curl -fsSL "$URL" -o "$TMP/$ASSET" \
            && curl -fsSL "$URL.sha256" -o "$TMP/$ASSET.sha256" \
            && (cd "$TMP" && sha256sum -c "$ASSET.sha256" >/dev/null 2>&1) \
            && tar -xzf "$TMP/$ASSET" -C "$TMP" \
            && [ -f "$TMP/officina" ]; then
            install -m 0755 "$TMP/officina" "$BIN_DIR/officina"
            download_ok=1
        else
            echo "Release download or checksum verification failed; falling back to source." >&2
        fi
        rm -rf "$TMP"
    fi
fi

# Fall back to building from source if no release binary was obtained.
if [ "$download_ok" -ne 1 ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        echo "No prebuilt release available and cargo not found." >&2
        echo "Install Rust (https://rustup.rs) or download a release manually." >&2
        exit 1
    fi
    echo "Building from source..."
    SRC_DIR="${TMPDIR:-/tmp}/officina-src"
    rm -rf "$SRC_DIR"
    git clone --depth 1 "https://github.com/$REPO.git" "$SRC_DIR"
    (cd "$SRC_DIR" && cargo build --release -p officina-app)
    install -m 0755 "$SRC_DIR/target/release/officina" "$BIN_DIR/officina"
    ICON_SRC="$SRC_DIR/crates/officina-app/assets/icon.png"
    DESKTOP_SRC="$SRC_DIR/packaging/officina.desktop"
    rm -rf "$SRC_DIR"
else
    ICON_SRC=""
    DESKTOP_SRC=""
fi

# Install desktop entry + icon when building from source (icon file ships in repo).
if [ -z "${ICON_SRC:-}" ]; then
    # download icon from repo for binary installs
    ICON_URL="https://raw.githubusercontent.com/$REPO/main/crates/officina-app/assets/icon.png"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$ICON_URL" -o "$ICON_DIR/officina.png" 2>/dev/null || true
    fi
else
    if [ -f "$ICON_SRC" ]; then
        install -m 0644 "$ICON_SRC" "$ICON_DIR/officina.png"
    fi
fi

if [ -n "${DESKTOP_SRC:-}" ] && [ -f "$DESKTOP_SRC" ]; then
    install -m 0644 "$DESKTOP_SRC" "$APP_DIR/officina.desktop"
else
    cat > "$APP_DIR/officina.desktop" <<EOF
[Desktop Entry]
Name=Officina
Comment=Mod manager and launcher for Assetto Corsa (Linux)
Exec=$BIN_DIR/officina
Icon=officina
Type=Application
Categories=Game;Utility;
StartupWMClass=officina
EOF
fi

# Refresh icon cache if possible.
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
fi

echo "Installed officina to $BIN_DIR/officina"
echo "Make sure $BIN_DIR is on your PATH."
