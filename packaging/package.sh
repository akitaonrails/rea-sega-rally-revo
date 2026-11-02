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

# DXVK: line 1 of dxvk.txt is the release tarball, line 2 its licence.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
grep -v '^#' "$ROOT/packaging/dxvk.txt" > "$work/pins"
{ read -r url digest; read -r licence_url licence_digest; } < "$work/pins"
curl -fsSL "$url" -o "$work/dxvk.tar.gz"
echo "$digest  $work/dxvk.tar.gz" | sha256sum -c -
top=$(basename "$url" .tar.gz)
tar -xOf "$work/dxvk.tar.gz" "$top/x32/d3d9.dll" > "$pkg/linux/d3d9.dll"
curl -fsSL "$licence_url" -o "$pkg/linux/dxvk-LICENSE"
echo "$licence_digest  $pkg/linux/dxvk-LICENSE" | sha256sum -c -

(cd "$dist" && rm -f "$name.zip" && zip -qr "$name.zip" "$name")

sh "$ROOT/sdk/packaging/appimage-wine.sh" --app srr --exe SRRLauncher.exe \
  --icon "$ROOT/packaging/appimage/srr-launcher.png" \
  --desktop "$ROOT/packaging/appimage/srr-launcher.desktop" \
  --payload linux/d3d9.dll --doc linux/dxvk-LICENSE \
  ${REA_CACHE:+--cache "$REA_CACHE"} \
  --name "$name" "$pkg" "$dist"
