#!/bin/bash
# Private key is delivered only over stdin, never through argv or a file.
set +x
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
assets="${1:?Usage: generate-update-feed.sh assets-dir release-tag}"
tag="${2:?Missing release tag}"
: "${SPARKLE_SDK:?Missing Sparkle SDK}"
: "${SPARKLE_PRIVATE_KEY:?Missing SPARKLE_PRIVATE_KEY secret}"
: "${SPARKLE_PUBLIC_ED_KEY:?Missing SPARKLE_PUBLIC_ED_KEY variable}"
: "${GITHUB_REPOSITORY:?Missing publishing repository}"
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Sparkle feed releases must use stable vMAJOR.MINOR.PATCH tags' >&2; exit 1; }
[[ "$GITHUB_REPOSITORY" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || exit 1
name="AppDock-${tag}-macos-universal.dmg"
archive="$assets/$name"
test -f "$archive"
feed_dir="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/appdock-feed.XXXXXX")"
trap 'rm -rf "$feed_dir"' EXIT
cp "$archive" "$feed_dir/$name"
prefix="https://github.com/$GITHUB_REPOSITORY/releases/download/$tag/"
printf '%s' "$SPARKLE_PRIVATE_KEY" | "$SPARKLE_SDK/bin/generate_appcast" \
  --ed-key-file - --maximum-deltas 0 --maximum-versions 1 \
  --download-url-prefix "$prefix" --link "https://github.com/$GITHUB_REPOSITORY" "$feed_dir"
feed="$feed_dir/appcast.xml"
signature="$(python3 "$repo_dir/scripts/validate_appcast.py" "$feed" "$archive" "${tag#v}" "$prefix$name")"
xcrun swift "$repo_dir/scripts/verify-update-signature.swift" "$SPARKLE_PUBLIC_ED_KEY" "$signature" "$archive"
printf '%s' "$SPARKLE_PRIVATE_KEY" | "$SPARKLE_SDK/bin/sign_update" --ed-key-file - --verify "$feed"
cp "$feed" "$assets/appcast.xml"
(cd "$assets" && shasum -a 256 appcast.xml > appcast.xml.sha256)
