# 架构约定

Glimpse 是 macOS 查看器，负责 Markdown、代码、图片和 Git diff 阅读。文件内容只读，Git 暂存、提交和克隆通过明确操作触发。

```text
glimpse-app ──────> glimpse-services ──────> glimpse-core
     └──────────────────────────────────> glimpse-core
```

## 职责

- `glimpse-core`：文档、路径对应的语言、目录项、仓库和变更范围模型，以及 unified/combined patch 中变更行的 UTF-8 字节范围。无 GPUI 或 I/O 依赖。
- `glimpse-services`：`files` 读取有限大小的 UTF-8 文本；`workspace` 按层读取目录；`git` 识别仓库、读取状态和 patch，并提供暂存/取消暂存、提交及克隆；`media` 在后台有限解码图片、将 SVG 栅格化为 PNG；`watch` 管理原生文件监听。阻塞 API 只从后台任务调用。
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

通过 `Command` 参数调用，不使用 shell 字符串。强制字面 pathspec，禁用 pager、external diff、textconv、fsmonitor 和可选 index 锁；清理可能覆盖工作目录的 Git 环境变量。stdout 有 8 MiB 上限，stderr 独立读取；超限终止子进程并返回明确错误。UI 通过服务明确执行 stage / unstage / commit / clone。提交只包含 index，保留 hooks 和签名配置；不提供文件编辑、checkout 或冲突解决。

`core/split_diff` 将普通 unified patch 对齐为 Before/After 两列，保留原行号并补齐缺失行。`views/split_diff` 持有两侧编辑器和装饰，纵向滚动同步。普通 patch 默认双栏，Inline 保留原始 patch；combined conflict patch 回退 Inline。

## 生命周期

Workspace 持有 Reader 标签列表、当前标签索引、标签栏滚动句柄、Explorer / Changes 实体、事件订阅、文件选择器任务和当前加载任务。新的读取替换旧任务，旧结果不能覆盖新选择；后台结果通过弱实体引用返回窗口。正在运行的同步 I/O 可能继续完成，但其结果不再被安装。

Reader 持有长期存在的 EditorState，render 不新建编辑器、不做 I/O。文件和 diff 均在 state 与 renderer 两层启用只读。代码语言按扩展名映射，按需开启 Tree-sitter grammar，不接 LSP。Markdown 用 Kit 的 Base TextView（Component 初始化安装主题和代码块高亮），并把相对图片 URL 解析为本地路径。

`workspace/updates` 持有 native notify watcher，500 ms 合并事件，后台刷新目录缓存、仓库状态和所有打开标签；仅替换内容改变的 Reader，保留目录展开、阅读模式与滚动位置。单文件打开监听其父目录以覆盖原子替换，linked worktree 同时监听 Git 元数据目录。Git 写入期间延后自动刷新，完成后显式刷新。会话恢复和 Markdown 本地文档跳转尚未实现。

## 品牌与构建

Logo 的 SVG 源文件在 `assets/branding`，PNG 内嵌于二进制用于欢迎页，ICNS 用于 app bundle。`scripts/render-icons.mjs` 是可选的图标生成工具；正常构建只使用版本控制内的资源，不需要 Node.js。

GPUI Kit 固定版本并提交 Cargo.lock。升级需验证启动、文本选择、中文、快捷键和窗口生命周期。macOS CI 执行格式、Clippy、测试、构建；实际界面验证记录见 verification.md。

## 工作台布局与标签

`workspace/render.rs` 组合固定宽度的活动栏、可调宽度的能力面板和右侧阅读区。Files / Git 切换只影响左侧面板。`workspace/tabs.rs` 管理标签栏、激活和关闭：文件路径与 diff 范围共同标识一个标签，重复打开复用 Reader；每个 Reader 保留自己的 EditorState、Markdown 模式和预览滚动句柄。Markdown 的内部 TextView ID 绑定编辑器实体，避免跨标签共享选择状态。关闭当前标签选择相邻标签，关闭其他标签保留当前文档，⌘W 优先关闭标签。

`views/minimap.rs` 缓存每行的文本宽度/缩进，按可见高度采样绘制文档轮廓；diff 增删行带颜色。源码使用 EditorState 的实际行高、可见行范围和滚动接口联动，Markdown 预览按渲染内容总高度映射滚动比例（缩略图表示源码结构，不是渲染页面的像素截图）。点击与拖动都能定位。GPUI Kit 的内置源码垂直滚动条位于文本内侧，Reader 遮盖该轨道，由 minimap 在最外侧绘制并驱动垂直轨道；横向滚动仍由编辑器管理。所有滚动与缩略图状态均属于单个 Reader。

## Source Control 树与导航

`views/changes/tree.rs` 先按 Conflict / Staged / Unstaged / Untracked 分组建立目录层级，再按每层目录在前、文件在后投影可见行。折叠键含 scope，避免同一路径在不同分组互相影响。目录/文件行使用固定行高、明确左对齐和统一缩进；长名称省略，状态和暂存按钮保留固定宽度。目录树与平铺列表共用同一虚拟列表。

`workspace/header` 管理历史和带 150 ms 防抖的工作区路径搜索；`workspace/clone` 协调 URL 输入、父目录选择和后台克隆。每个仓库保留独立提交草稿。

`Reader` 的图片数据来自 `services/media`，render 不读文件；PNG/JPEG/WebP/GIF/BMP/TIFF/ICO 解码后统一 PNG，GIF 当前只显示首帧。SVG 使用 resvg，不解析外部文件资源；ICNS 读取现代 PNG 表示。输入限制 32 MiB，渲染像素限制 1600 万；位图解码限制 8192 边长和 128 MiB 分配。图片快照指纹用于监听后的变化判断。错误留在 Reader 内容区，不变成整个工作台的文件读取错误横幅。

Material Icon Theme 文件图标固定上游版本，源 SVG、内嵌 PNG 和 MIT 归属在 `assets/file-icons`；活动栏使用原创细线 SVG。Dock 渲染使用略缩小的 SVG viewBox，将图案放大约 3.2%，不改欢迎页品牌尺寸。
