# Glimpse

A lightweight viewer for code, diffs, and Markdown in the AI era.

**AI writes. You see.**

面向 macOS 的轻量、只读文件查看器。使用 Rust + GPUI Kit，目标聚焦 Markdown、代码和 Git diff。

当前初始化版本支持原生窗口、文件选择器（⌘O）、命令行打开文件、Markdown 预览/源码切换，以及带行号的只读文本视图。文件读取在后台执行，失败时保留当前文档并显示错误。

尚未实现：目录树、多标签页、代码语法高亮、Git diff、文件变化监听、相对资源解析、Mermaid/公式、应用签名和自动更新。当前仅接受不超过 2 MiB 的 UTF-8 文本文件。

## 开发

- macOS，Rust 1.99.0（当前验证工具链）。
- Apple Command Line Tools：`xcode-select --install`。
- GPUI Kit 固定为 `0.7.1`，完整依赖由 `Cargo.lock` 锁定。
- 该版本启用了运行时 Metal shader 编译；首次启动需要初始化图形管线。

```sh
cargo run --locked
cargo run --locked -- README.md
cargo run --locked -- crates/glimpse-app/src/main.rs
```

应用支持 ⌘O 打开文件、⌘Q 退出。关闭最后一个窗口会退出进程。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
```

开发构建输出为 `target/debug/glimpse`，发布构建输出为 `target/release/glimpse`。使用下面的脚本可生成本地未签名的 `.app`：

```sh
scripts/bundle-macos.sh debug    # 开发构建
scripts/bundle-macos.sh release  # 发布优化构建
open dist/Glimpse.app
```

脚本需要 Python 3。分发所需的 Developer ID 签名和公证尚未配置。
首次编译需要下载和构建 GPUI 的依赖，开发构建体积不能代表最终应用体积。

## 目录

```text
crates/
  glimpse-core/                 # 文档等领域数据；不依赖 GPUI、文件系统或进程
    src/document.rs
  glimpse-services/             # 文件系统；未来扩展 Git、目录扫描和监听
    src/files.rs
  glimpse-app/                  # GPUI 桌面应用
    src/main.rs                # 极薄入口
    src/app/                   # 启动、窗口生命周期、动作与菜单
    src/views/                 # Workspace、Reader、欢迎页
      workspace.rs             # 界面状态和后台任务编排
      reader.rs                # 持久文本状态、预览/源码呈现
      welcome.rs
assets/macos/Info.plist        # 应用元数据
scripts/bundle-macos.sh        # 本地 .app 打包
.github/workflows/ci.yml       # macOS 格式、lint、测试和构建
docs/architecture.md          # 依赖边界与演进约定
```

详细设计见 [架构说明](docs/architecture.md)。

## License

[MIT](LICENSE). Third-party dependencies retain their respective licenses.
