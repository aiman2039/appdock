#!/bin/bash
set -euo pipefail
arm="${1:?Usage: assemble-universal.sh arm64-binary intel-binary output}"
intel="${2:?Usage: assemble-universal.sh arm64-binary intel-binary output}"
output="${3:?Usage: assemble-universal.sh arm64-binary intel-binary output}"
# Refuse swapped, missing, or accidentally universal inputs.
test "$(lipo -archs "$arm")" = arm64
test "$(lipo -archs "$intel")" = x86_64
mkdir -p "$(dirname "$output")"
lipo -create "$arm" "$intel" -output "$output"
chmod +x "$output"
lipo "$output" -verify_arch arm64 x86_64
