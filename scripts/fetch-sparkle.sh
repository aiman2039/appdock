#!/bin/bash
# Pin both the SDK version and its official release-asset digest.
set -euo pipefail
version=2.10.0
checksum=c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c
output="${1:-target/sparkle}"
mkdir -p "$output"
output="$(cd "$output" && pwd)"
archive="$output/Sparkle-$version.tar.xz"
if [[ ! -f "$archive" ]]; then
  curl --fail --silent --show-error --location --retry 3 \
    "https://github.com/sparkle-project/Sparkle/releases/download/$version/Sparkle-$version.tar.xz" \
    --output "$archive.download"
  mv "$archive.download" "$archive"
fi
printf '%s  %s\n' "$checksum" "$archive" | shasum -a 256 --check >&2
sdk="$output/Sparkle-$version"
mkdir -p "$sdk"
tar -xf "$archive" -C "$sdk"
test -x "$sdk/bin/generate_appcast"
test -x "$sdk/bin/sign_update"
lipo "$sdk/Sparkle.framework/Versions/B/Sparkle" -verify_arch arm64 x86_64
printf '%s\n' "$sdk"
