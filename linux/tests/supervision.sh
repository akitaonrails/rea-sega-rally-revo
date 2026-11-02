#!/bin/sh
# supervision.sh — owned-session lock, launcher argument passing, stop and
# the settings exit codes. Only harmless sleep and fake runners execute.
set -u
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d)
export T HOME=$T SRR_PREFIX=$T/prefix SRR_WINETRICKS=off
unset SRR_WAIT
STALE_PID= STALE_START= OTHER_PID= OTHER_START=
SRR_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../srr-play"
REA_TEST=1
stop_helper() {
  [ -n "$1" ] || return 0
  stats=$(rea_proc_stat "$1") || return 0
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
REA_GPU=

# A fake runner: records arguments, never runs Wine.
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in *wineserver*) echo "$*" >> "$T/wineserver"; exit 0 ;; esac
printf '<%s>\n' "$@" > "$T/args"
FAKE
chmod +x "$T/fake-run"
REA_RUN=$T/fake-run

# A live pid with the wrong start identity models a reused/stale lock.
sleep 30 & STALE_PID=$!
stats=$(rea_proc_stat "$STALE_PID"); STALE_START=${stats#* }
printf '%s %s\n' "$STALE_PID" "$((STALE_START + 1))" > "$REA_LOCK"
if rea_running; then fail "stale identity reported running"; else ok "stale/reused pid identity refused"; fi
rea_stop > /dev/null
if kill -0 "$STALE_PID" 2>/dev/null; then ok "stop did not signal stale/reused pid"; else fail "stale helper was signaled"; fi
[ -f "$REA_LOCK" ] && fail "stop left the lock" || ok "stop removes the lock"
case "$(cat "$T/wineserver" 2>/dev/null)" in
  *"WINEPREFIX=$SRR_PREFIX wineserver -k"*) ok "stop shuts down the private prefix" ;;
  *) fail "stop did not run wineserver -k on the prefix" ;;
esac
stop_helper "$STALE_PID" "$STALE_START"; STALE_PID= STALE_START=

# A valid lock (pid + start time of a live session leader) is running, and
# stop signals its whole process group.
setsid sleep 30 & OTHER_PID=$!
sleep 0.2
stats=$(rea_proc_stat "$OTHER_PID"); OTHER_START=${stats#* }
printf '%s %s\n' "$OTHER_PID" "$OTHER_START" > "$REA_LOCK"
if rea_running; then ok "own session reported running"; else fail "own session not seen"; fi
rea_stop > /dev/null
wait "$OTHER_PID"; rc=$?
if [ "$rc" = 143 ]; then ok "stop ended the owned session"; else fail "own session ended with $rc, not TERM"; fi
OTHER_PID= OTHER_START=

# An unrelated process with the game's name does not own this launch.
ln -s "$(command -v sleep)" "$T/SEGA Rally_SSE1"
"$T/SEGA Rally_SSE1" 30 & OTHER_PID=$!
stats=$(rea_proc_stat "$OTHER_PID"); OTHER_START=${stats#* }
rm -f "$REA_LOCK"
if rea_running; then fail "unrelated game process blocked launch"; else ok "unrelated game process does not block"; fi
rea_stop > /dev/null
if kill -0 "$OTHER_PID" 2>/dev/null; then ok "unrelated game process was not signaled"; else fail "unrelated helper was signaled"; fi
stop_helper "$OTHER_PID" "$OTHER_START"; OTHER_PID= OTHER_START=

# A v0.1.x lock (pid only) with a live pid still counts as running, so an
# upgrade does not start a second session; stop ends it.
setsid sleep 30 & OTHER_PID=$!
sleep 0.2
echo "$OTHER_PID" > "$REA_LOCK"
if rea_running; then ok "old pid-only lock with a live pid is running"; else fail "old-format lock ignored"; fi
rea_stop > /dev/null
wait "$OTHER_PID"; rc=$?
if [ "$rc" = 143 ]; then ok "stop ended the old-format session"; else fail "old session ended with $rc"; fi
OTHER_PID= OTHER_START=
echo 999999999 > "$REA_LOCK"
if rea_running; then fail "dead old-format pid reported running"; else ok "dead old-format pid is idle"; fi
rm -f "$REA_LOCK"

# Interrupting a waiting launch signals the session and waits for it.
cat > "$T/slow" <<'SLOW'
#!/bin/sh
trap 'sleep 1; echo cleaned > "$T/cleaned"; exit 0' TERM
sleep 30 & wait
SLOW
chmod +x "$T/slow"
(
  REA_RUN= REA_WAIT=1
  rea_launch "$T/slow"
) &
SUP=$!
for _ in 1 2 3 4 5 6 7 8 9 10; do [ -s "$REA_LOCK" ] && break; sleep 0.2; done
sleep 0.3
kill -TERM "$SUP"; wait "$SUP"; rc=$?
if [ "$rc" = 143 ] && [ -f "$T/cleaned" ]; then ok "interrupt waited for the session's cleanup"; else fail "interrupt rc=$rc cleaned=$(cat "$T/cleaned" 2>/dev/null)"; fi
[ -f "$REA_LOCK" ] && fail "interrupt left the lock" || ok "interrupt removed the lock"

# Waiting launch (the AppImage): the lock goes away on natural exit.
mkdir -p "$T/launcher it's \$(here)"
REA_EXE=$T/"launcher it's \$(here)"/SRRLauncher.exe; : > "$REA_EXE"
REA_WAIT=1 rea_start > /dev/null
if [ -f "$REA_LOCK" ]; then fail "natural exit left lock"; else ok "natural supervised exit removes lock"; fi

# Spaces, an apostrophe and $() stay in one positional argument, never source.
if [ "$(grep -n . "$T/args" | grep -F "<$REA_EXE>" | head -1 | cut -d: -f2-)" = "<$REA_EXE>" ]; then ok "awkward launcher path is one argument"; else fail "launcher argument: $(grep -n . "$T/args" | grep -F "<$REA_EXE>" | head -1 | cut -d: -f2-)"; fi
case "$(tr '\n' ' ' < "$T/args")" in
  *"<$REA_EXE> <--no-window> <--linux>"*) ok "the game run passes --no-window --linux" ;;
  *) fail "launcher flags: $(cat "$T/args")" ;;
