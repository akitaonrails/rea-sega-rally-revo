#!/bin/sh
# layout.sh — srr-play finds the play library beside itself (zip/AppImage)
# or in ../sdk (this repo), keeps its commands, and fails clearly without it.
set -u
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
export HOME=$T SRR_PREFIX=$T/prefix
unset SRR_LAUNCHER SRR_RUN SRR_WAIT
fails=0
check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: got '$2', want '$3'"; fails=$((fails + 1)); fi; }
mkdir -p "$T/pkg/linux" "$T/bare"
cp "$HERE/../srr-play" "$HERE/../../sdk/rea-play.sh" "$T/pkg/linux/"
cp "$HERE/../srr-play" "$T/bare/"
for play in "$T/pkg/linux/srr-play" "$HERE/../srr-play"; do
  check "status ($play)" "$(sh "$play" status)" idle
  check "stop ($play)" "$(SRR_RUN=true sh "$play" stop)" stopped
  sh "$play" nope >/dev/null 2>&1; check "unknown command exits 2" "$?" 2
done
out=$(sh "$T/pkg/linux/srr-play" --play 2>&1); rc=$?
check "no launcher: exit 1" "$rc" 1
case "$out" in *"SRRLauncher.exe not found"*"SRR_LAUNCHER"*) echo "ok   no launcher: names SRR_LAUNCHER" ;;
  *) echo "FAIL launcher message: $out"; fails=$((fails + 1)) ;; esac
out=$(sh "$T/bare/srr-play" status 2>&1); rc=$?
check "no library: exit 1" "$rc" 1
case "$out" in *"rea-play.sh not found"*) echo "ok   no library: clear message" ;;
  *) echo "FAIL no-library message: $out"; fails=$((fails + 1)) ;; esac
[ "$fails" = 0 ] && echo "layout: ok" || { echo "$fails failed"; exit 1; }
