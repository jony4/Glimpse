# Verification

Verified on macOS on 2026-10-09 using Rust 1.99.0 and GPUI Kit 0.7.1.

## Automated checks

- `cargo fmt --all --check` passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- `cargo test --workspace --locked` passed: 18 tests (3 app, 2 core, 13 services).
- `cargo build --locked` via `scripts/bundle-macos.sh debug` passed.
- Bundle property-list validation, shell syntax validation and `git diff --check` passed.

Tests cover tab selection after closing; hierarchical Git tree ordering and scope-specific collapse; aligned diff rows and UTF-8 ranges; bounded file reads; ignored entries and symlinks; Git discovery and linked worktrees; staged, unstaged, untracked, deleted, renamed and binary files; literal pathspecs; unresolved conflicts; staging/unstaging and committing only staged contents; atomic file replacement watching; PNG/SVG/ICNS project assets and malformed/oversized media.

The upstream `block 0.1.6` dependency produces a future Rust incompatibility notice; current checks pass.

## Native UI smoke checks

A separate `Glimpse QA.app` was exercised against a temporary workspace containing two local repositories. Existing user app instances were left running during this smoke run.

- Empty Explorer: explanatory copy, Open Folder and URL/Clone Repository controls. Native folder chooser opens a workspace.
- Files/Git activity icons: fine strokes, increased spacing and left selection marker.
- File-type icons in Explorer, tabs and Changes. Header has centered history controls and file search; footer shows only the format at the right.
- Git tree: three-level nesting, folders before files, left-aligned directory names, consistent indentation and row heights. A long filename truncates while status/staging actions stay in their right columns. Collapsing a subtree keeps other directories and scopes visible.
- List mode removes directory rows; repository selection replaces the change list.
- Normal diff opens Side by side; scrolling the right pane brings both panes to the same original line. Inline switches to the original unified patch.
- Staging `notes.md` in the temporary repository enables commit. A UI commit creates the expected local commit; Git inspection confirms it includes only `notes.md`, leaving `src/main.py` modified and other files untracked.
- PNG displays fully within the content region. Markdown opens rendered by default with Chinese text. Unsupported-file content/Issue entry was checked in the earlier iteration before PNG support was added.
- Without manual refresh, externally adding and deleting a file updates Explorer. Externally changing an already open Markdown file updates its rendered content.
- Header search finds `logo.png` and reuses its existing tab. Back returns to the Markdown tab.

Earlier native checks also covered Rust highlighting, read-only input, Command-F, Markdown tables/code fences/relative images, tab closing and minimap clicking.

## Scope and limits

This verifies a local unsigned debug build, not release performance, signing, notarization or distribution. Clone execution over HTTPS/SSH, signing/hooks in real repositories, keyboard-only Changes navigation, minimap dragging and retained scroll positions after an external edit were not separately exercised in the native UI. SVG/ICNS decoding is covered by service tests; native image verification used PNG. GIF displays its first frame. Legacy ICNS without PNG representations is not supported.

Dock artwork is regenerated at approximately 3.2% larger scale; visual comparison against other running Dock apps was not performed. macOS may retain a cached icon until the app is reopened.

PDF, audio/video and archive contents, commit-history comparisons, Markdown document-link navigation and session restoration are not implemented. Combined conflict patches use Inline. Text is limited to 2 MiB, images to 32 MiB / 16 megapixels, and Git output to 8 MiB. The minimap is a sampled text silhouette, not a pixel-perfect page thumbnail; split diffs use the two editors' scrollbars.

## Release 0.1.0 packaging

The explicit `aarch64-apple-darwin` optimized build passed. `scripts/package-dmg.sh` produced `Glimpse-0.1.0-macos-arm64.dmg` and its SHA-256 sidecar, with an ad-hoc signed app, Applications shortcut and installation notes. Disk-image checksums and the SHA-256 sidecar were verified. The image was mounted read-only: bundle version 0.1.0, arm64-only architecture, strict signature verification, executable equality with the built bundle and the Applications shortcut all passed. Linked libraries are system libraries/frameworks.

A separate copy extracted from the DMG was launched with only its test bundle identity changed and re-signed. The optimized executable opened the temporary workspace and rendered its Chinese Markdown correctly. The existing Glimpse app was not restarted. Formatting, strict Clippy, all 18 tests and a default debug build also passed before publishing. This first release has no Developer ID certificate or Apple notarization.
