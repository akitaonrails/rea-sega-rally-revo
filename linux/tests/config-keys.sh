#!/bin/sh
# config-keys.sh — the launcher.cfg reader used by srr-play.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
SRR_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../srr-play"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
CFG=$tmp/launcher.cfg
cat > "$CFG" <<'EOF'
# comment
game = Z:\home\me\Games\Sega Rally
resolution = 3840x2160
aspect = 21:9
fullscreen = false
EOF

[ "$(get_key game)" = 'Z:\home\me\Games\Sega Rally' ] || { echo "FAIL: game"; exit 1; }
[ "$(get_key resolution)" = "3840x2160" ] || { echo "FAIL: resolution"; exit 1; }
[ "$(get_key aspect)" = "21:9" ] || { echo "FAIL: aspect"; exit 1; }
[ "$(get_key fullscreen)" = "false" ] || { echo "FAIL: fullscreen"; exit 1; }
[ -z "$(get_key missing)" ] || { echo "FAIL: missing key"; exit 1; }

# the last value wins
printf 'vsync = false\nvsync = true\n' > "$CFG"
[ "$(get_key vsync)" = "true" ] || { echo "FAIL: last value"; exit 1; }

echo "config-keys: ok"
