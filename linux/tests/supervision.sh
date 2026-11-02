#!/bin/sh
# supervision.sh — owned-session lock, launcher argument passing, stop and
# the settings exit codes. Only harmless sleep and fake runners execute.
set -u
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d)
export T HOME=$T SRR_PREFIX=$T/prefix SRR_WINETRICKS=off
STALE_PID= STALE_START= OTHER_PID= OTHER_START=
SRR_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../srr-play"
stop_helper() {
  [ -n "$1" ] || return 0
  stats=$(proc_stat "$1") || return 0
  if [ "${stats#* }" = "$2" ]; then kill "$1" 2>/dev/null || true; fi
  wait "$1" 2>/dev/null || true
}
cleanup() {
  stop_helper "$STALE_PID" "$STALE_START"
  stop_helper "$OTHER_PID" "$OTHER_START"
  rm -rf "$T"
}
trap cleanup EXIT HUP INT TERM
fails=0
ok() { echo "ok   $1"; }
fail() { echo "FAIL $1"; fails=$((fails + 1)); }
: > "$SRR_PREFIX/.srr-winebus-v2"
GPU=

# A fake runner: records arguments, never runs Wine.
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in *wineserver*) echo "$*" >> "$T/wineserver"; exit 0 ;; esac
printf '<%s>\n' "$@" > "$T/args"
FAKE
chmod +x "$T/fake-run"
RUN=$T/fake-run

# A live pid with the wrong start identity models a reused/stale lock.
sleep 30 & STALE_PID=$!
stats=$(proc_stat "$STALE_PID"); STALE_START=${stats#* }
printf '%s %s\n' "$STALE_PID" "$((STALE_START + 1))" > "$LOCK"
if running; then fail "stale identity reported running"; else ok "stale/reused pid identity refused"; fi
stop_session > /dev/null
if kill -0 "$STALE_PID" 2>/dev/null; then ok "stop did not signal stale/reused pid"; else fail "stale helper was signaled"; fi
[ -f "$LOCK" ] && fail "stop left the lock" || ok "stop removes the lock"
case "$(cat "$T/wineserver" 2>/dev/null)" in
  *"WINEPREFIX=$SRR_PREFIX wineserver -k"*) ok "stop shuts down the private prefix" ;;
  *) fail "stop did not run wineserver -k on the prefix" ;;
esac
stop_helper "$STALE_PID" "$STALE_START"; STALE_PID= STALE_START=

# A valid lock (pid + start time of a live session leader) is running, and
# stop signals its whole process group.
setsid sleep 30 & OTHER_PID=$!
sleep 0.2
stats=$(proc_stat "$OTHER_PID"); OTHER_START=${stats#* }
printf '%s %s\n' "$OTHER_PID" "$OTHER_START" > "$LOCK"
if running; then ok "own session reported running"; else fail "own session not seen"; fi
stop_session > /dev/null
wait "$OTHER_PID"; rc=$?
if [ "$rc" = 143 ]; then ok "stop ended the owned session"; else fail "own session ended with $rc, not TERM"; fi
OTHER_PID= OTHER_START=

# An unrelated process with the game's name does not own this launch.
ln -s "$(command -v sleep)" "$T/SEGA Rally_SSE1"
"$T/SEGA Rally_SSE1" 30 & OTHER_PID=$!
stats=$(proc_stat "$OTHER_PID"); OTHER_START=${stats#* }
rm -f "$LOCK"
if running; then fail "unrelated game process blocked launch"; else ok "unrelated game process does not block"; fi
stop_session > /dev/null
if kill -0 "$OTHER_PID" 2>/dev/null; then ok "unrelated game process was not signaled"; else fail "unrelated helper was signaled"; fi
stop_helper "$OTHER_PID" "$OTHER_START"; OTHER_PID= OTHER_START=

# Waiting launch (the AppImage): the lock goes away on natural exit.
mkdir -p "$T/launcher it's \$(here)"
LAUNCHER=$T/"launcher it's \$(here)"/SRRLauncher.exe; : > "$LAUNCHER"
SRR_WAIT=1 start > /dev/null
if [ -f "$LOCK" ]; then fail "natural exit left lock"; else ok "natural supervised exit removes lock"; fi

# Spaces, an apostrophe and $() stay in one positional argument, never source.
if [ "$(tail -1 "$T/args")" = "<$LAUNCHER>" ]; then ok "awkward launcher path is one argument"; else fail "launcher argument: $(tail -1 "$T/args")"; fi
case "$(cat "$STATE/inner.sh")" in
  *'wine "$1" --no-window --linux'*) ok "inner uses quoted positional launcher" ;;
  *) fail "inner does not use wine \"\$1\"" ;;
esac
case "$(cat "$STATE/inner.sh")" in
  *"$LAUNCHER"*) fail "launcher path interpolated into shell source" ;;
  *) ok "launcher path absent from generated source" ;;
esac
sh -n "$STATE/inner.sh" && ok "generated inner.sh parses" || fail "inner.sh syntax"
grep -q '^game_alive()' "$STATE/inner.sh" && ok "inner.sh carries game_alive" || fail "inner.sh lacks game_alive"

# The settings window: Play (0) and unavailable window (4) continue to the
# game; closing it (3) or any other failure does not.
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in *"--settings --linux"*) echo "$*" > "$T/settings"; exit "$(cat "$T/rc")" ;; esac
exit 0
FAKE
for case in 0:0 4:0 3:1 1:1; do
  echo "${case%:*}" > "$T/rc"
  if settings > /dev/null; then got=0; else got=1; fi
  if [ "$got" = "${case#*:}" ]; then ok "settings exit ${case%:*} -> $got"; else fail "settings exit ${case%:*} gave $got"; fi
done
case "$(cat "$T/settings")" in
  *"WINEPREFIX=$SRR_PREFIX wine $LAUNCHER --settings --linux") ok "settings window uses the private prefix and helper flags" ;;
  *) fail "settings command: $(cat "$T/settings")" ;;
esac

[ "$fails" = 0 ] && echo "supervision: ok" || { echo "$fails failed"; exit 1; }
