#!/usr/bin/env bash
# Packs a Linux release bundle into a .deb: the bundle under /opt/daftar, a launcher in /usr/bin,
# the desktop entry, AppStream metadata and the icon.
#   packaging/linux/build_deb.sh <bundle-dir> <version> <out-dir>
# Set MAINTAINER="Name <email>" for a published package.
set -euo pipefail
BUNDLE=$1 VERSION=$2 OUT=$3
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT
chmod 755 "$ROOT"
mkdir -p "$ROOT/DEBIAN" "$ROOT/opt/daftar" "$ROOT/usr/bin" \
  "$ROOT/usr/share/applications" "$ROOT/usr/share/metainfo" \
  "$ROOT/usr/share/icons/hicolor/512x512/apps"
cp -a "$BUNDLE"/. "$ROOT/opt/daftar/"
ln -s /opt/daftar/daftar "$ROOT/usr/bin/daftar"
cp "$HERE/dev.daftar.Daftar.desktop" "$ROOT/usr/share/applications/"
cp "$HERE/dev.daftar.Daftar.metainfo.xml" "$ROOT/usr/share/metainfo/"
cp "$HERE/dev.daftar.Daftar.png" "$ROOT/usr/share/icons/hicolor/512x512/apps/"
cat > "$ROOT/DEBIAN/control" <<CONTROL
Package: daftar
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Depends: libgtk-3-0, libsecret-1-0, libgstreamer1.0-0, libgstreamer-plugins-base1.0-0, gstreamer1.0-plugins-good, libwebkit2gtk-4.1-0
Maintainer: ${MAINTAINER:-Daftar maintainers <maintainers@example.invalid>}
Description: Private notebook that files what you say into your own wiki
 Voice, text and photo captures are saved to a Git repository you own and
 filed into a bilingual wiki by the AI models you choose.
CONTROL
mkdir -p "$OUT"
dpkg-deb --root-owner-group --build "$ROOT" "$OUT/daftar_${VERSION}_amd64.deb"
