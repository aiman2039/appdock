#!/bin/bash
# Package an already-built app. Release CI calls this after stapling the app.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
app="${1:?Usage: package-dmg.sh /path/AppDock.app /path/output.dmg}"
output="${2:?Usage: package-dmg.sh /path/AppDock.app /path/output.dmg}"
if [[ "$(basename "$app")" != AppDock.app || ! -x "$app/Contents/MacOS/AppDock" ]]; then
  echo "Expected an AppDock.app bundle with an executable" >&2
  exit 1
fi
if [[ -e "$output" ]]; then
  echo "Output already exists: $output" >&2
  exit 1
fi
codesign --verify --deep --strict "$app"
mkdir -p "$(dirname "$output")"
"${DMGBUILD:-dmgbuild}" -s "$repo/scripts/dmg-settings.py" -D "app=$app" \
  -D "background=$repo/assets/installer/background.png" \
  "Install AppDock" "$output"
"$repo/scripts/verify-dmg.sh" "$output"
