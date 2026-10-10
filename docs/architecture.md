# 架构约定

Glimpse 是 macOS 查看器，负责 Markdown、代码、图片和 Git diff 阅读。普通 UTF-8 文件支持基础编辑和明确保存，Diff/图片仍只读；支持显式暂存/移出暂存，以及提交消息框的 ⌘Enter；提交仅包含 index，不提供克隆。

```text
glimpse-app ──────> glimpse-services ──────> glimpse-core
     └──────────────────────────────────> glimpse-core
```

## 职责

- `glimpse-core`：文档、路径对应的语言、目录项、仓库和变更范围模型，以及 unified/combined patch 中变更行的 UTF-8 字节范围。无 GPUI 或 I/O 依赖。
- `glimpse-services`：`files` 读取有限大小的 UTF-8 文本；`workspace` 按层读取目录；`git` 识别仓库、读取状态和 patch，并提供显式批量暂存/移出暂存与提交；`media` 在后台有限解码图片、将 SVG 栅格化为 PNG；`watch` 管理原生文件监听。阻塞 API 只从后台任务调用。
- `glimpse-app`：组合服务和 GPUI；`app` 管理启动、菜单、动作和内嵌资源；`views/workspace` 管理窗口状态、任务编排和整体布局；`Explorer`、`Changes`、`Reader` 分别管理三种呈现。

保留具体函数和明确的模块边界，在出现多个实现需求时再引入服务 trait。

## 目录树

只读取当前目录的直接子项，展开子目录时启动后台任务。缓存已经访问的目录；折叠不丢弃缓存。`ignore` 处理 ignore 规则，显示未被忽略的点文件，隐藏 `.git`，不递归跟随软链接。可见行由展开状态生成，使用 GPUI uniform_list 虚拟渲染。每个目录持有独立任务句柄，失败显示错误且可重试。

点击文件或键盘 Enter 发出 `ExplorerEvent::OpenFile`，窗口层读取文件。状态模型不包含 UI 颜色或 GPUI 类型。

## Git 比较语义

`git rev-parse --show-toplevel` 从任意子目录发现仓库，支持 `.git` 为文件的 linked worktree。非 Git 目录照常浏览；Git 不可用或读取失败时显示提示。

解析 `git status --porcelain=v1 -z`，保留空格、换行与原始路径字节。暂存和工作区修改是独立记录；冲突单独标识，重命名保留源路径。Changes 显示整个仓库的状态。

- Staged：`git diff --cached`，比较 HEAD 和 index；没有首个提交时也可读取。
- Unstaged：`git diff`，比较 index 和工作区。
- Untracked：`git diff --no-index /dev/null <file>`，接受退出码 1 表示差异。
- Conflict：`git diff --cc`，显示 Git 的 combined patch，不进行合并或解决冲突。

通过 `Command` 参数调用，不使用 shell 字符串。强制字面 pathspec，禁用 pager、external diff、textconv、fsmonitor 和可选 index 锁；清理可能覆盖工作目录的 Git 环境变量。stdout 有 8 MiB 上限，stderr 独立读取；超限终止子进程并返回明确错误。UI 通过明确的文件/目录/分组按钮更新 index，提交消息框执行 commit。提交只包含 index，保留 hooks 和签名配置；不提供 checkout 或冲突解决。

`core/split_diff` 将普通 unified patch 对齐为 Before/After 两列，保留原行号并补齐缺失行。`views/split_diff` 持有两侧编辑器和装饰，纵向滚动同步。普通 patch 默认双栏，Inline 仅展示 hunk 内容及增删标记；combined conflict patch 回退 Inline。

## 生命周期

Workspace 持有 Reader 标签列表、当前标签索引、标签栏滚动句柄、Explorer / Changes 实体、事件订阅、文件选择器任务和当前加载任务。新的读取替换旧任务，旧结果不能覆盖新选择；后台结果通过弱实体引用返回窗口。正在运行的同步 I/O 可能继续完成，但其结果不再被安装。

Reader 持有长期存在的 EditorState，render 不新建编辑器、不做 I/O。普通文件在 state 与 renderer 两层启用基础编辑；diff 两层均只读。代码语言按扩展名映射，按需开启 Tree-sitter grammar，不接 LSP。Markdown 用 Kit 的 Base TextView（Component 初始化安装主题和代码块高亮），并把相对图片 URL 解析为本地路径。

