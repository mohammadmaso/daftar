#!/usr/bin/env bash
# Packs Daftar.app into a DMG with an Applications shortcut. Signs and notarizes when the
# credentials are set (docs/packaging.md); without them the DMG is unsigned, for testing only.
#   packaging/macos/build_dmg.sh <Daftar.app> <version> <out-dir>
# Optional env: MACOS_SIGN_IDENTITY ("Developer ID Application: …"),
#               NOTARY_APPLE_ID, NOTARY_TEAM_ID, NOTARY_PASSWORD (app-specific password).
set -euo pipefail
APP=$1 VERSION=$2 OUT=$3
HERE=$(cd "$(dirname "$0")" && pwd)
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP" "$STAGE/"
NAME=$(basename "$APP")
if [[ -n "${MACOS_SIGN_IDENTITY:-}" ]]; then
  codesign --force --deep --timestamp --options runtime \
    --entitlements "$HERE/../../app/macos/Runner/Release.entitlements" \
    --sign "$MACOS_SIGN_IDENTITY" "$STAGE/$NAME"
fi
ln -s /Applications "$STAGE/Applications"
mkdir -p "$OUT"
DMG="$OUT/Daftar-${VERSION}.dmg"
hdiutil create -volname Daftar -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
if [[ -n "${MACOS_SIGN_IDENTITY:-}" ]]; then
  codesign --timestamp --sign "$MACOS_SIGN_IDENTITY" "$DMG"
  if [[ -n "${NOTARY_APPLE_ID:-}" ]]; then
    xcrun notarytool submit "$DMG" --apple-id "$NOTARY_APPLE_ID" --team-id "$NOTARY_TEAM_ID" \
      --password "$NOTARY_PASSWORD" --wait
    xcrun stapler staple "$DMG"
  fi
fi
echo "$DMG"
