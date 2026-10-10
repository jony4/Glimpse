# 文件格式适配记录（2026-10-10）

对 /Users/neo/dev 进行文件名统计，跳过 Git、依赖和缓存目录。
实文件检查按选定格式最多抽取 5 个样本，跳过隐藏目录和凭据文件；
只输出格式和通过数量，不输出文件内容。这不是对全部文件的穷举验证。

| 格式 | 目录内数量 | 本次支持 |
| --- | ---: | --- |
| Java | 1648 | 专用 Java 语法高亮 |
| SQL | 22 | 专用 SQL 语法高亮 |
| Vue | 619 | HTML 基础高亮，现有 JS/CSS 嵌入；不是完整 Vue/TS 语义支持 |
| XML / MaterialX / TMX | 82 / 100 / 1 | HTML 语法提供基础标签高亮，无 XML schema 校验 |
| GLSL / OSL | 216 / 173 | C 风格基础高亮，无着色器验证 |
| Metal / CUDA | 15 / 6 | C++ 基础高亮 |
| Cython pxd / pyx | 139 / 10 | Python 基础高亮，非完整 Cython 语法 |
| JSONL | 8 | JSON token 高亮，非数据表格 |
| LESS / Handlebars / OpenColorIO | 11 / 1 / 1 | CSS / HTML / YAML 基础高亮 |
| EXR / HDR | 31 / 3 | SDR 图片预览，不支持 HDR 色彩管理、曝光调整或多图层 |
| SVG | 453 | 修复大画布无法打开，预览按比例缩至最长 4000px |
| Makefile / .mk | — | Make 语法高亮 |
| .command 和常见 shell 配置名 | — | Bash 高亮 |

CSV、RST、INI、日志、properties、systemd、USDA 等 UTF-8 文本此前已能打开，
继续使用文本查看/编辑；CSV 尚无单元格视图，RST 尚无排版预览。
文本限制仍是 2 MiB，图片输入限制 32 MiB，位图保留原有像素和内存限制。

## 点开头的文件

不按隐藏文件名或缺少扩展名拒绝文件。UTF-8 文本正常查看和编辑。
无法作为文本或图片解码的点文件（例如 .DS_Store）使用只读字节预览：
显示偏移、十六进制和 ASCII，最多前 64 KiB，并明确标记截断。
这是原始字节查看，不是 .DS_Store 语义解析，也不支持二进制编辑。
超过文本大小限制的点文件同样降级到字节预览；权限不足、失效链接等仍会报错。
使用临时的 .env、.npmrc、.gitignore、.config.local 和二进制样本测试，
未读取用户真实凭据文件。

## 暂缓的专用预览

| 格式 | 数量 | 所需工作 |
| --- | ---: | --- |
| XLSX / XLSM | 8 / 1 | 工作簿解析、工作表切换、单元格表格和大表限制 |
| DOCX / PPTX | 3 / 3 | 页面/幻灯片排版、字体、嵌入媒体 |
| PDF | 4 | 分页渲染、缓存、缩放、文字选择和链接 |
| DWG | 3 | 专用 CAD 解码与图形浏览 |
| BLEND | 15 | Blender 场景解析或外部渲染流程 |
| ZIP / WHL / ZST | 1 / 2 / 54 | 有限制的归档浏览和解压流程 |
| 字体、编译库、可执行文件、编译着色器 | 多种 | 各自的元数据或二进制检查器 |

.dat 和数字后缀不代表固定格式，不盲目按扩展名转换。

## 验证与维护

实文件抽样共 19 类、81 个样本，通过现有服务入口全部成功读取/解码；
其中原先失败的两个 SVG 是画布尺寸超限，现已修复。
实文件抽样不等同于逐个 UI 渲染验证。

复现命令：
cargo run -p glimpse-services --example audit_formats --locked -- /path/to/project

GPUI Kit 仍固定 0.7.1，仅添加 Java/SQL/Make grammar 功能。
SQL grammar 的构建依赖要求 cc 1.2.x，Cargo.lock 中 cc 从 1.6.0
调整为 1.2.67，其余已有依赖版本保留。
