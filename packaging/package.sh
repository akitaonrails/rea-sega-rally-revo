#!/bin/sh
# packaging/package.sh <version-name> <dist-folder> <release-folder>
# Builds dist/SRRLauncher-<version-name>/ (with the pinned DXVK d3d9.dll and
# its licence), its .zip, and the AppImage through the SDK's builder.
# Needs SRRLauncher.exe in <release-folder>; REA_CACHE caches downloads.
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd -P)
version=$1 dist=$2 release=$3
case $version in '' | *[!A-Za-z0-9._-]*) echo "package: bad version name '$version'" >&2; exit 2 ;; esac
name=SRRLauncher-$version
pkg=$dist/$name
rm -rf "$pkg"
mkdir -p "$pkg"
cp "$release/SRRLauncher.exe" "$pkg/"
cp "$ROOT/README.md" "$ROOT/THIRD_PARTY.md" "$ROOT/LICENSE-MIT" "$ROOT/LICENSE-APACHE" "$pkg/"
cp -r "$ROOT/linux" "$pkg/linux"
# srr-play finds the SDK play library beside itself.
cp "$ROOT/sdk/rea-play.sh" "$pkg/linux/rea-play.sh"
chmod +x "$pkg/linux/srr-play"

# DXVK from its pins (verified sha256, mirror fallback, cached).
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
. "$ROOT/sdk/packaging/pins.sh"
PIN_FILE=$ROOT/packaging/third-party.txt
if [ -n "${REA_CACHE:-}" ]; then PIN_CACHE=$REA_CACHE/pins; fi
tarball=$(pin_fetch dxvk "$work")
top=$(basename "$tarball" .tar.gz)
tar -xOf "$tarball" "$top/x32/d3d9.dll" > "$pkg/linux/d3d9.dll"
licence=$(pin_fetch dxvk-license "$work")
cp "$licence" "$pkg/linux/dxvk-LICENSE"

(cd "$dist" && rm -f "$name.zip" && zip -qr "$name.zip" "$name")

sh "$ROOT/sdk/packaging/appimage-wine.sh" --app srr --exe SRRLauncher.exe \
  --icon "$ROOT/packaging/appimage/srr-launcher.png" \
  --desktop "$ROOT/packaging/appimage/srr-launcher.desktop" \
  --payload linux/d3d9.dll --doc linux/dxvk-LICENSE \
  ${REA_CACHE:+--cache "$REA_CACHE"} \
  --name "$name" "$pkg" "$dist"
