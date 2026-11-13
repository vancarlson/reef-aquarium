#!/usr/bin/env bash
# Start the aquarium fullscreen. Esc or Q quits, F toggles fullscreen.
#   ./run.sh                    fullscreen
#   ./run.sh --windowed         in a window
#   ./run.sh --help             all options (fish count, seagrass, speed, ...)
cd "$(dirname "$(readlink -f "$0")")" || exit 1
export BEVY_ASSET_ROOT="$PWD"
[ -x target/release/aquarium ] || cargo build --release || exit 1
exec target/release/aquarium "$@"
