# 架构约定

Glimpse 是只读 macOS 查看器，负责 Markdown、代码和 Git diff 阅读。

```text
glimpse-app ──────> glimpse-services ──────> glimpse-core
     └──────────────────────────────────> glimpse-core
```

## 职责

- `glimpse-core`：文档、路径对应的语言、目录项、仓库和变更范围模型，以及 unified/combined patch 中变更行的 UTF-8 字节范围。无 GPUI 或 I/O 依赖。
- `glimpse-services`：`files` 读取有限大小的 UTF-8 文本；`workspace` 按层读取目录；`git` 识别仓库、读取状态和 patch。阻塞 API 只从后台任务调用。
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

通过 `Command` 参数调用，不使用 shell 字符串。强制字面 pathspec，禁用 pager、external diff、textconv、fsmonitor 和可选 index 锁；清理可能覆盖工作目录的 Git 环境变量。stdout 有 8 MiB 上限，stderr 独立读取；超限终止子进程并返回明确错误。UI 不执行 stage、commit、checkout 或其他仓库写入。

当前 diff 以只读文本提供原始 patch、红绿变更行、选择复制与搜索。代码差异模型和呈现分离，后续可在其上增加双栏对齐与行内差异。

## 生命周期

Workspace 持有当前 Reader、Explorer / Changes 实体、事件订阅、文件选择器任务和当前加载任务。新的读取替换旧任务，旧结果不能覆盖新选择；后台结果通过弱实体引用返回窗口。正在运行的同步 I/O 可能继续完成，但其结果不再被安装。

Reader 持有长期存在的 EditorState，render 不新建编辑器、不做 I/O。文件和 diff 均在 state 与 renderer 两层启用只读。代码语言按扩展名映射，按需开启 Tree-sitter grammar，不接 LSP。Markdown 用 Kit 的 Base TextView（Component 初始化安装主题和代码块高亮），并把相对图片 URL 解析为本地路径。

Refresh 在后台重新读取目录、仓库状态及当前内容；当前版本会重建展开状态和阅读位置。自动监听、会话恢复、多标签页和 Markdown 本地文档跳转留待后续迭代。

## 品牌与构建

Logo 的 SVG 源文件在 `assets/branding`，PNG 内嵌于二进制用于欢迎页，ICNS 用于 app bundle。`scripts/render-icons.mjs` 是可选的图标生成工具；正常构建只使用版本控制内的资源，不需要 Node.js。

GPUI Kit 固定版本并提交 Cargo.lock。升级需验证启动、文本选择、中文、快捷键和窗口生命周期。macOS CI 执行格式、Clippy、测试、构建；实际界面验证记录见 verification.md。
