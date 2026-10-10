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

Outputs for 0.1.1:

- `dist/release/Glimpse.app`
- `dist/Glimpse-0.1.1-macos-arm64.dmg`
- `dist/Glimpse-0.1.1-macos-arm64.dmg.sha256`

The release directory is separate from the development bundle. Builds do not restart an existing Glimpse instance. Normal packaging needs no Node.js and uses the checked-in artwork.

## Verify and publish

Mount the DMG read-only, verify the bundled executable is arm64 and that its signature is intact, confirm the Applications shortcut, and install and launch the verified bundle for a smoke check without renaming Glimpse. Verify the checksum from `dist`:

```sh
shasum -a 256 -c Glimpse-0.1.1-macos-arm64.dmg.sha256
```

Commit and push the source, create an annotated `v0.1.1` tag at that commit, and publish a GitHub Release with both artifacts. Include the tested capabilities, platform and signing status in the release notes.

## Signing

Version 0.1.1 is ad-hoc signed, without Developer ID signing or Apple notarization. This checks local bundle integrity but does not establish publisher trust. Gatekeeper can require explicit approval in System Settings → Privacy & Security. A Developer ID certificate and notarization credentials are needed for a trusted distribution pipeline; they are not included in the repository.
