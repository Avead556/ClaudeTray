#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"

../mac/build.sh >/dev/null
APP=../mac/build/ClaudeTray.app/Contents/MacOS/ClaudeTray
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
snapshot() { "$APP" --snapshot "$@" --demo -AppleLanguages '(en)' -AppleLocale en_US; }

snapshot "$TMP/collapsed.png" --menubar
snapshot "$TMP/expanded.png" --menubar --expanded
snapshot settings.png --settings

ffmpeg -y -loglevel error \
    -loop 1 -t 3 -i "$TMP/collapsed.png" -loop 1 -t 4 -i "$TMP/expanded.png" -loop 1 -t 1 -i "$TMP/collapsed.png" \
    -filter_complex "[0][1]xfade=transition=fade:duration=0.5:offset=2.5[a];[a][2]xfade=transition=fade:duration=0.5:offset=6,fps=20,scale=840:-1:flags=lanczos,split[x][y];[x]palettegen=stats_mode=full[p];[y][p]paletteuse=dither=sierra2_4a" \
    -loop 0 menubar.gif
echo "Rendered docs/menubar.gif and docs/settings.png"