`workspace/updates` 持有 native notify watcher，500 ms 合并事件，后台刷新目录缓存、仓库状态和所有打开标签；仅替换内容改变的 Reader，保留目录展开、阅读模式与滚动位置。单文件打开监听其父目录以覆盖原子替换，linked worktree 同时监听 Git 元数据目录。Git 写入期间延后自动刷新，完成后显式刷新。会话恢复尚未实现。

## 品牌与构建

Logo 的 SVG 源文件在 `assets/branding`，PNG 内嵌于二进制用于欢迎页，ICNS 用于 app bundle。`scripts/render-icons.mjs` 是可选的图标生成工具；正常构建只使用版本控制内的资源，不需要 Node.js。

GPUI Kit 固定版本并提交 Cargo.lock。升级需验证启动、文本选择、中文、快捷键和窗口生命周期。macOS CI 执行格式、Clippy、测试、构建；实际界面验证记录见 verification.md。

## 工作台布局与标签

`workspace/render.rs` 组合固定宽度的活动栏、可调宽度的能力面板和右侧阅读区。Files / Git 切换只影响左侧面板。`workspace/tabs.rs` 管理标签栏、激活和关闭：文件路径与 diff 范围共同标识一个标签，重复打开复用 Reader；每个 Reader 保留自己的 EditorState、Markdown 模式和预览滚动句柄。Markdown 的内部 TextView ID 绑定编辑器实体，避免跨标签共享选择状态。关闭当前标签选择相邻标签，关闭其他标签保留当前文档，⌘W 优先关闭标签。

`views/minimap.rs` 缓存每行的文本宽度/缩进，按可见高度采样绘制文档轮廓；diff 增删行带颜色。源码使用 EditorState 的实际行高、可见行范围和滚动接口联动，Markdown 预览按渲染内容总高度映射滚动比例（缩略图表示源码结构，不是渲染页面的像素截图）。点击与拖动都能定位。GPUI Kit 的内置源码垂直滚动条位于文本内侧，Reader 遮盖该轨道，由 minimap 在最外侧绘制并驱动垂直轨道；横向滚动仍由编辑器管理。所有滚动与缩略图状态均属于单个 Reader。

## Source Control 树与导航

`views/changes/tree.rs` 仅投影为 Staged Changes / Changes 两组，再建立目录层级；Changes 合并 Worktree、Untracked、Conflict，原始 GitChange scope 不变，用于正确读取 patch，再按每层目录在前、文件在后投影可见行。折叠键含 scope，避免同一路径在不同分组互相影响。目录/文件行使用固定行高、明确左对齐和统一缩进；长名称省略，状态标记保留固定宽度。目录树与平铺列表共用同一虚拟列表。

`workspace/header` 管理历史和带 150 ms 防抖的工作区路径搜索，Clone UI 和服务已移除。每个仓库保留独立提交草稿。

`Reader` 的图片数据来自 `services/media`，render 不读文件；PNG/JPEG/WebP/GIF/BMP/TIFF/ICO 解码后统一 PNG，GIF 当前只显示首帧。SVG 使用 resvg，不解析外部文件资源；ICNS 读取现代 PNG 表示。输入限制 32 MiB，渲染像素限制 1600 万；位图解码限制 8192 边长和 128 MiB 分配。图片快照指纹用于监听后的变化判断。错误留在 Reader 内容区，不变成整个工作台的文件读取错误横幅。

Material Icon Theme 文件图标固定上游版本，源 SVG、内嵌 PNG 和 MIT 归属在 `assets/file-icons`；活动栏使用原创细线 SVG。Dock 渲染使用略缩小的 SVG viewBox，将图案放大约 5.8%，不改欢迎页品牌尺寸。

## 窗口和阅读呈现

`app::open_workspace` 统一创建最大化普通窗口，File → New Window / ⌘⇧N 创建独立 Workspace；各窗口的项目、标签、监听和任务互不共享。关闭最后一个窗口才退出应用。

