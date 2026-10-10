<p align="center">
  <img src="assets/branding/glim.png" width="112" alt="Glim logo" />
</p>

# Glim

**AI 时代的全能查看器 · The all-purpose viewer for the AI era.**

**从一行配置，到一整个模型。打开文件，看清内容。**

Glim 是使用 Rust + GPUI Kit 构建的原生 macOS 查看器。代码、文档、配置、图片、Git 变更与 AI 模型元数据，在一个轻巧的工作台里阅读。普通文本支持基础编辑与自动保存；大文件按需读取，模型权重留在磁盘。

## 一个窗口，读懂更多文件

| 内容 | 支持格式 / 文件名 | 阅读体验 |
| --- | --- | --- |
| 系统与应用开发 | Rust、C、C++、Go、Swift、Java、C#、Kotlin、Zig | Tree-sitter 高亮、行号、折叠、搜索、Minimap |
| 脚本与后端 | Python、Ruby、PHP、Lua、Scala、Elixir、Bash / Shell | 语法高亮；识别 Gemfile、Rakefile、Podfile、Brewfile、`.zshrc` 等无扩展名文件 |
| Web | JavaScript / JSX、TypeScript / TSX、HTML、CSS、Astro、Svelte、ERB、EJS | 语法高亮与已有 grammar 的嵌入语言支持 |
| 模板 | Jinja / Jinja2 / J2、Nunjucks / NJK、Twig | 轻量词法着色：模板定界符、控制关键字、字符串、注释与基础 HTML 标签 |
| 数据与接口 | JSON / JSONC / JSONL、GeoJSON、TOML、YAML、GraphQL、Protobuf、SQL | 高亮与文本阅读；`.ipynb` 查看 JSON 源码，不执行 notebook |
| 构建与容器 | Dockerfile / Dockerfile.*、Containerfile、Makefile / .mk、CMakeLists.txt / .cmake | Docker 指令、变量、字符串着色；Make / CMake 使用语法树 |
| 开发环境 | `.npmrc`、`.yarnrc`、`.editorconfig`、`.gitconfig`、`.gitmodules`、`.env` / .env.*、INI / CFG / properties、systemd .service | 键、分节、变量、字符串、数字与注释着色；不展开环境变量 |
| 项目规则 | `.gitignore`、`.dockerignore`、`.npmignore`、`.gitattributes`、Nix | 忽略规则通配符 / Nix 常用关键字的轻量着色 |
| 兼容语法 | Vue、XML / XSD / XSL / plist、LESS / SCSS、Cython、CUDA / Metal、GLSL / OSL | 分别复用 HTML、CSS、Python、C++、C 的基础高亮，不宣称完整专用语义 |
| Markdown | .md / .markdown | 排版预览、表格、代码块、相对图片和本地链接；顶部切换原文 |
| Git | Staged / Unstaged / Untracked / Conflict、.diff / .patch | Git 双栏 / Inline 增删对照，显式暂存与提交；独立 patch 文件按源码高亮 |
| 图像 | PNG、JPEG、WebP、GIF、BMP、TIFF、ICO、SVG、ICNS、EXR、HDR | 原生图片预览；GIF 首帧、EXR/HDR 转 SDR、ICNS 需包含 PNG 表示 |
| AI 模型 | **safetensors** | **头部预览：文件体积、张量数量、参数总数、精度分布、模型元数据、张量名称 / shape / 存储字节数** |
| 其他文本 | TXT、日志、CSV、RST 和其他 UTF-8 文件 | 文本阅读；大文件只读分页。CSV 不提供表格、RST 不提供排版 |
| 二进制点文件 | .DS_Store 等 | 前 64 KiB 十六进制 / ASCII 只读检查 |

词法着色使用主题语义色，不依赖 LSP；常见编程语言使用独立 grammar。普通源码统一等宽字体与舒展行距。模板与配置文件的轻量着色不等同于语法校验或完整跨行解析。

### 大文件：先看到，再决定读多少

