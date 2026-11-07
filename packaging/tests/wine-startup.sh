#!/bin/sh
# wine-startup.sh — the bundled Wine must start without dying by a signal
# and without reserving an absurd address space.
#
# Why this exists: Wine 11.20 sized its page-protection table from a
# host-address-space probe that ignored MAP_FIXED_NOREPLACE, reserving
# ~64 GiB of virtual address space per Wine process. Machines that cannot
# reserve that (16 GB handhelds) got a SIGSEGV in ntdll.so at the first
# view (issue #3); machines with more RAM only looked fine. 11.21 fixes
# the probe and reserves ~8 GiB. The threshold is far from both numbers so
# it fails only on that class of bug, not on a busy host.
#
#   SRR_APPIMAGE=dist/SRRLauncher-vX-x86_64.AppImage sh packaging/tests/wine-startup.sh
#   SRR_WINE=path/to/extracted/wine sh packaging/tests/wine-startup.sh
#
# With neither set it skips (the CI package step runs it with the built
# AppImage).
set -u
LIMIT_KIB=$((32 * 1024 * 1024))   # 32 GiB of VmPeak
APPIMAGE=${SRR_APPIMAGE:-}
WINE_DIR=${SRR_WINE:-}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/srr-wine-startup.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

if [ -z "$WINE_DIR" ]; then
  if [ -z "$APPIMAGE" ]; then
    echo "wine-startup: skipped (set SRR_APPIMAGE or SRR_WINE)"
    exit 0
  fi
  [ -f "$APPIMAGE" ] || { echo "wine-startup: no such AppImage: $APPIMAGE" >&2; exit 1; }
  (cd "$tmp" && "$APPIMAGE" --appimage-extract >/dev/null 2>&1) ||
    { echo "wine-startup: cannot extract $APPIMAGE" >&2; exit 1; }
  WINE_DIR=$tmp/squashfs-root/usr/lib/wine
fi
[ -x "$WINE_DIR/bin/wine" ] || { echo "wine-startup: no bin/wine in $WINE_DIR" >&2; exit 1; }

version=$(PATH=$WINE_DIR/bin:$PATH wine --version 2>/dev/null) || version=unknown
prefix=$tmp/prefix
PATH=$WINE_DIR/bin:$PATH WINEPREFIX=$prefix WINEDEBUG=-all \
  timeout 300 wine wineboot --init >"$tmp/wine.log" 2>&1 &
pid=$!

# Sample every Wine process' virtual-address peak while it starts.
peak=0
while kill -0 "$pid" 2>/dev/null; do
  for p in $(pgrep -f "$WINE_DIR" 2>/dev/null); do
    cur=$(awk '/^VmPeak:/{print $2}' "/proc/$p/status" 2>/dev/null) || continue
    [ -n "$cur" ] || continue
    [ "$cur" -gt "$peak" ] 2>/dev/null && peak=$cur
  done
  sleep 0.05
done
rc=0
wait "$pid" 2>/dev/null || rc=$?

status=ok
if [ "$rc" -ge 128 ]; then
  echo "wine-startup: FAIL: $version died from signal $((rc - 128)) at startup" >&2
  tail -5 "$tmp/wine.log" >&2
  status=fail
fi
if [ "$peak" -ge "$LIMIT_KIB" ]; then
  echo "wine-startup: FAIL: $version reserved $((peak / 1024 / 1024)) GiB of address space at startup (limit: 32 GiB)" >&2
  status=fail
elif [ "$peak" -gt 0 ]; then
  echo "wine-startup: $version reserved $((peak / 1024)) MiB (${peak} KiB) of address space"
fi
[ "$status" = ok ] || exit 1
echo "wine-startup: ok"
