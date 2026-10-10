# 格式与限制

主要格式见 [README](../README.md#支持的格式)。本文记录当前实现边界，不记录个人目录扫描或历史验证结果。

## 渲染方式

| 内容 | 当前行为 |
| --- | --- |
| 常见编程语言 | Tree-sitter 高亮，普通源码支持行号、折叠、搜索和 Minimap |
| Jinja / Nunjucks / Twig、Dockerfile、INI、dotenv、忽略规则、Nix | 可见行轻量着色，不提供完整跨行语义解析 |
| Vue、XML / plist、SCSS / LESS、Cython、CUDA / Metal、GLSL / OSL | 分别复用 HTML、CSS、Python、C++、C 高亮，不提供专用语义校验 |
| HTML / HTM | 工作台内静态排版预览与源码切换，不执行 JavaScript；不等同于完整浏览器 CSS 布局 |
| Markdown | 排版预览、表格、代码块、相对图片、本地链接及源码编辑 |
| JSON | 源码高亮与只读可折叠树；JSONC / JSONL 使用源码，Notebook 只看 JSON、不执行 |
| Office | DOC / DOCX、XLS / XLSX、PPT / PPTX、RTF，通过系统 Quick Look 只读预览；排版与可用性取决于系统预览器 |
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
| 图片 | 输入最多 32 MiB / 1,600 万像素；位图边长最多 8,192，SVG 预览最长 4,000px |
| safetensors | 头部最多 8 MiB；张量列表最多 2,000 项 / 约 512 KiB，元数据最多 100 项 |
| 二进制点文件 | 前 64 KiB |
| Git | 单次 stdout 最多 8 MiB；Graph 最多 1,000 提交、分支最多 1,000 引用、stash 最多 100 条 |

预算按打开的文件计算，多标签会累计内存占用。safetensors 只做头部和偏移范围检查，不等同于完整模型验证。SVG 不解析外部文件资源。非 UTF-8 文本和归档内容尚无专用预览。Office 不提供编辑、公式重算或宏执行；复杂排版、加密文档或缺少系统预览器时，可从预览窗口选择默认应用打开。文件夹播放只枚举直接子文件，最多 10,000 项，顺序采用自然文件名排序，随机播放将队列打乱；无法播放的条目跳过。PDF 和音视频通过系统框架按需加载，不读入 Rust 文本缓冲区。

## 格式维护工具

`cargo run -p glim-services --example audit_formats --locked -- /path/to/project` 可按格式抽样调用读取服务，仅输出计数，不输出文件路径与正文。此工具不替代 UI 验证，按需运行。
