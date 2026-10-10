<p align="center">
  <img src="assets/branding/glimpse.png" width="112" alt="Glimpse logo" />
</p>

# Glimpse

A lightweight viewer for code, diffs, and Markdown in the AI era.

**AI writes. You see.**

Glimpse 是使用 Rust + GPUI Kit 构建的 macOS 查看器（文件内容只读，支持显式 Git 暂存和提交）。打开项目目录，在文件和 Git 变更之间切换，专注阅读 AI 生成的代码与文档。

## 当前功能

- **文件夹与目录树**：原生文件夹选择器、按需加载子目录、虚拟列表、键盘导航和可调宽度侧栏。遵循 `.gitignore`，隐藏 `.git`，不递归跟随目录软链接。
- **代码阅读**：只读文本、行号、选择复制、⌘F 搜索和 Tree-sitter 高亮。支持 Rust、JavaScript/TypeScript/TSX、JSON、Python、Go、C/C++、Swift、HTML/CSS、Shell、TOML、YAML、Markdown 和 diff；其他文本按纯文本显示。
- **Git diff**：自动识别仓库、分支和 linked worktree。区分 Staged / Unstaged / Untracked / Conflict，支持多个仓库，Changes 提供树状/平铺模式；普通 diff 默认双栏对齐，可切换 Inline，以红绿背景标记增删。支持删除、重命名和二进制差异提示，提供 View file 返回工作区文件。
- **Markdown**：标题、列表、表格、带高亮的代码块、选择复制、预览/源码切换、相对路径图片和本地文件/目录/标题链接跳转。
- **多窗口**：默认最大化普通窗口；File → New Window / ⌘⇧N 新建窗口，每个窗口独立打开项目。
- **多标签与布局**：左侧 Files / Git 功能栏及可调宽度面板，右侧标签栏、文件路径和阅读区。同一文件重复打开会切回已有标签；源码与不同范围的 diff 独立保留阅读状态。Markdown 默认渲染，标签栏右端可切换源码。
- **Minimap**：正文右侧显示文档结构缩略图、当前可见范围，支持点击/拖动定位；垂直滚动条位于最右侧。
- **图片**：直接预览 PNG、JPEG、WebP、GIF（首帧）、BMP、TIFF、ICO、SVG 和含 PNG 表示的 ICNS；不支持或损坏的文件在内容区显示提示和 Issue 入口。
- **Git 操作**：Staged Changes / Changes 两组，支持文件、目录和全部暂存/移出暂存；消息框 ⌘Enter 只提交已暂存内容，保留 hooks 与签名。不提供克隆或丢弃文件修改。
- **自动刷新**：监听文件与 Git 元数据变更，合并事件后后台刷新，保留目录展开、标签模式和阅读位置；⌘R 可手动刷新。
- **导航**：顶部居中的前进/后退与工作区文件搜索；底部只显示当前文件格式。
- **文件图标**：Material Icon Theme 图标（MIT，归属与固定版本见 assets/file-icons）。
- **品牌资源**：原创 SVG、PNG 和 macOS ICNS 图标。

当前是早期可用版本。尚未实现提交历史比较、Mermaid/公式、Developer ID 签名与公证和自动更新。文本文件上限 2 MiB，图片上限 32 MiB / 1600 万像素（位图边长最多 8192），单次 Git 输出上限 8 MiB；不支持非 UTF-8 文本、PDF、音视频和压缩包内容预览。Combined conflict patch 使用 Inline。SVG 的外部文件资源不解析。Git 视图显示整个仓库的变更，即使打开的是其中的子目录。

## 下载

[下载 Glimpse 0.1.1 · macOS Apple Silicon DMG](https://github.com/jony4/Glimpse/releases/tag/v0.1.1)。打开 DMG，将 Glimpse 拖到 Applications。

当前版本使用 ad-hoc 签名，尚未进行 Developer ID 签名和公证；首次打开可能需要在系统设置 → 隐私与安全性中手动允许。发布页同时提供 SHA-256 校验文件。

## 运行

需要 macOS、Rust 1.99.0 和 Apple Command Line Tools（`xcode-select --install`）。Git 功能需要可用的 `git` 命令。

```sh
cargo run --locked
cargo run --locked -- /path/to/project
cargo run --locked -- README.md
```

| 操作 | 快捷键 |
| --- | --- |
| 新建独立窗口 | ⌘⇧N |
| 打开文件夹 | ⌘⇧O |
| 打开文件 | ⌘O |
| 刷新 | ⌘R |
| 文本内搜索（源码/diff 获得焦点时） | ⌘F |
| 目录树选择 / 展开 / 收起 / 打开 | ↑ ↓ / → / ← / Enter |
| 关闭当前标签（无标签时关闭窗口） / 退出 | ⌘W / ⌘Q |

## 构建与检查

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked
scripts/bundle-macos.sh debug
open dist/Glimpse.app
```

发布优化构建使用 `scripts/bundle-macos.sh release`；ARM DMG 使用 `scripts/package-dmg.sh`，详见 [发布说明](docs/releasing.md)。打包脚本需要 Python 3；开发打包输出 `.app`，DMG 脚本额外进行 ad-hoc 签名；Developer ID 签名和公证尚未配置。普通构建不需要 Node.js。

GPUI Kit 固定为 `0.7.1`，依赖由 `Cargo.lock` 锁定，使用运行时 Metal shader 编译。首次构建需要下载和编译依赖；开发构建体积不能代表发布版本体积。

## 架构

```text
crates/
  glimpse-core/src/           # 文档、语言映射、目录/Git 模型、diff 字节范围
  glimpse-services/src/       # 文件/图片读取、目录扫描、Git 命令和监听
  glimpse-app/src/
    app/                     # 启动、菜单、快捷键、内嵌资源
    views/
      workspace/             # 窗口状态、异步加载、整体布局
      explorer.rs            # 按需目录树
      changes.rs             # 多仓库、提交入口、变更列表
      changes/tree.rs        # 目录层级与可见行投影
      reader.rs              # Markdown / 代码 / 图片 / diff 阅读
      split_diff.rs          # 对齐双栏与同步滚动
      minimap.rs             # 文档缩略图和滚动定位
      welcome.rs             # 欢迎页
assets/branding/             # SVG 原稿与 PNG
assets/macos/                # ICNS 与 Info.plist
scripts/                     # 本地打包、可选图标生成
```

依赖方向为 `glimpse-app → glimpse-services → glimpse-core`，应用层也可直接引用 core。见 [架构说明](docs/architecture.md) 和 [验证记录](docs/verification.md)。

## License

[MIT](LICENSE). Third-party dependencies retain their respective licenses.