`core::diff_content` 在纯函数中移除 patch 文件头和 hunk 头，并将高亮范围映射到显示文本；双栏同样隐藏元数据，保留真实行号，多个 hunk 用空行分隔。无文本差异和二进制变化显示简短说明。Reader 的 snapshot 始终保留原 patch 供刷新比较。

行号不再通过单独 Entity 延后绘制：SourceReader 使用编辑器原生 gutter，与文本、折叠和滚动使用同一布局帧。原生折叠栏也为行号和正文提供间距。活动栏选中指示不受 Button 悬停边框覆盖。

## 快速打开与紧凑侧栏

`services::workspace::open_folder` 只规范化路径并读取直接子项。Workspace 先安装 Explorer，再用独立 repository_task 发现仓库、读取 Git 状态和安装监听；新目录会取消旧句柄并校验返回根目录，文件阅读不再等待 Git。仓库发现跳过普通文件，同一仓库只 inspect 一次；Watcher 不再自行发现仓库，外部 worktree 元数据由已发现仓库列表补充。

仅 Files 保留根目录折叠状态；Git 不显示冗余根目录；Changes 另外保存 Repositories/Changes 折叠状态。仓库列表采用 26 px 行高和限高滚动区，选中行有主题背景与左边标记。提交消息框使用 ⌘Enter，普通 Enter 不提交；切换仓库保留独立草稿。搜索框有上下留白并禁用 focus border。Reader 遮盖内置垂直轨道时覆盖到底，避免底部出现第二个滚动条。

## 具体阅读器与共享编辑器

`views/reader/mod.rs` 的 Reader 仅持有标签身份、原文快照和 Renderer 枚举，按内容分派，不以多个可选字段拼凑互斥状态。具体实现为：

- `source.rs`：SourceReader 管理长期 EditorState、minimap 和差异装饰；代码与 JSON 按语言启用原生语法折叠，行号随同一布局帧绘制，不改原文件文本。
- `markdown.rs`：MarkdownReader 组合源码阅读器与持久 TextViewState、预览滚动和标题锚点。按当前文档 URL 解析相对/绝对本地链接及百分号编码；本地文件/目录通过 Workspace 的后台读取路径打开，HTTP(S)/mailto 交系统。`#标题` 与其他文档标题锚点通过解析后的 rendered range 定位，等待解析完成后再执行。
- `diff.rs`：DiffReader 组合 Inline SourceReader 和双栏 SplitDiff；仅在差异阅读器中持有切换状态。保留原 patch 快照以便刷新比较。
- `media.rs`：ImageReader 只保留解码图片；UnavailableReader 只保留错误与预填 Issue URL，不再创建无用 EditorState。Issue 草稿携带扩展名、版本、系统和架构，不自动附带文件正文、完整本地路径或可能含路径的错误详情，仍由用户提交。

所有内容读取和 Git 命令留在 Services 后台任务中；不引入插件/trait 注册表。新格式须先确定服务端加载数据和限制，再添加实际使用的 Renderer 实现。原生横向滚动条使用统一 Hover 模式和主题淡入淡出动画，文件/Inline/双栏共用；minimap 的垂直轨道保持独立。

## 搜索与目录上下文

搜索结果通过 deferred 绝对定位浮层覆盖在内容上，不参与页面高度计算；点击外部或 Escape 关闭。打开搜索结果会切换 Files 并取消过时搜索任务。Markdown 本地跳转复用相同后台打开流程，并把锚点随待加载文档关联，防止旧请求定位新文档。

Explorer 不设额外路径说明栏。目录展开时缓存每行的父行索引和子树结束位置；滚动时按当前布局偏移把已经离开顶部的祖先目录行绘制为吸顶行，与普通目录行复用缩进、箭头、层级竖线和点击逻辑。吸顶行覆盖原列表而不占用额外布局高度，切换到兄弟分支时旧祖先逐级退出；点击吸顶行可以收起并定位该目录。根目录只显示一次，仍可折叠。Changes 的目录/文件/组按钮选出相应 GitChange 集合，服务以字面路径批量执行，包含重命名旧路径，处理 unborn HEAD；“移出”只改变 index，绝不删除工作区文件。

