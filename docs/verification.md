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

## Window and reader refinements — 2026-10-10

All four required Cargo checks pass, including 20 tests. New parser tests cover hidden file/hunk headers, header-like content, Unicode decoration offsets, combined patches, multiple hunks and binary/mode-only changes.

A separate debug QA app confirmed native fullscreen startup; Command-Shift-N creates another window, and closing it leaves the original document/project window intact. Native checks also confirmed Files/Git switching, top-only tab selection, compact Markdown mode controls, source gutter spacing and matching line numbers after scrolling, and both split/inline diff content without patch metadata. The optional Code icon was added to the asset loader after the first visual pass.

Two different project folders loaded concurrently, multi-monitor fullscreen transitions, and keyboard-only activity-bar navigation were not separately exercised.

## Maximized reader and compact source control — 2026-10-10

Formatting, strict workspace Clippy, all 22 tests and debug build pass. The regenerated ICNS also passes the media decoding test. Folder-opening coverage verifies that first-level browsing works independently of invalid Git metadata; discovery coverage checks nested repositories without duplicates. Commit coverage verifies that only previously staged contents enter HEAD and newer worktree contents remain untouched.

A one-off debug service timing on `/Users/neo/dev` measured 6.19 ms for the six first-level entries and 951.08 ms for discovering/status-reading 12 repositories. This is a single local measurement, not a cold-start benchmark. Git discovery and watcher setup now run after Explorer installation; duplicate discovery in the watcher has been removed.

Native QA confirmed Files/Git root collapse independently, Repositories/Changes collapse, selected repository background and left marker, compact list rows, search focus without an outer ring and visible vertical spacing, removal of Clone/staging/commit buttons, and no stray inner scrollbar at the bottom of a long source file. Startup uses native window zoom (Maximized) instead of fullscreen. The Dock artwork is enlarged another 2.5% relative to the previous icon, using a 968-unit viewBox.

The new Command-Enter commit shortcut was not exercised against a live user repository; commit semantics are covered with disposable repositories in service tests. Multi-monitor placement and side-by-side visual Dock size comparisons remain unverified.

## Reader separation, navigation and index actions — 2026-10-10

Required format, strict Clippy, workspace tests (26) and debug build pass. Tests cover two-group projection without changing conflict/untracked semantics, whole-group collapse, batch stage/unstage in unborn and committed repositories, literal paths and renames, unchanged worktree contents, Markdown Unicode/duplicate-heading anchor mapping, and privacy-safe Issue draft metadata.

The installed release retained the Glimpse name and io.github.jony4.glimpse identifier. The obsolete Window QA bundle was moved to Trash. Native checks on a disposable repository confirmed directory staging, all-changes unstaging, single-file staging, compact Tree/List and split/inline icons, no Git root row, borderless commit input, and search results floating above the existing content without changing layout height.

Further interactive checks were interrupted by active user interaction with Glimpse; the automation explicitly refused control while the user was interacting. JSON folding, Markdown link clicks/cross-document anchors, search-result Files activation, deep-tree breadcrumb updates and horizontal scrollbar hover/drag are implemented but not all were verified end-to-end in the native UI in this pass. Native gutter/folding share the editor layout instead of a separate observer-driven gutter. Scrollbars use the pinned toolkit's Hover mode, including its enter/exit motion and drag state.

JSON folding applies to multiline objects/arrays; the viewer preserves source text rather than reformatting compact JSON. Markdown file/URL resolution uses standard URL semantics; heading anchors currently index ATX headings. Issue drafts include format/version/platform/architecture, not file contents or private paths.

## Long diff scrolling stability — 2026-10-10

All four required Cargo checks pass (30 tests), and the optimized arm64 bundle builds. Regression tests cover delayed follower notifications, follower clamping, either pane becoming the driver, restoration without reverse propagation, and decoration compaction preserving colors/context/hunk boundaries. A 10,000-line added block now has one highlight range rather than 10,000.

