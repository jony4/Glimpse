# Glim 0.1.3 发布验证

日期：2026-10-11。环境：macOS Apple Silicon，Homebrew Rust 1.99.0。

## 自动检查

- `cargo fmt --all --check` 通过。
- `cargo clippy --workspace --all-targets --locked -- -D warnings` 通过。修复轻量词法着色中的重复条件分支；依赖 `block 0.1.6` 仍有 Rust future-incompatibility 提示。
- `cargo test --workspace --locked`：53 项测试全部通过，包含文件原子替换监听。此前在受限沙箱中的监听失败在恢复正常权限后消失。
- `cargo build --locked` 和 ARM64 release 构建通过。
- 新增 4 项服务集成测试覆盖分支创建/改名/删除、陈旧请求及脏工作区保护、stash 恢复、Graph/提交详情；临时本地 bare remote 的 Publish/Fetch/Pull/Push；UTF-8 分页边界、完整加载限制；2 GiB 稀疏 safetensors 头部读取和越界拒绝。测试未操作用户真实远程仓库。

## 打包与安装

- workspace、Cargo.lock、Info.plist 版本为 0.1.3，CFBundleVersion 为 4。
- `scripts/package-dmg.sh` 成功生成 ARM64 DMG 与 SHA-256 文件；`hdiutil verify`、SHA-256、plist 和 ad-hoc 签名校验通过。
- 只读挂载 DMG，核对 `Glim.app` 版本、签名及 Applications 快捷方式；从挂载包安装至 `/Applications/Glim.app`，安装后签名校验通过。
- 旧版已正常退出并备份至本机 `~/Library/Application Support/Glim/Backups/20261011-051646/Glimpse.app`。
- 安装后的应用启动成功，实机截图确认品牌菜单、Markdown 排版与 PNG 图片在阅读区域居中显示。

## 验证边界

macOS 未授予当前执行端辅助访问权限，自动点击、拖动、键盘编辑、侧栏切换与 Graph 展开等交互未覆盖，不将其标记为已验证。Git 操作通过服务层临时仓库测试。应用尚无 Developer ID 签名与公证。
