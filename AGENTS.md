# Repository Guidelines

## 项目结构与模块组织
- `src/`：React 19 + Vite 前端代码，包含路由、设置界面与 UI 组件。
- `src-tauri/`：Tauri 2 Rust 后端，`src/audio/` 负责录音、Silero VAD、Paraformer ASR 与标点；`models/` 存放内置 ONNX 模型；`settings.rs` 管理持久化配置。
- `scripts/`：Bun 脚本（如 `download-models.ts`、`check-models.ts`）用于下载和校验模型文件。
- 根目录配置：`package.json`、`ci.yml`、`tauri.conf.json`、`src-tauri/Cargo.toml` 等。

## 构建、测试与开发命令
- `bun install`：安装前端依赖。
- `bun run download-models` → `bun run check-models`：一键下载并校验 ASR/VAD/标点模型。
- `bun run tauri dev`：启动 Vite + Tauri 联调环境。
- `bun tauri build --config src-tauri/tauri.prod.conf.json`：生成跨平台桌面安装包（模型随包分发）。
- `cargo fmt`、`cargo check`（在 `src-tauri/` 目录执行）：格式化并静态检查 Rust 代码。
- `bun run lint`、`bun run format`：Biome Lint/格式化前端代码。

## 代码风格与命名约定
- TypeScript/TSX：使用 Biome 默认（2 空格缩进、单引号），组件 PascalCase，工具函数 camelCase。
- Rust：遵循 `rustfmt` 输出，模块文件 snake_case，类型名 CamelCase。
- 模型与资源按功能划分目录：`models/asr`、`models/punc`、`models/vad`；发布新模型时同步更新 `scripts/download-models.ts` 中的整包版本信息与仓库 Release 资产。

## 测试指引
- Rust：新增逻辑时运行 `cargo check`，需要时补充模块单测并执行 `cargo test`。
- 前端：暂未启用自动化测试，提交前需确保 `bun run build` 通过并手动验证主要交互。
- 打包前务必再次执行 `bun run check-models`，避免缺失模型导致构建失败。

## Commit 与 Pull Request 规范
- 推荐使用语义化前缀：`feat:...`、`fix:...`、`chore:...` 等，例如 `feat(audio): 支持本地标点模型`。
- PR 描述需包含变更摘要、测试结果（列出执行过的命令），UI 改动附截图/录屏，并在必要时关联 Issue。
- 若涉及模型或配置调整，请同步更新脚本、文档与 `tauri.prod.conf.json` 的资源列表。

## 安全与配置提示
- 不要提交 `.env` 或 API Key；LLM 密钥通过设置界面输入。
- macOS 上调试录音需为运行 `bun run tauri dev` 的终端授予麦克风权限。
- 更新模型版本时需核对许可证，并在 `README`/`NOTICE` 中补充来源信息。
