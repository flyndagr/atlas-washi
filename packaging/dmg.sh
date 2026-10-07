#!/bin/bash
# Package the verified release bundle; run packaging/macos.sh first.
set -euo pipefail
ATLAS_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ATLAS_APP="$ATLAS_ROOT/dist/Atlas.app"
ATLAS_VERSION=$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "$ATLAS_APP/Contents/Info.plist")
ATLAS_ARCH=$(lipo -archs "$ATLAS_APP/Contents/MacOS/Atlas")
if [[ "$ATLAS_ARCH" != "arm64" ]]; then
  echo "Expected the Apple Silicon release bundle; found $ATLAS_ARCH" >&2
  exit 1
fi
codesign --verify --strict --deep "$ATLAS_APP"
ATLAS_STAGE=$(mktemp -d "${TMPDIR:-/tmp}/atlas-dmg.XXXXXX")
trap 'rm -rf "$ATLAS_STAGE"' EXIT
 ditto "$ATLAS_APP" "$ATLAS_STAGE/Atlas.app"
ln -s /Applications "$ATLAS_STAGE/Applications"
cat > "$ATLAS_STAGE/Read me.txt" <<'NOTES'
Atlas — The Washi Edition

Drag Atlas.app to Applications, then launch it and choose a notebook folder.
This early release is for Apple Silicon Macs running macOS 12 or later.

The app is locally signed, but not Apple Developer ID signed or notarized.
If macOS blocks it, only open it if you trust the release. See Apple's guidance:
https://support.apple.com/en-us/102445
Do not disable Gatekeeper or other system security protections.

Notes are stored as Markdown files. Keep backups of important notes.
Source, feedback, and license: https://github.com/flyndagr/atlas-washi
NOTES
ATLAS_DMG="$ATLAS_ROOT/dist/Atlas-$ATLAS_VERSION-apple-silicon.dmg"
hdiutil create -volname "Atlas $ATLAS_VERSION" -srcfolder "$ATLAS_STAGE" -ov -format UDZO "$ATLAS_DMG"
hdiutil verify "$ATLAS_DMG"
(cd "$ATLAS_ROOT/dist" && shasum -a 256 "$(basename "$ATLAS_DMG")" > SHA256SUMS.txt)
echo "Created $ATLAS_DMG"
