<p align="center">
  <p align="center">
    <img width="150" height="150" src="https://github.com/zhanyuilong/miaoyu/blob/main/src-tauri/icons/Square310x310Logo.png" alt="妙语 Logo">
  </p>
  <h1 align="center"><b>妙语</b></h1>
  <p align="center">
    面向中文用户的离线智能语音工作流
  </p>
</p>

<br/>

[![License](https://img.shields.io/badge/license-Apache_2.0-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-macOS%20|%20Windows%20|%20Linux-lightgrey)
![Release](https://img.shields.io/badge/release-v0.2.0-orange)
![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)

---

## 🪶 简介

**妙语** 是一款专注中文语境的桌面语音输入工具。
它将语音录制、离线识别、智能标点和自动粘贴串成一条工作流，让你开口即可成文，并支持可选的 LLM 润色。

与传统云端语音服务不同，妙语默认在本地推理完成整个 ASR 流程，确保隐私、安全和低延迟。

---

## ✨ 核心能力

| 能力 | 说明 |
|------|------|
| 📴 **全离线语音识别** | 内置 Paraformer Small + sherpa-rs，在本地 CPU 上完成推理，无需网络。 |
| 🎯 **Silero VAD 精准检测** | Silero VAD 切分语音片段，自动过滤静音段，避免冗余文本。 |
| 📝 **智能标点补全** | ct-transformer 标点模型让文本更易读（敬请期待下一版本接入）。 |
| 🪄 **可选 LLM 润色** | 可接入任何兼容 OpenAI API 的模型（如 DeepSeek、qwen、Kimi 等），用于对识别文本做风格化润色。 |
| ⌨️ **跨应用输入** | 通过全局快捷键触发录音，识别结果自动写入剪贴板并粘贴到光标所在位置。 |

---

## 📦 模型与目录结构

所有模型按照功能分类存放在 `src-tauri/models` 下：

```
src-tauri/models/
├── asr/
│   ├── model.int8.onnx      # Paraformer ASR 模型
│   ├── tokens.txt           # ASR 词表
│   └── config.yaml          # 原始模型配置（可选）
├── vad/
│   └── silero_vad.onnx      # Silero VAD 模型
└── punc/
    ├── model.onnx           # ct-transformer 标点模型
    ├── tokens.json          # 标点词表
    └── config.yaml          # 原始模型配置（可选）
```

> 应用运行时会将模型缓存到 `AppData/models` 下。开发环境也可以提前将文件放入 `src-tauri/models`，首次启动会自动拷贝到缓存目录。

### 手动下载示例

```bash
MODEL_DIR=sherpa-onnx-paraformer-zh-small-2024-03-09

# Paraformer Small（16 kHz，int8）
mkdir -p src-tauri/models/$MODEL_DIR
curl -L -o /tmp/paraformer-small.tar.bz2 \
  https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-paraformer-zh-small-2024-03-09.tar.bz2
tar -xjf /tmp/paraformer-small.tar.bz2 -C src-tauri/models/$MODEL_DIR --strip-components=1

# 标点与 VAD（如需手动替换，可参考官方模型包）
```

> 推荐在「模型管理」页面直接点击“下载模型”，应用会自动将文件安装到 AppData 目录。

---

## 🚀 快速开始

### 环境要求

- [Bun](https://bun.sh) 1.1+
- Node.js 18+（用于类型检查）
- Rust 工具链（`rustup` 安装 stable 即可）
- macOS 10.15+ / Windows 10+ / 大多数主流 Linux 发行版

### 安装依赖

```bash
git clone https://github.com/your-org/miaoyu.git
cd miaoyu

bun install
```

### 配置可选的 LLM API（可跳过）

```bash
cd src-tauri
cp .env.example .env

# 编辑 .env，添加：
DEEPSEEK_API_KEY=your_api_key_here
```

### 准备模型

语音识别功能暂停期间，无需在 `src-tauri/models` 放置任何 ASR/VAD 模型文件。

### 启动开发模式

```bash
bun run tauri dev
```

### 构建生产安装包

```bash
bun tauri build --config src-tauri/tauri.prod.conf.json
```

---

## 🧠 运行时流程

```plaintext
麦克风 → CPAL 录音线程
       → Paraformer Small ASR（sherpa-rs）
       → （可选）LLM 润色
       → 剪贴板/自动粘贴 → 目标应用
```

---

## 🧩 常见问题

| 问题 | 排查建议 |
|------|----------|
| 为什么没有语音识别入口？ | 语音模块正在重构，当前版本仅可设置文本模型。后续版本恢复后会在更新日志中说明。 |

更多调试日志可在 `src-tauri/tauri.conf.json` 中开启。

---

## 🛠️ 技术栈

- **前端**：React 19、Vite 7、Tailwind CSS 4
- **桌面容器**：Tauri 2、Rust 2021
- **语音模块**：正在重构，后续版本将重新启用本地录制与 ASR 能力
- **日志**：tracing、tracing-subscriber
- **打包**：Tauri Bundler（macOS / Windows / Linux）

---

## 💬 交流与社区 (Communication & Community)

「妙语」是一个由社区驱动的开源项目，我们相信开放的交流能激发最好的创意。你的每一个想法、每一次反馈都对项目至关重要。

我们诚挚地邀请你加入官方微信交流群，在这里你可以：

*   🚀 **获取一手资讯**：第一时间了解项目更新、新功能预告和开发路线图。
*   💬 **直接与开发者对话**：遇到安装难题？有绝妙的功能点子？在群里可以直接 @ 作者和核心贡献者。
*   💡 **分享与学习**：交流你的 AI 指令 (Prompt) 和自动化工作流，看看别人是怎么把「妙语」玩出花的。
*   🤝 **参与项目共建**：从一个想法的提出，到一次代码的提交 (Pull Request)，社区是你最好的起点。

<div align="center">

| 微信扫码，加入官方交流群 |
| :----------------------------------------------------------: |
| <img src="src-tauri/assets/wechat-community-qrcode.png" width="200" alt="" />
| <p style="font-size:12px; color: #888;">如果二维码过期或无法加入，请在 <a href="https://github.com/pengling9405/miaoyu/issues">Issues</a> 中提一个 Issue 提醒我们，谢谢！</p> |

</div>

## 🤝 贡献

欢迎提交 Issue 或 PR。
在提交之前请执行：

```bash
bun run lint
cargo fmt
cargo check
```

期待与你一起把妙语打造成更好用的中文语音工作流工具。
