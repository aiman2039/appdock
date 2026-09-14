#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

python3 scripts/test-update-metadata.py
cargo fmt --all --check
cargo check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
cargo build --all-features --locked
