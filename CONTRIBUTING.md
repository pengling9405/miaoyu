# 妙语贡献指南

非常感谢你有兴趣为 **妙语 (Miaoyu)** 贡献代码或想法！下面梳理了参与项目的常见方式、开发环境准备以及提交规范，帮助你快速、高效地参与协作。

---

## 🤝 如何参与

- **反馈问题**：在 GitHub Issues 中描述复现步骤、期望行为、操作系统及版本信息。
- **提出建议**：欢迎分享新的功能点、交互优化或模型选择上的想法。
- **贡献代码**：请先在 Issue 中简单同步意图或认领任务，再提交 PR。
- **完善文档**：README、指南、脚本注释等文字改进同样宝贵。

---

## 🛠️ 开发环境要求

- [Bun](https://bun.sh/) 1.1+
- Node.js 18+（用于类型检查，Bun 会在脚本中调用）
- Rust stable toolchain（通过 `rustup` 安装）
- `cargo` 及目标平台需要的构建依赖（macOS 需安装 `cmake`/Command Line Tools，Windows 推荐使用 MSVC）

### 模型文件

桌面端依赖本地 ONNX 模型，请在开始开发前准备：

```
src-tauri/models/
├── asr/model.int8.onnx
├── asr/tokens.txt
├── punc/model.onnx
├── punc/tokens.json
└── vad/silero_vad.onnx
```

参考 README 中的脚本快速下载。缺失模型会导致录音流程报错。

---

## 🚀 本地运行

```bash
# 安装前端依赖
bun install

# 启动开发模式（Tauri + 前端）
bun run tauri dev
```

若需要构建安装包：

```bash
bun run tauri:prod build
```

---

## 🧹 代码规范与质量

在提交 PR 前，请确保通过以下检查：

```bash
bun run lint            # 前端代码质量检查
cargo fmt               # Rust 代码格式化
cargo check             # Rust 静态检查
```

如新增或修改命令/脚本，请同步更新文档。

### 提交信息

- 遵循简要明确的 commit message，例如 `feat(audio): 支持本地 VAD`。
- 多人协作建议使用特性分支（如 `feat/local-asr`、`fix/window-resize`）。
- PR 描述请包含变更内容、测试情况，以及是否存在待办事项。

---

## 🔐 许可证

妙语项目基于 [Apache License 2.0](LICENSE)。提交贡献即表示你同意作品以该许可证发布。

---

欢迎加入妙语社区！如果有任何疑问，可通过 Issue 与我们联系，共同打造更好用的中文语音工作流工具。谢谢！ 😊
