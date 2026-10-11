<p align="center">
  <img src="assets/branding/repository-banner.svg" alt="Glim — The all-purpose viewer for the AI era. Code, documents, data, and Git." width="100%" />
</p>

<p align="center">
  <a href="https://github.com/jony4/Glim/releases/latest"><img src="https://img.shields.io/github/v/release/jony4/Glim?style=flat-square&amp;color=268879" alt="Latest release" /></a>
  <a href="https://github.com/jony4/Glim/actions/workflows/ci.yml"><img src="https://github.com/jony4/Glim/actions/workflows/ci.yml/badge.svg" alt="Desktop builds" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT License" /></a>
</p>

<p align="center"><b>English</b> · <a href="README.zh-CN.md">简体中文</a></p>

Glim brings code, documents, data, and Git changes into one native desktop workspace. Read what your AI tools produce, inspect a model's metadata, or review a change before committing it.

**The all-purpose viewer for the AI era.** Free and open source. Built with Rust and GPUI.

## Download

| Your device | Glim 0.1.8 |
| --- | --- |
| Mac · Apple Silicon (M series) | [Download ARM64 DMG](https://github.com/jony4/Glim/releases/download/v0.1.8/Glim-0.1.8-macos-arm64.dmg) |
| Mac · Intel | [Download Intel DMG](https://github.com/jony4/Glim/releases/download/v0.1.8/Glim-0.1.8-macos-x86_64.dmg) |
| Windows · Intel / AMD 64-bit | [Download x64 ZIP](https://github.com/jony4/Glim/releases/download/v0.1.8/Glim-0.1.8-windows-x64.zip) |

[Latest release & checksums](https://github.com/jony4/Glim/releases/latest) · [All releases](https://github.com/jony4/Glim/releases)

**macOS:** open the DMG and drag Glim into Applications. The app is ad-hoc signed, **not yet Developer ID signed or notarized**. If macOS blocks it, try opening it once, then go to **System Settings → Privacy & Security → Open Anyway**. Only approve a download you trust. [Apple's instructions](https://support.apple.com/en-us/102445).

**Windows:** extract the ZIP and run `Glim.exe`. Install [Git for Windows](https://gitforwindows.org/) to use Git features. The executable is not yet Authenticode signed. Requires DirectX 11; Windows ARM is not included.

## What you can open

| Content | Formats and experience |
| --- | --- |
| Code & configuration | Rust, Python, Go, JavaScript/TypeScript, C/C++, Java, Swift, Ruby, and more. Syntax highlighting, folding, search, and Minimap. Dockerfile, `.env`, `.npmrc`, Jinja, and other everyday project files. |
| Documents & data | Markdown and static HTML preview; highlighted JSON source and expandable tree; YAML, TOML, SQL, GraphQL, and other UTF-8 text. |
| Images | PNG, JPEG, WebP, GIF (first frame), SVG, TIFF, BMP, ICO, ICNS, EXR, and HDR. |
| Git | Working tree and staged diffs, commit history, inline and side-by-side comparison. |
| AI model metadata | safetensors tensors, shapes, dtypes, parameter counts, and metadata — without loading model weights. |
| Native documents · macOS | PDF, Word, Excel, PowerPoint, Keynote, Pages, Numbers, and RTF previews. |
| Media, fonts & databases · macOS | Audio/video playback and folder queues; TTF/OTF and font collections; read-only SQLite tables, schema, and associated WAL files. |

**Platform scope:** Windows includes the main workspace: code/text editing, Markdown/HTML, JSON, images, safetensors metadata, and Git. PDF, Office/iWork, media, fonts, SQLite native previews, and default file-type management currently require macOS.

[Full format list and limits](docs/file-format-support.md) · Office rendering and media codecs depend on macOS. HTML preview is static and does not execute scripts. Archives and non-UTF-8 text are not yet supported.

## A few things that make it useful

- **Read, then edit.** Switch Markdown/HTML between preview and source. Edit ordinary text with undo/redo and autosave; reading-mode preferences are remembered.
- **Inspect JSON both ways.** Use a collapsible tree to explore structure, or highlighted source to edit it.
- **Keep Git close.** Stage, unstage, commit, manage branches, fetch, pull, push, and stash. Expand commits into changed files; choose inline or side-by-side diffs.
- **Open a workspace, not just a file.** Multiple folders, tabs, windows, and file search keep related material together.
- **Read large files in chunks.** Text beyond the editing budget opens in read-only pages; full loading asks first and has a bounded limit. Model inspection reads only the safetensors header.

## Get involved

Found a file Glim should open better? [Report a bug](https://github.com/jony4/Glim/issues/new?template=bug_report.yml), [request a format](https://github.com/jony4/Glim/issues/new?template=feature_request.yml), or [start a discussion](https://github.com/jony4/Glim/discussions). English and Chinese are welcome.

[Contributing](CONTRIBUTING.md) · [Build from source](docs/releasing.md) · [Roadmap](docs/TODO.md) · [Security](SECURITY.md) · [MIT License](LICENSE)
