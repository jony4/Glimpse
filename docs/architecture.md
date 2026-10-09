# 架构约定

## 目标与边界

Glimpse 是只读桌面应用，核心场景为 Markdown、代码和 Git diff。当前阶段建立可构建、可启动的基础及文件阅读闭环，后续以小步功能迭代推进。

采用三个 crate，避免 UI 类型进入领域模型，也避免为了尚不存在的能力建立空模块或通用框架。

```text
glimpse-app ──────> glimpse-services ──────> glimpse-core
     └─────────────────────────────────> glimpse-core
```

- `glimpse-core`：可独立测试的领域值与纯逻辑。只在这里存在真正共享的概念时扩充模型，不引入 GPUI、窗口、文件读取或进程启动。
- `glimpse-services`：具体 I/O 实现。返回领域值和可追踪的错误；不依赖应用层，不决定界面布局。现阶段直接函数足够，存在替换实现的需求时再引入 trait。
- `glimpse-app`：组合服务与 GPUI；负责窗口、动作、焦点、状态、任务生命周期和呈现。`main.rs` 只调用启动入口。

## 状态与任务

`Workspace` 是窗口级状态所有者，持有当前 `Reader`、加载状态、错误和读取任务。`Reader` 持有长期存在的 `EditorState`，不在 `render` 内重建编辑器或执行文件 I/O。Markdown 使用 GPUI Kit 的 TextView。

文件读取在 GPUI 的 background executor 上执行，完成后回到窗口上下文安装数据。窗口实体使用弱引用，窗口关闭后不会被任务继续持有；替换任务使旧结果失效。读取失败保留旧文档。当前只读路径不提供任何磁盘写入 API。

初始版本通过 2 MiB 实际读取上限限制内存增长，并明确拒绝 NUL 字节和非 UTF-8 文件。这个限制是早期保护，不是最终的大文件方案；目录扫描、超长行、大文件和 Markdown 解析耗时应另行测量。渲染线程上的组件初始化仍可能包含解析工作。

## 后续功能落点

- 文件树：`glimpse-services/src/workspace/` 负责目录扫描；`glimpse-app/src/views/explorer/` 负责虚拟列表、展开状态和选择。按需加载子目录。
- Git diff：`glimpse-core/src/diff/` 定义差异模型；`glimpse-services/src/git/` 封装只读 Git 调用；`glimpse-app/src/views/diff/` 实现 unified / split 呈现。
- 文件监听：服务层产出文件变化事件；应用层去抖并更新状态，保留阅读位置。
- 配置与会话：有实际持久化需求时新增具体服务，不把磁盘操作放到视图中。

以上路径是演进约定，目前不创建占位模块。

Git 层需明确区分工作区、暂存区和提交比较，使用参数化进程调用与路径分隔符，不拼接 shell 命令。差异数据与颜色、行高、滚动位置分别建模。

## 依赖和质量

GPUI Kit 使用精确版本并提交 Cargo.lock。升级时单独验证 API、启动、选择复制、中文、快捷键和窗口生命周期。暂不打开全部 Tree-sitter 语言、LSP 或 JS 扩展功能，按实际需求增加。

格式、Clippy、测试和构建由 macOS CI 执行。服务层测试覆盖文件保真与输入限制。图形界面仍需本机检查：欢迎页、⌘O 和取消、Markdown 预览/源码切换、文本选择复制、不可编辑、加载错误、窗口缩放和退出。CI 成功不等于这些交互已经验证。
