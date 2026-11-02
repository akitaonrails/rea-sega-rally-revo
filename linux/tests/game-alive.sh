#!/bin/sh
# game-alive.sh — a zombie game process must not count as running.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
SRR_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../srr-play"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin" "$tmp/proc/4242"
# a fake pgrep that reports one SEGA Rally_SSE1 process, pid 4242
cat > "$tmp/bin/pgrep" <<'FAKE'
#!/bin/sh
[ "$2" = "SEGA Rally_SSE1" ] && echo 4242
exit 0
FAKE
chmod +x "$tmp/bin/pgrep"
PATH="$tmp/bin:$PATH"; export SRR_PROC="$tmp/proc"

echo '4242 (SEGA Rally_SSE1) Z 1 4242' > "$tmp/proc/4242/stat"
if game_alive; then echo "FAIL: a zombie counted as a running game"; exit 1; fi

echo '4242 (SEGA Rally_SSE1) S 1 4242' > "$tmp/proc/4242/stat"
game_alive || { echo "FAIL: a sleeping game process was not seen"; exit 1; }

rm "$tmp/proc/4242/stat"
if game_alive; then echo "FAIL: a vanished process counted as running"; exit 1; fi
echo "game-alive: ok"
