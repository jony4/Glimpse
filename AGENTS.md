# Glimpse development

- Product scope: a macOS viewer for Markdown, source code, and Git diffs, with explicit Git staging and commit controls. File content remains read-only.
- Keep crate dependencies one-way: glimpse-app -> glimpse-services -> glimpse-core;
  glimpse-app may also depend on glimpse-core directly.
- glimpse-core must not depend on GPUI or perform I/O. Services do not import UI.
- Keep main.rs thin. Put application startup/actions in app/ and presentation in views/.
- Never perform file reads, directory scans, or Git commands inside render methods.
  Schedule blocking work through the background executor and retain/cancel task handles.
- Keep GPUI entities alive across renders. Use theme tokens for UI colors.
- Do not add editing, LSP, plugins, speculative abstraction layers, or empty placeholder modules.
- Pin GPUI Kit; retain Cargo.lock. Enable extra features only when used.
- Validate with cargo fmt --all --check, cargo clippy --workspace --all-targets --locked -- -D warnings,
  cargo test --workspace --locked, and cargo build --locked.
- For UI work, smoke-test on macOS when available and report any unverified interactions.
- Document changes to module boundaries in docs/architecture.md.
