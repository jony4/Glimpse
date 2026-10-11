# 格式与限制

本文记录当前支持的格式与实现边界。返回 [English](../README.md) · [简体中文](../README.zh-CN.md)。

**平台说明：** PDF、Office/iWork、音视频、字体与 SQLite 原生预览目前仅 macOS 提供；其余工作台功能支持 macOS 和 Windows。

## 支持的格式

| 类型 | 格式与文件 |
| --- | --- |
| 编程语言 | Rust、C / C++、Go、Swift、Java、C#、Kotlin、Zig、Python、Ruby、PHP、Lua、Scala、Elixir、Shell |
| Web 与模板 | JavaScript / JSX、TypeScript / TSX、HTML、CSS、Astro、Svelte、ERB、EJS、Jinja / Jinja2、Nunjucks、Twig；Vue 复用 HTML 高亮 |
| SQLite | DB / SQLite / SQLite3、WAL / SHM 关联文件；表结构、定义 SQL、分页记录与二进制摘要，只读快照 |
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

编程语言使用语法高亮；部分模板和配置采用轻量着色。GIF 显示首帧，EXR / HDR 转为 SDR 预览，ICNS 需包含 PNG 图像。音视频实际解码能力取决于 macOS 支持的编码；压缩包内容暂不支持。



## 渲染方式

| 内容 | 当前行为 |
| --- | --- |
| 常见编程语言 | Tree-sitter 高亮，普通源码支持行号、折叠、搜索和 Minimap |
| Jinja / Nunjucks / Twig、Dockerfile、INI、dotenv、忽略规则、Nix | 可见行轻量着色，不提供完整跨行语义解析 |
| Vue、XML / plist、SCSS / LESS、Cython、CUDA / Metal、GLSL / OSL | 分别复用 HTML、CSS、Python、C++、C 高亮，不提供专用语义校验 |
| HTML / HTM | 工作台内静态排版预览与源码切换，不执行 JavaScript；不等同于完整浏览器 CSS 布局 |
| Markdown | 排版预览、表格、代码块、相对图片、本地链接及源码编辑 |
| JSON | 源码高亮与只读可折叠树；JSONC / JSONL 使用源码，Notebook 只看 JSON、不执行 |
| Apple 办公 | `.key`、`.pages`、`.numbers`，通过 Quick Look 预览，兼容目录形式的文档包；不提供编辑或幻灯片动画播放 |
| Office | DOC / DOCX、XLS / XLSX、PPT / PPTX、RTF，通过系统 Quick Look 只读预览；排版与可用性取决于系统预览器 |
| SQLite | 识别 SQLite 文件头及 DB / DB3 / SQLite / SQLite3；WAL / SHM / journal 通过同名主库打开，只剩 sidecar 时显示关联说明。表结构、定义 SQL、只读记录分页，支持视图 |
| 字体 | TTF / OTF / TTC / OTC / DFONT，CoreText 原生只读预览，集合内切换字体，自定义样例文字与多字号展示 |
| PDF | 原生 PDFKit 窗口，翻页、缩放、文字选择；加密文档可输入密码 |
| 音视频 | 原生 AVKit 窗口；MP3、MP4、M4A、M4V、MOV、AAC、WAV、AIF/AIFF、CAF、FLAC，具体编码取决于系统解码器 |
| 图片 | PNG、JPEG、WebP、GIF 首帧、BMP、TIFF、ICO、SVG、含 PNG 表示的 ICNS；EXR/HDR 转 SDR |
| safetensors | 仅读取头部，展示张量、shape、参数数量、精度和元数据；不加载权重或执行模型 |
| CSV、RST、日志等 UTF-8 文件 | 文本阅读与基础编辑，不提供表格或专用排版 |
| 二进制点文件（如 .DS_Store） | 有界十六进制/ASCII 预览，不提供语义解析或编辑 |

## 资源预算

| 模式 | 限制 |
| --- | --- |
| 普通文本编辑 | 8 MiB；超过 10 万换行符或单行超过 16 KiB 时进入分页 |
| 只读分页 | 约 256 KiB/页，补齐 UTF-8 字符；替换旧页，不提供编辑或全文搜索 |
| 完整只读加载 | 用户确认后最多 64 MiB / 100 万换行符 |
| JSON 树 | 4 MiB / 50,000 节点，serde_json 默认嵌套深度限制；键和值摘要最多 512 字符 |
| SQLite | 主库 + WAL + journal 临时快照合计最多 2 GiB；最多 1,000 张表/视图，200 行/页、64 列、单元格摘要 512 字节；查询限 5 秒，SQLite 字段/行长度限 8 MiB |
| 字体 | 输入最多 64 MiB，集合显示前 128 个字体，自定义样例最多 512 字符；不安装字体 |
| 图片 | 输入最多 32 MiB / 3,200 万像素，完整解码前检查像素数与 128 MiB 像素缓冲上限；位图边长最多 8,192，SVG 预览最长 4,000px |
| safetensors | 头部最多 8 MiB；张量列表最多 2,000 项 / 约 512 KiB，元数据最多 100 项 |
| 二进制点文件 | 前 64 KiB |
| Git | 单次 stdout 最多 8 MiB；Graph 最多 1,000 提交、分支最多 1,000 引用、stash 最多 100 条 |

预算按打开的文件计算，多标签会累计内存占用。safetensors 只做头部和偏移范围检查，不等同于完整模型验证。SVG 不解析外部文件资源。非 UTF-8 文本和归档内容尚无专用预览。Office 不提供编辑、公式重算或宏执行；复杂排版、加密文档或缺少系统预览器时，可从预览窗口选择默认应用打开。文件夹播放只枚举直接子文件，最多 10,000 项，顺序采用自然文件名排序，随机播放将队列打乱；无法播放的条目跳过。PDF 和音视频通过系统框架按需加载，不读入 Rust 文本缓冲区。

## 格式维护工具

`cargo run -p glim-services --example audit_formats --locked -- /path/to/project` 可按格式抽样调用读取服务，仅输出计数，不输出文件路径与正文。此工具不替代 UI 验证，按需运行。

`.key` 同时可能表示密钥文件：普通文本密钥仍按文本打开，仅 ZIP 容器或目录文档包进入 Keynote 预览。系统文件关联按后缀/类型工作，管理 `.key` 的默认应用时可能同时影响同后缀文件。

SQLite 预览复制主库、WAL 和 rollback journal，在临时副本中进行恢复与查询，SHM 由副本重建，不修改或 checkpoint 源文件。复制前后检查文件大小与时间；忙碌数据库可能需要暂停写入或提供稳定备份。预览不是实时连接，重新打开以刷新；不提供写 SQL，也不支持 SQLCipher 等加密库。
