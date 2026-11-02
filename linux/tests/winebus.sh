#!/bin/sh
# winebus.sh — srr-play seeds the controller bridge under the value names
# winebus.sys reads, never overwrites a player's value, and runs once
# (marker .srr-winebus-v2), and never runs winetricks (the DirectX 9
# files are the launcher's Download button now).
set -eu
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
export HOME=$T SRR_PREFIX=$T/prefix T
REA_FUNCTIONS_ONLY=1 REA_PLAY_HERE=$HERE/.. . "$HERE/../srr-play"

# A fake runner: records every command; "Enable SDL" already exists;
# winetricks is "installed".
cat > "$T/run" <<'FAKE'
#!/bin/sh
printf '%s|' "$@" >> "$T/log"; echo >> "$T/log"
case "$*" in
  *"command -v winetricks"*) exit 0 ;;
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

# winetricks is never run, even when installed.
if grep -q "winetricks" "$T/log"; then echo "FAIL: winetricks ran"; cat "$T/log"; exit 1; fi

# Seeded once: a second bootstrap runs nothing.
: > "$T/log"
bootstrap > /dev/null
[ ! -s "$T/log" ] || { echo "FAIL: the bootstrap ran again"; cat "$T/log"; exit 1; }
echo "winebus: ok"