esac
case "$(tr '\n' ' ' < "$T/args")" in
  *"<REA_GAME_PROCS=SEGA Rally.exe|SEGA Rally_SSE1>"*) ok "the inner script waits for the game" ;;
  *) fail "game processes not passed: $(cat "$T/args")" ;;
esac
case "$(tr '\n' ' ' < "$T/args")" in
  *"<SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1>"*) ok "the game gets SDL background events" ;;
  *) fail "SDL env missing: $(cat "$T/args")" ;;
esac
case "$(tr '\n' ' ' < "$T/args")" in
  *gamescope*) fail "srr-play must never start gamescope" ;;
  *) ok "no gamescope" ;;
esac
case "$(cat "$REA_STATE/inner.sh")" in
  *'wine "$@"'*) ok "inner uses quoted positional launcher" ;;
  *) fail "inner does not use wine \"\$@\"" ;;
esac
case "$(cat "$REA_STATE/inner.sh")" in
  *"$REA_EXE"*) fail "launcher path interpolated into shell source" ;;
  *) ok "launcher path absent from generated source" ;;
esac
sh -n "$REA_STATE/inner.sh" && ok "generated inner.sh parses" || fail "inner.sh syntax"
grep -q '^rea_game_alive()' "$REA_STATE/inner.sh" && ok "inner.sh carries the game wait" || fail "inner.sh lacks rea_game_alive"

# The settings window: Play (0) and unavailable window (4) continue to the
# game; closing it (3) or any other failure does not.
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in *"--settings --linux"*) echo "$*" > "$T/settings"; exit "$(cat "$T/rc")" ;; esac
exit 0
FAKE
for case in 0:0 4:0 3:1 1:1; do
  echo "${case%:*}" > "$T/rc"
  if rea_settings > /dev/null; then got=0; else got=1; fi
  if [ "$got" = "${case#*:}" ]; then ok "settings exit ${case%:*} -> $got"; else fail "settings exit ${case%:*} gave $got"; fi
done
case "$(cat "$T/settings")" in
  *"WINEPREFIX=$SRR_PREFIX wine $REA_EXE --settings --linux") ok "settings window uses the private prefix and helper flags" ;;
  *) fail "settings command: $(cat "$T/settings")" ;;
esac

[ "$fails" = 0 ] && echo "supervision: ok" || { echo "$fails failed"; exit 1; }
