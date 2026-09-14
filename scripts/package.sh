#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
[ -n "$version" ] || { echo "Missing package version" >&2; exit 1; }
profile=release
binary=
case "${1:-}" in
  --debug) profile=debug; cargo build --locked ;;
  '') cargo build --release --locked ;;
  --prebuilt) binary="${2:?Missing prebuilt executable}" ;;
  *) echo 'Usage: scripts/package.sh [--debug | --prebuilt executable]' >&2; exit 2 ;;
esac
output="${APPDOCK_PACKAGE_DIR:-$PWD/dist}"
mkdir -p "$output"
output="$(cd "$output" && pwd)"
bundle="$output/AppDock.app"
mkdir -p "$bundle/Contents/MacOS"
binary="${binary:-target/$profile/appdock}"
[ -f "$binary" ] || { echo "Missing app executable" >&2; exit 1; }
cp "$binary" "$bundle/Contents/MacOS/AppDock"
chmod +x "$bundle/Contents/MacOS/AppDock"
mkdir -p "$bundle/Contents/Resources"
iconset="$output/AppDock.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" assets/branding/appdock.png \
    --out "$iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" assets/branding/appdock.png \
    --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$bundle/Contents/Resources/AppDock.icns"
cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>dev.appdock.AppDock</string>
<key>CFBundleName</key><string>AppDock</string>
<key>CFBundleDisplayName</key><string>AppDock</string>
<key>CFBundleExecutable</key><string>AppDock</string>
<key>CFBundleIconFile</key><string>AppDock.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>${version%%-*}</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSPrincipalClass</key><string>NSApplication</string>
<key>NSAccessibilityUsageDescription</key><string>AppDock arranges only the windows you choose to organize and restores their original state when released.</string>
</dict></plist>
PLIST
if [ -n "${SPARKLE_SDK:-}" ]; then
  python3 scripts/configure_updates.py "$bundle"
  framework="$bundle/Contents/Frameworks/Sparkle.framework"
  if [ -e "$framework" ]; then
    echo "Use a clean package output directory when embedding Sparkle" >&2
    exit 1
  fi
  mkdir -p "$bundle/Contents/Frameworks"
  ditto "$SPARKLE_SDK/Sparkle.framework" "$framework"
  cp "$SPARKLE_SDK/LICENSE" "$bundle/Contents/Resources/Sparkle-LICENSE.txt"
  # AppDock is not sandboxed; Sparkle explicitly supports omitting these services.
  rm -rf "$framework/Versions/B/XPCServices" "$framework/XPCServices"
  codesign --force --sign - "$framework/Versions/B/Autoupdate"
  codesign --force --sign - "$framework/Versions/B/Updater.app"
  codesign --force --sign - "$framework"
elif [ -e "$bundle/Contents/Frameworks/Sparkle.framework" ]; then
  echo "Refusing stale Sparkle framework without update configuration; use a clean output directory" >&2
  exit 1
fi
plutil -lint "$bundle/Contents/Info.plist"
codesign --force --sign - "$bundle"
codesign --verify --strict "$bundle"
echo "$bundle"
