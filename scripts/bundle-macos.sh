#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
profile="${1:-debug}"
case "$profile" in debug) ;; release) ;; *) printf 'Usage: %s [debug|release]\n' "$0" >&2; exit 2;; esac
if [ "$profile" = release ]; then
  "${CARGO:-$HOME/.cargo/bin/cargo}" build --release --locked
else
  "${CARGO:-$HOME/.cargo/bin/cargo}" build --locked
fi
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
bundle="target/$profile/Informe.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "target/$profile/informe" "$bundle/Contents/MacOS/informe.new"
mv "$bundle/Contents/MacOS/informe.new" "$bundle/Contents/MacOS/informe"
cp examples/viewer/dashboard.json "$bundle/Contents/Resources/dashboard.json"
cp schema/artifact-v1.json "$bundle/Contents/Resources/artifact-v1.json"
cp assets/brand/AppIcon.icns "$bundle/Contents/Resources/AppIcon.icns"
cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>informe</string>
<key>CFBundleIdentifier</key><string>dev.nachogea.informe</string>
<key>CFBundleName</key><string>Informe</string>
<key>CFBundleDisplayName</key><string>Informe</string>
<key>CFBundleIconFile</key><string>AppIcon</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>${MACOSX_DEPLOYMENT_TARGET:-26.0}</string>
</dict></plist>
PLIST
printf 'App bundle: %s/%s\n' "$PWD" "$bundle"
