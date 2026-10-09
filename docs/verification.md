# Verification

Verified on macOS on 2026-10-09 using Rust 1.99.0 and GPUI Kit 0.7.1.

## Automated checks

- `cargo fmt --all --check` passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- `cargo test --workspace --locked` passed: 11 tests (1 core, 10 services).
- `cargo build --locked` and `scripts/bundle-macos.sh debug` passed.
- Bundle property-list validation, shell syntax validation, and `git diff --check` passed.

Tests cover diff byte ranges; file limits and invalid input; ignored entries and directory symlinks; Git discovery and linked worktrees; staged, unstaged, untracked, deleted, renamed and binary files; literal pathspecs; and unresolved merge conflicts. Git tests use temporary repositories and check that conflict state remains unresolved.

The upstream `block 0.1.6` dependency produces a future Rust incompatibility notice; it does not fail the current checks.

## Native UI smoke checks

The debug app bundle was exercised with a local fixture repository:

- Welcome logo and native folder chooser.
- Automatic repository/branch detection and change counts.
- Directory expansion and opening source files.
- Rust highlighting, line numbers, read-only input, and Command-F search.
- Markdown headings, Chinese text, tables, highlighted code fences, relative images, and source/preview switching.
- Refresh picking up added directories and changed Git status.
- Separate staged and unstaged patches, with red deletions and green additions.
- Sidebar alignment and Command-Q exit.

Keyboard-only tree navigation and opening a non-Git folder were not separately exercised in the native UI. Non-Git discovery is covered by service tests.

## Scope and limits

This verifies a local unsigned debug build, not release performance, signing, notarization, or distribution. Diff viewing is unified and refresh is manual. Refresh rebuilds tree expansion and reading position. Multi-tab reading, split diffs, commit-history comparison, automatic file watching, and Markdown document-link navigation are not implemented.
