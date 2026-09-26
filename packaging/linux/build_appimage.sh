#!/usr/bin/env bash
# Packs a Linux release bundle into an AppImage (the primary Linux format, brief §3).
#   packaging/linux/build_appimage.sh <bundle-dir> <version> <out-dir>
# Needs appimagetool on PATH, or APPIMAGETOOL pointing at it.
set -euo pipefail
BUNDLE=$1 VERSION=$2 OUT=$3
HERE=$(cd "$(dirname "$0")" && pwd)
TOOL=${APPIMAGETOOL:-appimagetool}
APPDIR=$(mktemp -d)/Daftar.AppDir
trap 'rm -rf "$(dirname "$APPDIR")"' EXIT
mkdir -p "$APPDIR/usr/share/metainfo"
cp -a "$BUNDLE"/. "$APPDIR/"
cp "$HERE/dev.daftar.Daftar.desktop" "$APPDIR/"
cp "$HERE/dev.daftar.Daftar.png" "$APPDIR/"
cp "$HERE/dev.daftar.Daftar.metainfo.xml" "$APPDIR/usr/share/metainfo/dev.daftar.Daftar.appdata.xml"
cat > "$APPDIR/AppRun" <<'RUN'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
exec "$HERE/daftar" "$@"
RUN
chmod +x "$APPDIR/AppRun"
mkdir -p "$OUT"
# --appimage-extract-and-run lets the tool itself run without FUSE (containers, CI).
ARCH=x86_64 VERSION=$VERSION "$TOOL" --appimage-extract-and-run --no-appstream "$APPDIR" \
  "$OUT/Daftar-${VERSION}-x86_64.AppImage"
