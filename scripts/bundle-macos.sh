#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then
    echo 'The app bundle requires macOS.' >&2
    exit 1
fi
build_profile="${1:-release}"
build_target="${2:-}"
cargo_args=(build --locked)
case "$build_profile" in
    debug) ;;
    release) cargo_args+=(--release) ;;
    *) echo 'Usage: scripts/bundle-macos.sh [debug|release] [target-triple]' >&2; exit 1 ;;
esac
if [[ -n "$build_target" ]]; then
    cargo_args+=(--target "$build_target")
fi
cargo "${cargo_args[@]}"
# Respect Cargo target-dir overrides without assuming a checkout-local target directory.
target_dir="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
if [[ -n "$build_target" ]]; then
    target_dir="$target_dir/$build_target"
fi
bundle_dir="${GLIMPSE_BUNDLE_DIR:-$PWD/dist/Glimpse.app}"
mkdir -p "$bundle_dir/Contents/MacOS" "$bundle_dir/Contents/Resources"
cp "$target_dir/$build_profile/glimpse" "$bundle_dir/Contents/MacOS/glimpse"
cp assets/macos/Glimpse.icns "$bundle_dir/Contents/Resources/Glimpse.icns"
cp assets/macos/Info.plist "$bundle_dir/Contents/Info.plist"
printf 'Created %s\n' "$bundle_dir"
