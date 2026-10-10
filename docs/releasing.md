# macOS releases

The first distribution target is Apple Silicon (`aarch64-apple-darwin` / arm64).

## Build

On macOS, install Rust with the target's standard library, Apple Command Line Tools and Python 3. Keep the workspace package version and `CFBundleShortVersionString` in `assets/macos/Info.plist` equal; increase `CFBundleVersion` for subsequent builds.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked
scripts/package-dmg.sh
```

The script builds with the release profile and an explicit ARM target, validates the bundle version and architecture, applies an ad-hoc signature, and creates a compressed, verified DMG. It uses temporary staging, includes an Applications shortcut and installation notes, and writes a SHA-256 sidecar.

Outputs when packaging the current source (version 0.1.3):

- `dist/release/Glim.app`
- `dist/Glim-0.1.3-macos-arm64.dmg`
- `dist/Glim-0.1.3-macos-arm64.dmg.sha256`

The release directory is separate from the development bundle. Builds do not restart an existing Glim instance. Normal packaging needs no Node.js and uses the checked-in artwork.

The existing published 0.1.2 artifacts retain the former Glimpse name. These Glim filenames describe future local packaging, not an already published replacement. `GLIM_BUNDLE_DIR` overrides the development bundle path; the legacy `GLIMPSE_BUNDLE_DIR` remains a fallback. The bundle identifier is retained for macOS identity continuity.

## Verify and publish

Mount the DMG read-only, verify the bundled executable is arm64 and that its signature is intact, confirm the Applications shortcut, and install and launch the verified bundle for a smoke check without renaming Glim. Verify the checksum from `dist`:

```sh
shasum -a 256 -c Glim-0.1.3-macos-arm64.dmg.sha256
```

Commit and push the source, create an annotated `v0.1.3` tag at that commit, and publish a GitHub Release with both artifacts. Include the tested capabilities, platform and signing status in the release notes.

## Signing

Version 0.1.3 is ad-hoc signed, without Developer ID signing or Apple notarization. This checks local bundle integrity but does not establish publisher trust. Gatekeeper can require explicit approval in System Settings → Privacy & Security. A Developer ID certificate and notarization credentials are needed for a trusted distribution pipeline; they are not included in the repository.
