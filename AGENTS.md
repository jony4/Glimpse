# Glim development

- Product scope: a macOS viewer for Markdown, source code, and Git diffs, with explicit staging/unstaging controls and a Git commit input (Command-Enter, staged contents only); no cloning. Ordinary UTF-8 files support basic notepad-style editing, automatic saving and explicit saving; diffs, images and unsupported files remain read-only.
- Keep crate dependencies one-way: glim-app -> glim-services -> glim-core;
  glim-app may also depend on glim-core directly.
- glim-core must not depend on GPUI or perform I/O. Services do not import UI.
- Keep main.rs thin. Put application startup/actions in app/ and presentation in views/.
- Never perform file reads, directory scans, or Git commands inside render methods.
  Schedule blocking work through the background executor and retain/cancel task handles.
- Keep GPUI entities alive across renders. Use theme tokens for UI colors.
- Keep editing minimal: typing, selection, undo/redo, automatic save and explicit save. Do not add LSP, plugins, speculative abstraction layers, or empty placeholder modules.
- Pin GPUI Kit; retain Cargo.lock. Enable extra features only when used.
- By default, only make the requested code changes. Do not automatically run tests, build/package, install, launch, or perform UI smoke tests; the user handles testing. Do those steps only when explicitly requested.
- When validation is requested, use cargo fmt --all --check, cargo clippy --workspace --all-targets --locked -- -D warnings, cargo test --workspace --locked, and cargo build --locked.
- When UI smoke testing is requested, use macOS when available and report unverified interactions.
- Document changes to module boundaries in docs/architecture.md.
- When explicitly asked to package Glim, also install the new build over the existing local Glim.app by default, preserving a recoverable backup, unless the user asks for packaging only. Verify the installed bundle and launch it.

- Keep documentation focused on current behavior, architecture and unfinished work. Put release notes in GitHub Releases; do not accumulate per-session operation logs, historical verification reports or personal filesystem inventories in the repository.
