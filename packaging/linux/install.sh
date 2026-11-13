#!/usr/bin/env bash
# Adds Reef Aquarium to your applications menu, and optionally starts it when you log in.
#   ./install.sh                add to the menu
#   ./install.sh --autostart    add to the menu and start at login
#   ./install.sh --uninstall    remove both
set -e
DIR="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
AUTO="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
FILE=reef-aquarium.desktop

if [ "$1" = "--uninstall" ]; then
  rm -f "$APPS/$FILE" "$AUTO/$FILE" "$ICONS/reef-aquarium.png"
  echo "Removed."
  exit 0
fi

[ -x "$DIR/aquarium" ] || { echo "Run this from the unpacked reef-aquarium folder (the 'aquarium' program is missing)."; exit 1; }
mkdir -p "$APPS" "$ICONS"
cp "$DIR/reef-aquarium.png" "$ICONS/reef-aquarium.png"
cat > "$APPS/$FILE" <<EOF
[Desktop Entry]
Type=Application
Name=Reef Aquarium
Comment=A calm, photographic reef aquarium
Exec="$DIR/aquarium"
Path=$DIR
Icon=reef-aquarium
Terminal=false
Categories=Graphics;Screensaver;
EOF
echo "Added Reef Aquarium to your applications menu."
if [ "$1" = "--autostart" ]; then
  mkdir -p "$AUTO"
  cp "$APPS/$FILE" "$AUTO/$FILE"
  echo "It will now start when you log in (remove with: $0 --uninstall)."
fi
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true
