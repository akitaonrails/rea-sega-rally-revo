#!/bin/sh
# winebus.sh — srr-play seeds the controller bridge under the value names
# winebus.sys reads, never overwrites a player's value, and runs once
# (marker .srr-winebus-v2); the winetricks step only counts as done once
# winetricks really ran.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
export HOME=$T SRR_PREFIX=$T/prefix T
unset SRR_WINETRICKS
SRR_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../srr-play"

# A fake runner: records every command; "Enable SDL" already exists;
# winetricks is "installed" when $T/has-winetricks exists.
cat > "$T/run" <<'FAKE'
#!/bin/sh
printf '%s|' "$@" >> "$T/log"; echo >> "$T/log"
case "$*" in
  *"command -v winetricks"*) [ -e "$T/has-winetricks" ]; exit ;;
  *"reg query"*"Enable SDL"*) exit 0 ;;
  *"reg query"*) exit 1 ;;
esac
exit 0
FAKE
chmod +x "$T/run"
REA_RUN=$T/run REA_GPU=
bootstrap > /dev/null

for name in DisableHidraw DisableInput "Map Controllers"; do
  grep -q "|reg|add|.*winebus|/v|$name|/t|REG_DWORD|/d|1|" "$T/log" ||
    { echo "FAIL: $name was not seeded"; cat "$T/log"; exit 1; }
done
if grep -q "Disable Hidraw\|Disable Input" "$T/log"; then
  echo "FAIL: misspelled value names (Wine ignores them)"; exit 1
fi
if grep -q "|reg|add|.*|Enable SDL|" "$T/log"; then
  echo "FAIL: overwrote the player's existing Enable SDL"; exit 1
fi
grep -q "wineserver|-w" "$T/log" || { echo "FAIL: prefix not shut down after seeding"; exit 1; }
grep -q "WINEPREFIX=$SRR_PREFIX|" "$T/log" || { echo "FAIL: not the private prefix"; exit 1; }
[ -f "$SRR_PREFIX/.srr-winebus-v2" ] || { echo "FAIL: winebus marker is not .srr-winebus-v2"; exit 1; }

# Without winetricks the step is skipped and tried again next time.
[ ! -f "$SRR_PREFIX/.srr-tricks" ] || { echo "FAIL: a skipped winetricks step was marked done"; exit 1; }
if seed_winetricks > /dev/null; then echo "FAIL: a skip must return non-zero"; exit 1; fi

# Seeded once: a second bootstrap runs no reg command; winetricks, now
# installed, runs once and is marked done.
: > "$T/log"; : > "$T/has-winetricks"
bootstrap > /dev/null
if grep -q "|reg|" "$T/log"; then echo "FAIL: seeded twice"; exit 1; fi
grep -q "|winetricks|-q|d3dx9|directplay|d3dcompiler_47|" "$T/log" ||
  { echo "FAIL: winetricks did not run"; cat "$T/log"; exit 1; }
[ -f "$SRR_PREFIX/.srr-tricks" ] || { echo "FAIL: winetricks not marked done"; exit 1; }
: > "$T/log"
bootstrap > /dev/null
[ ! -s "$T/log" ] || { echo "FAIL: the bootstrap ran again"; cat "$T/log"; exit 1; }

# SRR_WINETRICKS=off skips it even when installed.
rm -f "$SRR_PREFIX/.srr-tricks"
if SRR_WINETRICKS=off seed_winetricks > /dev/null; then echo "FAIL: off must skip"; exit 1; fi
echo "winebus: ok"
