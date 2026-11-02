#!/bin/sh
# game-alive.sh — srr-play waits for its own game processes; a zombie must
# not count as running.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export HOME=$tmp
SRR_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../srr-play"
REA_TEST=1

[ "$REA_GAME_PROCS" = 'SEGA Rally.exe|SEGA Rally_SSE1' ] ||
  { echo "FAIL: game process names: $REA_GAME_PROCS"; exit 1; }

mkdir -p "$tmp/bin" "$tmp/proc/4242"
# a fake pgrep that reports one SEGA Rally_SSE1 process, pid 4242
cat > "$tmp/bin/pgrep" <<'FAKE'
#!/bin/sh
[ "$2" = "SEGA Rally_SSE1" ] && echo 4242
exit 0
FAKE
chmod +x "$tmp/bin/pgrep"
PATH="$tmp/bin:$PATH"; REA_PROC="$tmp/proc"

echo '4242 (SEGA Rally_SSE1) Z 1 4242' > "$tmp/proc/4242/stat"
if rea_game_alive; then echo "FAIL: a zombie counted as a running game"; exit 1; fi

echo '4242 (SEGA Rally_SSE1) S 1 4242' > "$tmp/proc/4242/stat"
rea_game_alive || { echo "FAIL: a sleeping game process was not seen"; exit 1; }

rm "$tmp/proc/4242/stat"
if rea_game_alive; then echo "FAIL: a vanished process counted as running"; exit 1; fi
echo "game-alive: ok"
