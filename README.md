<p align="center">
  <img src="assets/branding/glim.png" width="112" alt="Glim logo" />
</p>

# Glim

**The all-purpose viewer for the AI era.**

AI 时代的全能查看器。原生 macOS 应用，在一个轻巧的窗口里阅读代码、文档、图片、Git 变更和 AI 模型元数据。

## 支持的格式

| 类型 | 格式与文件 |
| --- | --- |
| 编程语言 | Rust、C / C++、Go、Swift、Java、C#、Kotlin、Zig、Python、Ruby、PHP、Lua、Scala、Elixir、Shell |
| Web 与模板 | JavaScript / JSX、TypeScript / TSX、HTML、CSS、Astro、Svelte、ERB、EJS、Jinja / Jinja2、Nunjucks、Twig；Vue 复用 HTML 高亮 |
| 数据与接口 | JSON / JSONC / JSONL、GeoJSON、TOML、YAML、SQL、GraphQL、Protobuf；Notebook 查看 JSON 源码 |
| 配置与构建 | Dockerfile、Containerfile、Makefile、CMake、Nix、INI、`.env`、`.npmrc`、`.editorconfig`、`.gitignore`、Gemfile 等 |
| 文档与文本 | HTML / Markdown 排版预览（HTML 为静态阅读，不执行脚本）；TXT、日志、CSV 和其他 UTF-8 文件按文本查看 |
| Apple 办公 | Keynote（KEY）、Pages、Numbers；原生只读预览，支持文件及文档包 |
| Office | Word（DOC / DOCX）、Excel（XLS / XLSX）、PowerPoint（PPT / PPTX）、RTF；macOS 原生只读预览 |
| 字体 | TTF、OTF、TTC、OTC、DFONT；字体信息、字形切换、自定义示例与多字号预览 |
| PDF | 原生窗口阅读、翻页、缩放和文字选择 |
| 音视频 | MP3、MP4、M4A、MOV、WAV、AAC 等；原生播放控件，文件夹顺序 / 随机队列 |
| 图片 | PNG、JPEG、WebP、GIF、BMP、TIFF、ICO、SVG、ICNS、EXR、HDR |
| Git | 工作区与暂存区 diff、提交补丁、`.diff` / `.patch` |
| AI 模型 | safetensors：张量、shape、精度、参数数量与元数据，只读头部，不加载权重 |

编程语言使用语法高亮；部分模板和配置采用轻量着色。GIF 显示首帧，EXR / HDR 转为 SDR 预览，ICNS 需包含 PNG 图像。音视频实际解码能力取决于 macOS 支持的编码；压缩包内容暂不支持。[完整格式与限制](docs/file-format-support.md)

## 核心功能

- **JSON 双视图**：源码语法高亮与可折叠树状视图随时切换；树状显示键、类型、数组下标和元素数量，源码保留编辑能力。标准 JSON 树状解析限 4 MiB / 5 万节点，JSONC / JSONL 使用源码查看。
- **专注阅读**：多标签、多窗口、文件搜索、行号、折叠、Minimap 与换行开关；HTML / Markdown 默认预览，可切换源码并分别记住阅读模式。
- **轻量编辑**：普通文本支持编辑、撤销 / 重做、自动保存与 ⌘S。
- **原生文档与媒体窗口**：点击文件打开阅读 / 播放窗口；文件夹右键选择顺序或随机播放，播放当前目录下的音视频，不递归子目录。关闭对应标签会结束播放。
- **多目录工作区**：按需展开目录、添加多个文件夹，Files / Git 面板随时收起。
- **Git 工作流**：双栏 / Inline diff、暂存与提交、分支管理、Fetch / Pull / Push / Sync、Stash，以及默认收起的提交 Graph。Pull 仅快进，不强推。
- **大文件按需读取**：普通文本编辑预算 8 MiB，大文件或超长行进入只读分页，每页约 256 KiB；完整加载先确认，上限 64 MiB / 100 万行。
- **模型轻量预览**：safetensors 仅解析有限头部，显示模型结构，不把大体积权重载入内存。

## 下载与安装

**[下载最新版 · macOS Apple Silicon](https://github.com/jony4/Glim/releases/latest)**

下载 Release 中的 `.dmg`，打开后将 **Glim.app** 拖入 **Applications（应用程序）**。从旧版 Glimpse 升级时，先退出旧应用，再使用 Glim。

### 设为默认打开方式

安装后，在 **Glim → Default File Types…** 中选择格式，点击 **Set Selected to Glim** 并确认。窗口显示当前默认应用，也可以用 **Restore Selected** 恢复此前保存的默认应用。macOS 按文件类型设置，同一类型的多个后缀可能一起生效。

### 首次打开被 macOS 拦截？

Glim 目前使用 ad-hoc 签名，尚未取得 **Apple Developer ID 签名与公证**。如果提示“无法验证开发者”或“Apple 无法检查是否包含恶意软件”，确认下载自上面的项目发布页后：

1. 在“应用程序”中尝试打开 Glim 一次。
2. 打开 **系统设置 → 隐私与安全性**，找到 Glim 的拦截提示，点击 **仍要打开**。
3. 在确认窗口点击 **打开**，按系统提示验证身份。之后可以正常启动。

这是 Apple 提供的单个应用放行方式，无需关闭系统安全保护。[Apple 官方说明](https://support.apple.com/zh-cn/102445)

发布页同时提供 `.sha256` 校验文件。如提示文件已损坏，请先重新下载并核对校验值。

---

[开发与构建](docs/releasing.md) · [架构](docs/architecture.md) · [待办](docs/TODO.md) · [MIT License](LICENSE)
