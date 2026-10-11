# 构建与发布

目标平台：macOS Apple Silicon（`aarch64-apple-darwin`）、macOS Intel（`x86_64-apple-darwin`）、Windows x64（`x86_64-pc-windows-msvc`）。macOS 需要 Rust、Apple Command Line Tools（包含 Swift 编译器）和 Python 3；Git 功能使用本机 Git。

## 开发

```sh
cargo run --locked -- /path/to/project
```

按需验证，不将历史运行结果作为当前代码已通过的证据：

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked
```

## 打包

保持 Cargo workspace 版本与 `assets/macos/Info.plist` 的 `CFBundleShortVersionString` 一致，新发布递增 `CFBundleVersion`。

```sh
scripts/bundle-macos.sh release
scripts/package-dmg.sh arm64
scripts/package-dmg.sh x86_64
```

应用打包输出 `dist/Glim.app`；DMG 脚本输出 `dist/release-<arch>/Glim.app`、`dist/Glim-<version>-macos-<arch>.dmg` 及同名 `.sha256` 文件。脚本检查架构、版本与签名，并校验磁盘映像。`GLIM_BUNDLE_DIR` 可覆盖应用输出路径，兼容旧 `GLIMPSE_BUNDLE_DIR`。

`target/` 和 `dist/` 是忽略的构建缓存与产物，不提交到 Git。正常构建使用现有图标，无需 Node.js。

## 安装与发布

1. 校验 DMG 的 SHA-256、只读挂载内容、ARM64 架构与签名。
2. 正常退出旧应用，保留可恢复备份后安装至 `/Applications/Glim.app`，确认安装内容一致并启动。
3. 验证本次改动涉及的界面行为；无法覆盖的交互在 Release 说明中明确列出。
4. 提交最终源码、推送版本标签，向 [GitHub Releases](https://github.com/jony4/Glim/releases) 上传 DMG 与校验文件；确认发布状态及远端校验值。

发布说明保留在 GitHub Release，源码历史由 Git 保存。仓库文档只维护当前状态和未完成事项，不积累逐次操作或验证流水。

应用目前使用 ad-hoc 签名，尚无 Developer ID 签名与公证；首次启动放行步骤见 [README](../README.md#首次打开被-macos-拦截)。

## Windows 与持续集成

Windows 构建需要 Visual Studio C++ Build Tools 和 Windows SDK（rc.exe、fxc.exe）。在 Developer PowerShell 中执行 `scripts/package-windows.ps1`，输出 `dist/Glim-<version>-windows-x64.zip` 和校验文件。使用静态 CRT，应用包含 ICO 资源；正常使用需支持 DirectX 11 的图形环境。尚无 Authenticode 签名。

`.github/workflows/ci.yml` 在 Apple Silicon、Intel Mac 和 Windows 主机分别执行格式、Clippy、测试、构建；main 和手动运行同时生成各平台产物。发布前下载同一提交的三个 artifact，核对架构及校验文件，再上传同一 GitHub Release。CI 不自动创建公开 Release。

Windows 不编译 Swift 辅助程序；原生文档/媒体预览和文件类型管理保持 macOS 专属，平台支持范围见 README。