- 普通 UTF-8 文本的完整编辑预算由 **2 MiB 提升至 8 MiB**，支持自动保存与 ⌘S。
- 超过预算，超过 **10 万换行符**，或出现超过 **16 KiB 的单行**（例如压缩 JSON），自动进入只读大文件阅读器：首次约 **256 KiB**，点击 **Next page / Previous** 按需读取。旧页被替换，显示明确的字节范围；UTF-8 字符跨页时不会被截断。
- **Load all…** 在加载前提示内存成本；完整只读视图限 **64 MiB / 100 万换行符**。更大的文件继续分页。读取期间检测文件尺寸与修改时间变化，避免拼接不同版本。
- 大文件使用虚拟列表，仅绘制可见行；超长行分段显示并支持横向滚动，不构建完整语法树、编辑历史或 Minimap。此模式不提供编辑、全文搜索或编辑器式文本选择；普通文件保留这些能力。
- 内存预算按打开的文件计算，多标签会累计占用；这不是对任意数量标签的进程内存硬上限。

### 模型文件：看结构，不搬权重

safetensors 预览遵循 [官方文件格式](https://github.com/safetensors/safetensors#format)：只读取长度字段与 JSON 头部，不加载 tensor payload，不运行模型、不反序列化 pickle。即使权重很大，也只处理有界元数据。

头部预算 **8 MiB**；张量列表最多 **2,000 项 / 约 512 KiB**，元数据最多 **100 项**，长字段截断并注明。预览校验头部、维度整数和偏移范围，但不替代完整模型校验。后续可扩展 GGUF 元数据、ONNX 图结构、NPY/NPZ 张量摘要；这些目前尚未实现，也不把 `.pt` / `.pth` 当作可安全执行的预览输入。

## 当前功能

- **文件夹与目录树**：原生文件夹选择器、按需加载子目录、虚拟列表、键盘导航和可调宽度侧栏。遵循 `.gitignore`，隐藏 `.git`，不递归跟随目录软链接。
- **代码阅读与基础编辑**：文本输入、删除、选择复制、撤销/重做、自动保存（保留 ⌘S）、行号、⌘F 搜索和 Tree-sitter 高亮。支持 Rust、JavaScript/TypeScript/TSX、JSON、Python、Go、Java、SQL、Make、C/C++、Swift、HTML/CSS、Shell、TOML、YAML、Markdown 和 diff；其他文本按纯文本显示。
- **Git diff**：自动识别仓库、分支和 linked worktree。区分 Staged / Unstaged / Untracked / Conflict，支持多个仓库，Changes 提供树状/平铺模式；普通 diff 默认双栏对齐，可切换 Inline，以红绿背景标记增删。支持删除、重命名和二进制差异提示，提供 View file 返回工作区文件。
- **Markdown**：标题、列表、表格、带高亮的代码块、选择复制、预览/源码切换、相对路径图片和本地文件/目录/标题链接跳转。
- **多窗口**：默认最大化普通窗口；File → New Window / ⌘⇧N 新建窗口，每个窗口独立打开项目。
- **多标签与布局**：左侧 Files / Git 功能栏及可调宽度面板，右侧标签栏、文件路径和阅读区。同一文件重复打开会切回已有标签；源码与不同范围的 diff 独立保留阅读状态。Markdown 默认渲染，顶部右侧可切换源码。
- **Minimap**：正文右侧显示文档结构缩略图、当前可见范围，支持点击/拖动定位；垂直滚动条位于最右侧。
- **图片**：直接预览 PNG、JPEG、WebP、GIF（首帧）、BMP、TIFF、ICO、EXR/HDR（SDR 预览）、SVG 和含 PNG 表示的 ICNS；不支持或损坏的文件在内容区显示提示和 Issue 入口。
- **点文件**：UTF-8 文本正常查看和编辑；无法解码的点文件提供只读十六进制/ASCII 预览，最多前 64 KiB。格式覆盖和限制见 [适配清单](docs/file-format-support.md)。
- **Git 操作**：Staged Changes / Changes 两组，支持文件、目录和全部暂存/移出暂存；消息框 ⌘Enter 只提交已暂存内容，保留 hooks 与签名。不提供克隆或丢弃文件修改。
- **自动刷新**：监听文件与 Git 元数据变更，合并事件后后台刷新，保留目录展开、标签模式和阅读位置；⌘R 可手动刷新。
- **导航**：顶部居中的前进/后退与工作区文件搜索；移除底部状态栏，留出更多阅读空间。
- **文件图标**：Material Icon Theme 图标（MIT，归属与固定版本见 assets/file-icons）。
- **品牌资源**：原创 SVG、PNG 和 macOS ICNS 图标。

当前是早期可用版本。尚未实现历史版本逐文件双栏比较、Mermaid/公式、Developer ID 签名与公证和自动更新。文本加载预算见上方大文件说明；图片上限 32 MiB / 1600 万像素（位图边长最多 8192），单次 Git 输出上限 8 MiB；不支持非 UTF-8 文本、PDF、音视频和压缩包内容预览。Combined conflict patch 使用 Inline。SVG 的外部文件资源不解析。Git 视图显示整个仓库的变更，即使打开的是其中的子目录。

## Git：沿用熟悉的工作方式

操作名称与入口参考 [VS Code 的分支工作流](https://code.visualstudio.com/docs/sourcecontrol/branches-worktrees)。选择左侧 Git 仓库后，在 Changes 区域使用分支按钮与操作栏：

| 操作 | 使用方式 / 行为 |
| --- | --- |
| Checkout Branch | 点击当前分支，输入文字筛选本地与远程分支；选择远程分支后填写本地名称，创建 tracking branch |
| Create Branch / Create Branch From | 从当前 HEAD 创建并切换；右键分支可从该分支创建 |
| Rename Branch | 分支选择器内 Rename 重命名当前分支 |
| Delete Branch | 右键本地分支；确认后使用 `git branch -d`，不强删未合并或正在使用的分支 |
| Merge into Current Branch | 右键来源分支；确认后合并到当前分支。冲突留在 Changes 中处理，可使用 Abort Merge |
| Fetch | 获取已配置的全部远程；不改工作区 |
| Pull | 从当前 upstream 拉取，使用 **fast-forward only**；分叉时报错，不自动 rebase |
| Push | 推送当前 HEAD 到当前分支配置的 upstream；不 force push |
| Publish Branch | 无 upstream 时，按远程名选择发布入口；发布当前分支并建立 tracking 关系 |
| Sync Changes | 确认后依次 Pull → Push；若 Push 失败，已成功拉取的结果仍保留 |
| Stash / Apply Stash | 暂存 tracked + untracked 修改，忽略文件不包含在内；分支选择器内可恢复最近 100 个 stash，恢复后保留 stash |
| Commit / Stage / Unstage | 保留原有暂存、取消暂存与 ⌘Enter 提交；只提交已暂存内容，沿用 Git hooks 和签名 |

**Graph 位于 Changes 下方，默认收起。** 展开才读取历史，彩色连线展示分支与合并关系，显示提交标题、引用、作者、相对时间和短 SHA。可切换 All Branches / Current Branch；每次增加 200 个提交，最多 1,000 个。点击提交在主阅读区打开只读详情与补丁，右键复制完整 Commit ID；合并提交的补丁比较第一父提交。Graph 分隔线可拖动调整高度。

分支列表最多读取最近更新的 1,000 个引用。领先/落后数量依据本地 tracking refs，需 Fetch 才反映远端新变化。网络操作使用电脑已有 Git 凭据与 SSH 配置，不提供内置登录页面；错误显示在右下角，可刷新后重试。

修改工作区前需要保存 Glim 各窗口的草稿；切分支、拉取、合并、应用 stash 等要求磁盘工作区干净，可先 Commit 或 Stash。执行期间临时锁定各窗口文本编辑与其他 Git 写操作，完成后刷新并恢复编辑。外部终端/编辑器仍由 Git 自身与文件保存冲突检查保护，应用内锁不代替跨进程事务。

当前未提供 clone、远程配置增删、远程分支删除、force push、rebase、reset、cherry-pick、历史版本逐文件双栏对比或三路合并编辑器。冲突可在文本中编辑后暂存、提交；Abort Merge 会先提示可能丢弃解决冲突期间的编辑。

## 下载

[下载 Glim 0.1.3 · macOS Apple Silicon DMG](https://github.com/jony4/Glimpse/releases/tag/v0.1.3)。打开 DMG，将 `Glim.app` 拖到 Applications。Glim 原名 Glimpse，仓库地址保持不变；旧版换行偏好可继续读取。

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
| 保存当前文件 | ⌘S |
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
open dist/Glim.app
```

发布优化构建使用 `scripts/bundle-macos.sh release`；ARM DMG 使用 `scripts/package-dmg.sh`，详见 [发布说明](docs/releasing.md)。打包脚本需要 Python 3；开发打包输出 `.app`，DMG 脚本额外进行 ad-hoc 签名；Developer ID 签名和公证尚未配置。普通构建不需要 Node.js。

GPUI Kit 固定为 `0.7.1`，依赖由 `Cargo.lock` 锁定，使用运行时 Metal shader 编译。首次构建需要下载和编译依赖；开发构建体积不能代表发布版本体积。

## 架构

```text
crates/
  glim-core/src/           # 文档、语言映射、目录/Git 模型、diff 字节范围
  glim-services/src/       # 文件/图片读取、目录扫描、Git 命令和监听
  glim-app/src/
    app/                     # 启动、菜单、快捷键、内嵌资源
    views/
      workspace/             # 窗口状态、异步加载、整体布局
      explorer.rs            # 按需目录树
      changes.rs             # 多仓库、提交入口、变更列表
      changes/tree.rs        # 目录层级与可见行投影
      reader/                # Markdown / 代码 / 图片 / diff 阅读
      split_diff.rs          # 对齐双栏与同步滚动
      minimap.rs             # 文档缩略图和滚动定位
      welcome.rs             # 欢迎页
assets/branding/             # SVG 原稿与 PNG
assets/macos/                # ICNS 与 Info.plist
scripts/                     # 本地打包、可选图标生成
```

依赖方向为 `glim-app → glim-services → glim-core`，应用层也可直接引用 core。见 [架构说明](docs/architecture.md) 和 [验证记录](docs/verification.md)。

## License

[MIT](LICENSE). Third-party dependencies retain their respective licenses.

普通文本、代码和 Markdown 源码支持记事本式编辑。Markdown 点击 Source 图标编辑，切回 Preview 查看当前内容。修改后关闭按钮位置出现低对比小点，停止输入约 800 毫秒后自动保存；也可通过 ⌘S 或菜单 File → Save 立即保存。切换标签不影响待保存文件。关闭、切换项目及应用菜单退出会提示未保存内容；外部文件变化不会覆盖草稿，保存冲突会报错并保留编辑。当前仅编辑已打开的 UTF-8 文件，不提供新建空白文件、另存为。

### 阅读与工作区选项

- 在 Files 面板空白处右键选择 **Add Folder to Workspace…** 可继续添加多个目录，保留已打开文件；搜索和 Git 仓库列表覆盖所有目录。
- 再次点击左侧 Files / Git 图标可收起对应面板；Repositories 与 Changes 之间的分隔线可上下拖动。
- 顶部右侧提供换行开关，默认不换行，设置会记住；Markdown 的预览 / 原文按钮也位于此处，选中使用浅色背景。
- 新增 Ruby（含 Gemfile、Rakefile、Podfile）、PHP、C#、Kotlin、Lua、Scala、Elixir、Zig、GraphQL、Protobuf、CMake、Astro、Svelte、ERB、EJS 高亮，保留已有语言支持。
- `.DS_Store` 是 macOS Finder 二进制元数据；Git 无法生成逐行文本差异，界面会说明原因，可通过 View file 查看字节预览。
