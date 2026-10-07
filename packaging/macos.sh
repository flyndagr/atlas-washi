#!/bin/bash
set -euo pipefail
ATLAS_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ATLAS_ROOT"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
cargo build --release --locked
ATLAS_APP="$ATLAS_ROOT/dist/Atlas.app"
mkdir -p "$ATLAS_APP/Contents/MacOS" "$ATLAS_APP/Contents/Resources"
cp target/release/atlas "$ATLAS_APP/Contents/MacOS/Atlas"
cp assets/Atlas.icns "$ATLAS_APP/Contents/Resources/Atlas.icns"
cp assets/OFL-*.txt "$ATLAS_APP/Contents/Resources/"
cat > "$ATLAS_APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Atlas</string>
<key>CFBundleDisplayName</key><string>Atlas</string>
<key>CFBundleExecutable</key><string>Atlas</string>
<key>CFBundleIdentifier</key><string>local.atlas.notebook</string>
<key>CFBundleIconFile</key><string>Atlas</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.2.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
</dict></plist>
PLIST
codesign --force --sign - "$ATLAS_APP"
codesign --verify --strict --deep "$ATLAS_APP"
echo "Built $ATLAS_APP"