A native 12,000-line replacement diff was opened in the installed Glimpse build. Scrolling the right pane down and then the left pane up showed matching line numbers in both panes (1341 and 1257 respectively), with stable pane boundaries in the sampled screenshots. The test confirms the settled scroll positions, not frame-by-frame animation timing. Native source-file scrolling verification was interrupted by active user interaction; no claim is made that every intermittent source flicker has been reproduced.

The sidebar opts out of automatic growth and defaults to 220 logical pixels. Startup now uses the display's usable bounds before native maximization, avoiding proportional sidebar expansion from an intermediate 1100px window. Final startup geometry, trackpad inertia, scrollbar dragging and multi-display transitions were not separately rechecked in this pass.

## Natural folder sticky rows and dotfile Issue metadata — 2026-10-10

Formatting, strict Clippy, all 34 tests, debug build and optimized arm64 packaging pass. New tests cover nested ancestors, fractional sibling push-out, branch changes, collapsed/empty directories and .DS_Store/.env/README Issue metadata. Extensions remain the format for ordinary files; extensionless names are preserved without parent paths.

Native checks used a disposable four-level tree with 90 files. After scrolling, dev / yuanxu / devops / coredns remained as ordinary indented tree rows with disclosure arrows and vertical hierarchy guides. There was no separate path text, repeated root explanation or horizontal separator. Clicking the pinned coredns row collapsed its children and returned to the corresponding tree position. The signed installed Glimpse.app was verified against the packaged executable. Issue title/body generation was verified by tests; no GitHub issue was submitted.

## Source Control visual hierarchy — 2026-10-10

Formatting, strict workspace Clippy, all 34 tests, debug build and optimized arm64 packaging pass. The signed bundle was installed over Glimpse.app with a recoverable backup and matching executable hash, then launched with the existing project.

Repositories and the outer Changes section use 15px bold type and shared left-aligned outline chevrons. Repository rows have a repository icon, inset selected background and secondary branch text. Inner Staged Changes/Changes groups use normal-weight 14px text on a secondary surface, with a separate count badge at the right. Directory/file rows start farther inward, distinguishing section, group and tree levels. The commit input aligns with the main section label and has more separation from the groups.

Native visual review of the Git pane was interrupted by user interaction; the control tool rejected further clicks after the active window changed. Final Git-pane screenshot comparison, dark-mode appearance and long-label truncation were not completed in this pass.

## Window bounds, centered search and path menus — 2026-10-10

Required formatting, strict Clippy, all 34 tests, debug build and optimized arm64 build pass. The bundle remains Glimpse.app with its original identifier. Startup uses Windowed(visible_bounds) rather than toggling native zoom on an already full-size frame; sidebar default is now 280px.

Native checks confirmed matching 380px input/dropdown edges centered across the full window, full-sized ordinary window presentation, and the Explorer context menu with Reveal in Finder / Copy Relative Path / Copy Absolute Path. Copy Relative Path on docs was pasted into the search input and produced exactly docs. The search field was cleared afterward. The copied path replaced the clipboard during this check.

Dock menu registration reuses the existing global NewWindow action; requesting the Dock accessibility surface timed out, so its menu selection was not exercised. Finder launching, absolute-path clipboard contents, Git-row context-menu actions and multi-monitor placement were not separately exercised in this pass.

## Release 0.1.1 — Apple Silicon

The 0.1.1 workspace and bundle versions match (CFBundleVersion 2). Required fmt, strict Clippy, all 34 tests and debug build passed; the explicit aarch64-apple-darwin optimized build and DMG packaging succeeded.

The DMG was verified and mounted read-only. The embedded app is arm64-only, its ad-hoc signature passes strict validation, its executable matches the built release bundle, and its Applications shortcut points to /Applications. The SHA-256 sidecar validates successfully. The exact release bundle replaced the local installation with a recoverable backup; it launched under the unchanged Glimpse name and displayed the project Explorer. Existing native-test limitations above remain applicable.

DMG SHA-256: ec1ad562b5f3e18ae191a04abc8ad94b80a39dab356dc936b7117a5833929bfd

## Basic text editing (local development build)

