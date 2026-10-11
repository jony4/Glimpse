#!/bin/bash
# Build and package a macOS release. Requires macOS, Rust and Python 3.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then
    echo 'DMG packaging requires macOS.' >&2
    exit 1
fi
release_arch="${1:-arm64}"
case "$release_arch" in
    arm64) release_target=aarch64-apple-darwin ;;
    x86_64) release_target=x86_64-apple-darwin ;;
    *) echo 'Usage: scripts/package-dmg.sh [arm64|x86_64]' >&2; exit 1 ;;
esac
release_bundle="$PWD/dist/release-$release_arch/Glim.app"
GLIM_BUNDLE_DIR="$release_bundle" scripts/bundle-macos.sh release "$release_target"
version=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$release_bundle/Contents/Info.plist")
cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; d=json.load(sys.stdin); v=sys.argv[1]; assert all(p["version"] == v for p in d["packages"]), "Bundle/Cargo version mismatch"' "$version"
[[ "$(lipo -archs "$release_bundle/Contents/MacOS/glim")" == "$release_arch" ]]
# Ad-hoc signing provides local integrity, not Developer ID trust or notarization.
codesign --force --sign - --timestamp=none "$release_bundle"
codesign --verify --deep --strict "$release_bundle"
plutil -lint "$release_bundle/Contents/Info.plist"
dmg_name="Glim-$version-macos-$release_arch.dmg"
python3 - "$release_bundle" "$PWD/dist/$dmg_name" "$version" "$release_arch" <<'PY'
import pathlib, shutil, subprocess, sys, tempfile
bundle, output, version, architecture = sys.argv[1:]
with tempfile.TemporaryDirectory(prefix='glim-dmg-') as directory:
    stage = pathlib.Path(directory)
    shutil.copytree(bundle, stage / 'Glim.app', symlinks=True)
    (stage / 'Applications').symlink_to('/Applications')
    (stage / 'Read Me.txt').write_text(
        f'Glim {version} — macOS {architecture}\n\n'
        'Drag Glim.app to Applications.\n'
        'The all-purpose viewer for the AI era.\n\n'
        'This release is ad-hoc signed, without Developer ID signing or notarization.\n'
        'macOS Gatekeeper may require explicit approval in System Settings > Privacy & Security.\n\n'
        'Source and issues: https://github.com/jony4/Glim\n', encoding='utf-8')
    subprocess.run(['hdiutil', 'create', '-ov', '-volname', f'Glim {version}',
                    '-srcfolder', directory, '-format', 'UDZO', '-imagekey', 'zlib-level=9', output], check=True)
PY
hdiutil verify "$PWD/dist/$dmg_name"
(cd dist && shasum -a 256 "$dmg_name" > "$dmg_name.sha256")
printf 'Release artifact: %s\n' "$PWD/dist/$dmg_name"
