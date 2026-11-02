#!/bin/sh
# packaging/appimage/build.sh <package-folder> <output-folder>
set -eu
PKG=$(readlink -f "$1"); OUT=$(readlink -f "$2")
HERE=$(dirname "$(readlink -f "$0")")
CACHE=${SRR_CACHE:-$HOME/.cache/srr-appimage}
WINE_VERSION=11.20
WINE_URL=https://github.com/Kron4ek/Wine-Builds/releases/download/$WINE_VERSION/wine-$WINE_VERSION-amd64-wow64.tar.xz
WINE_SHA=482a493c755dcea776199b86560e0fb0f969bb0275c67d88120b0b35b0df34b9
TOOL_URL=https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
TOOL_SHA=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
RUNTIME_VERSION=20251108
RUNTIME_URL=https://github.com/AppImage/type2-runtime/releases/download/$RUNTIME_VERSION/runtime-x86_64
RUNTIME_SHA=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
fetch() {
  mkdir -p "$CACHE"
  if [ ! -f "$CACHE/$3" ]; then
    curl -sfL "$1" -o "$CACHE/$3.part"
    mv "$CACHE/$3.part" "$CACHE/$3"
  fi
  echo "$2  $CACHE/$3" | sha256sum -c --quiet || { echo "checksum mismatch: $3"; rm -f "$CACHE/$3"; exit 1; }
}
fetch "$WINE_URL" "$WINE_SHA" wine.tar.xz
fetch "$TOOL_URL" "$TOOL_SHA" appimagetool
fetch "$RUNTIME_URL" "$RUNTIME_SHA" runtime-x86_64-$RUNTIME_VERSION
chmod +x "$CACHE/appimagetool"
APPDIR=$(mktemp -d)/SRRLauncher.AppDir
trap 'rm -rf "$(dirname "$APPDIR")"' EXIT
mkdir -p "$APPDIR/usr/lib/srr/linux" "$APPDIR/usr/lib/wine"
tar -xJf "$CACHE/wine.tar.xz" -C "$APPDIR/usr/lib/wine" --strip-components=1
rm -rf "$APPDIR/usr/lib/wine/include" "$APPDIR/usr/lib/wine/share/man"
for tool in function_grep.pl widl winebuild winecpp winedump wineg++ winegcc winemaker wmc wrc msidb; do
  rm -f "$APPDIR/usr/lib/wine/bin/$tool"
done
cp "$PKG/SRRLauncher.exe" "$PKG/linux/srr-play" "$APPDIR/usr/lib/srr/"
cp "$PKG/linux/d3d9.dll" "$APPDIR/usr/lib/srr/linux/"
mkdir -p "$APPDIR/usr/share/doc/wine" "$APPDIR/usr/share/doc/dxvk"
cp "$HERE"/wine/* "$APPDIR/usr/share/doc/wine/"
cp "$PKG/linux/dxvk-LICENSE" "$APPDIR/usr/share/doc/dxvk/LICENSE"
cp "$HERE/AppRun" "$APPDIR/AppRun"
cp "$HERE/srr-launcher.desktop" "$HERE/srr-launcher.png" "$APPDIR/"
chmod +x "$APPDIR/AppRun" "$APPDIR/usr/lib/srr/srr-play"
mkdir -p "$OUT"
ARCH=x86_64 "$CACHE/appimagetool" --appimage-extract-and-run --no-appstream --runtime-file "$CACHE/runtime-x86_64-$RUNTIME_VERSION" "$APPDIR" "$OUT/SRRLauncher-x86_64.AppImage"
echo "built $OUT/SRRLauncher-x86_64.AppImage (Wine $WINE_VERSION)"