Ordinary UTF-8 file/source views now accept typing, deletion, selection, undo/redo and explicit Save / Command-S. Markdown source editing feeds the preview when switching modes; all diff surfaces remain read-only. This is an edit-existing-files workflow, without New Document, Save As, autosave or session-draft recovery.

Required format, strict Clippy, workspace tests (40), debug build and optimized arm64 build pass. The toolkit's test-support feature is enabled only for development tests. Headless production-window tests drive actual editor keyboard input and Command-S, verify the file is untouched before Save, confirm undo remains available after Save, protect a dirty buffer from external refresh, reject conflicting saves, and cancel a close prompt without losing the buffer. Further tests verify preview text reflects unsaved Markdown and diff input remains unchanged. File-service tests cover concurrent saves, external changes/deletion, null-byte rejection, permissions and symlink preservation.

The local app was installed and launched without renaming it. Desktop keyboard testing was interrupted by active user interaction, so tests were completed in isolated headless windows without touching user documents. macOS IME composition, native close-button behavior, multi-window Quit dialogs and OS-level termination were not fully exercised. Quit via the app action and normal window/tab/project closures are guarded; forced/system termination has no draft-recovery guarantee. Atomic replacement preserves permission bits but not hard-link identity or extended attributes; it is not an interprocess compare-and-swap primitive.

## File-tree click stability

The pinned-directory click regression was reproduced in a headless production window: restoring the old zero-offset scroll behavior moved the clicked label from y=39px to y=7px. The fix preserves the ancestor-row offset and the label's visual slot. Regression coverage also checks label bounds through collapsed/loading/expanded states and 20 consecutive cached expand/collapse clicks.

Rows, buttons and their context menus now use path-based identities rather than changing visible-list indices. Click handlers resolve the current row by path. A fixed 16px icon slot keeps font-dependent loading/disclosure glyph widths out of filename positioning. Native-font differences are addressed by fixed geometry; the headless test font alone does not reproduce proportional-glyph width differences. These checks do not cover every real trackpad/double-click timing scenario.

### UI experiment rollback (2026-10-10)
The incomplete glass-style experiment was fully withdrawn: original 44px title bar,
search results styling, 52×48px rectangular activity controls, and edge-to-edge reader
layout restored. Window blur, translucent tints, added reader frame and panel insets
are absent. Default sidebar width remains 320px. Basic editing, file-tree jitter fixes
and the separately requested Dock artwork are preserved.
Formatting, strict Clippy, all 43 tests, debug and ARM release builds pass. Signed
bundle installed with recoverable backup and matching executable hash. Native macOS
launch screenshot confirms rectangular activity controls and reader, restored header
height and no extra reader frame/insets. Dark-mode interaction was not re-tested.

## Format coverage and dotfiles (2026-10-10)
49 tests pass, including grammar availability, extension aliases, EXR/HDR-to-PNG,
bounded large-SVG rasterization, hidden text routing and bounded byte previews.
A headless production reader test confirms byte previews reject keyboard edits,
expose no save editor and remain clean. All required formatting, strict workspace
Clippy, locked debug and ARM release builds pass.
The opt-in audit opened/decoded 81 representative files across 19 formats from
the requested directory; only aggregates were emitted, and hidden/credential
files were excluded. Dotfiles used synthetic fixtures instead.
The signed installed executable matches the release build; prior app is backed up.
Existing user windows were retained. Native desktop interactions for these new
formats remain unverified; relaunch is required to use the installed update.

## 0.1.2 release
Version 0.1.2 (bundle build 3) passes all four required checks and 49 tests.
The ARM-only DMG was verified and mounted read-only; its bundle version, identifier,
arm64 executable, strict ad-hoc signature and Applications shortcut were checked.
Its executable matches the optimized release bundle and the installed application.
The existing local app was backed up before replacement; active windows were preserved.
DMG SHA-256: 90b861d923f262e1f849cd30dc8dbc7c7706c3a23be5b8c3da905e80003cb4fd.
Native new-format interactions remain subject to the limitations above.