目录行、按钮和右键菜单的控件身份绑定路径，事件回调按路径解析当前行，避免目录加载/刷新后列表序号变化时复用其他行的交互状态。图标槽固定 16px，加载省略号与展开箭头不改变文件名起点；点击吸顶目录定位时为父级吸顶行保留偏移，防止收起后上跳一行。

Issue 格式名优先使用非空扩展名；无扩展名时使用文件名本身（如 `.DS_Store`、`.env`、`README`），仍不携带父目录路径或文件内容。

正式包名始终为 Glimpse.app，bundle identifier 为 io.github.jony4.glimpse；不为测试修改正式包名。

## 稳定滚动和布局

双栏 diff 只转发当前接收用户输入一侧的纵向滚动。滚轮在 capture 阶段确定驱动侧，点击、拖动和键盘输入也会切换驱动侧。EditorState 在下一帧应用滚动位置；跟随侧的延迟通知、边界裁剪和初始化不再反向写回。横向位置独立保留，标题栏禁止压缩。

SourceReader 使用独立编辑器列和固定 110px 缩略图列，不再使用编辑器右内边距预留空间。左侧面板默认 320px，禁止自动伸展，仍可在 180–420px 范围拖拽调整。

连续同色 diff 行合并为高亮范围：双栏合并相邻范围，Inline 只跨单个换行合并，不跨上下文、颜色和 hunk 分隔。这样可减少长 diff 每帧扫描的装饰数量。

启动窗口直接使用屏幕 visible_bounds 创建普通窗口，不再对已铺满的初始框架调用会切换尺寸的 macOS zoom。搜索框和下拉浮层均为 380px，以完整窗口宽度居中；前进/后退按钮绝对定位在输入框左侧，不参与居中计算。标题栏取消默认的单侧 80px 内边距。

Dock 菜单复用 NewWindow 全局动作。Explorer 普通行、吸顶行、工作区根目录以及 Git 路径行共享路径菜单，提供 Finder 显示和相对/绝对路径复制；菜单操作绑定被点击行，不依赖正在阅读的文件。树行关闭按钮焦点描边，右键选择以行底色表示。

## Basic text editing

SourceReader retains an editable EditorState for ordinary text, with auto-closing pairs and smart indentation disabled. DiffReader and SplitDiff remain read-only. Input change events update the dirty flag against the saved baseline and refresh minimap data; rendering does not snapshot or write files. Markdown refreshes its preview and anchor map from the current buffer when entering Preview. Saving does not reset the editor, cursor or undo stack.

`workspace/editing.rs` owns save/confirmation task handles. Save captures the editor identity, disk baseline and current text, then performs I/O on the background executor. Success advances the baseline to the captured text, preserving any newer edits. Pre-save refresh work is canceled, and refresh results never replace dirty or saving tabs. Tab/window close, replacing a project and the app's Quit action offer Cancel/Discard when drafts exist. The application tracks weak workspace references to guard Quit across all windows.

`services/files::save_document` serializes local saves, resolves symlinks, checks the current content against the baseline, writes a same-directory temporary file, preserves permissions and syncs before replacement. It rechecks content before replacement and rejects read-only, deleted, externally modified or oversized files. Atomic replacement prevents truncation on failure; it does not provide a cross-process filesystem transaction, preserve hard-link identity or recover from force-quit. No new-document/Save As/session-draft recovery flow is added.

### Dock artwork
Dock-only artwork uses a brighter tile and 16% larger internal mark, not a larger
silhouette. Window chrome retains the original opaque, rectangular layout; no
experimental glass surfaces, rounded reader frames or additional panel insets remain.

## Format coverage and hidden-file inspection
Core owns filename-to-language routing. Java/SQL/Make use enabled toolkit grammars;
Vue/XML, shader and Cython aliases provide basic highlighting only. Services own
EXR/HDR-to-SDR decoding and bounded SVG rasterization. Rendering performs no I/O.
services/binary.rs adds bounded raw-byte previews for dotfiles that cannot use the
text/image readers. Workspace background loading routes these to Renderer::Bytes,
which retains a read-only SourceReader but exposes no save editor or dirty state.
It is separate from editable Source and read-only Diff despite sharing presentation.
See file-format-support.md for coverage, limits and deferred dedicated viewers.
