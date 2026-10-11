# Contributing to Glim

Thanks for helping make more files readable. English and Chinese are both welcome.

## Bugs, formats, and ideas

- Search [existing issues](https://github.com/jony4/Glim/issues) before opening a new one.
- For a bug, include the Glim version, OS/architecture, steps, and expected versus actual behavior. Attach a small, sanitized example if the file format matters.
- For a format request, describe the information you want to see and a representative public or synthetic file. Never upload private documents, credentials, or proprietary model weights.
- Use [Discussions](https://github.com/jony4/Glim/discussions) for questions and early ideas. See [SECURITY.md](SECURITY.md) for vulnerabilities.

## Code changes

Keep PRs focused. For a substantial feature, discuss the scope in an issue first. See [the roadmap](docs/TODO.md), [architecture](docs/architecture.md), and [build instructions](docs/releasing.md).

The dependency direction is `glim-app → glim-services → glim-core`. Core contains no I/O or GPUI; services contain no UI. Keep filesystem/Git work off rendering paths and retain background task handles. Prefer bounded reads and cancellation for large files.

Use the Rust version pinned in [CI](.github/workflows/ci.yml). Before submitting code changes, run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked
```

Describe the user-visible change and how you verified it. For UI changes, include a screenshot with private paths/content removed; say which platform you tested. Documentation-only changes need link/content checks, not a full build. macOS native preview features need macOS testing; do not imply Windows parity without implementation.

Contributions are provided under the project's [MIT license](LICENSE). Please follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## 中文说明

欢迎中文 Issue 和 PR。报告问题时请提供版本、系统、复现步骤与脱敏样例；新格式请说明希望看到的信息。大功能先讨论范围，小改动保持聚焦。代码提交前执行上面的检查，说明实测平台及尚未验证的交互；纯文档改动无需构建。
