#!/bin/bash
# Read-only validation of exactly the disk image we are going to distribute.
set -euo pipefail
image="${1:?Usage: verify-dmg.sh image.dmg [expected-architecture]}"
expected_arch="${2:-}"
hdiutil verify "$image"
mount_dir="$(mktemp -d "${TMPDIR:-/tmp}/appdock-dmg-verify.XXXXXX")"
mounted=false
cleanup() {
  if [[ "$mounted" == true ]]; then
    hdiutil detach "$mount_dir" >/dev/null || return 1
  fi
  rmdir "$mount_dir"
}
trap cleanup EXIT
hdiutil attach -readonly -nobrowse -mountpoint "$mount_dir" "$image" >/dev/null
mounted=true
app="$mount_dir/AppDock.app"
test -x "$app/Contents/MacOS/AppDock"
test -f "$mount_dir/.DS_Store"
test -f "$mount_dir/.background.png"
test "$(readlink "$mount_dir/Applications")" = /Applications
codesign --verify --deep --strict --verbose=2 "$app"
if [[ -n "$expected_arch" ]]; then
  if [[ "$expected_arch" == universal ]]; then
    lipo "$app/Contents/MacOS/AppDock" -verify_arch arm64 x86_64
    sparkle="$app/Contents/Frameworks/Sparkle.framework/Versions/B"
    for binary in "$sparkle/Sparkle" "$sparkle/Autoupdate" "$sparkle/Updater.app/Contents/MacOS/Updater"; do
      lipo "$binary" -verify_arch arm64 x86_64
    done
  else
    lipo "$app/Contents/MacOS/AppDock" -verify_arch "$expected_arch"
  fi
fi
printf '%s\n' 'Installer contents, Applications shortcut, and app signature verified.'
