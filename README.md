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
![Release](https://img.shields.io/badge/release-v0.1.0-orange)
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
| 📴 **全离线语音识别** | 基于 `sherpa-rs` + `Paraformer` 的 ASR 引擎，使用 ONNX Runtime 直接在本地运行，无需网络。 |
| 🎯 **Silero VAD 精准检测** | 内置 Silero VAD 模型，自动裁剪静音段，按语句识别并生成时间戳。 |
| 📝 **智能标点补全** | 使用 ct-transformer 标点模型对识别结果补全标点和断句，输出更自然。 |
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

> 推荐执行 `bun run download-models` 一键下载并更新模型。脚本会跳过已存在的文件。
>
> 如果目录中缺失模型文件，应用会在启动录音时提示错误。确保已按上表放置模型。

### 快速下载脚本

```bash
cd src-tauri/models

# 语音活动检测
mkdir -p vad
curl -L -o vad/silero_vad.onnx \
  https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx

# Paraformer 中文 ASR（示例：小尺寸 int8 版本）
mkdir -p asr
curl -L -o /tmp/paraformer.tar.bz2 \
  https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-paraformer-zh-2024-03-09.tar.bz2
tar -xjf /tmp/paraformer.tar.bz2 -C asr --strip-components=1

# 标点模型
mkdir -p punc
curl -L -o /tmp/punc.tar.bz2 \
  https://github.com/k2-fsa/sherpa-onnx/releases/download/punctuation-models/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12.tar.bz2
tar -xjf /tmp/punc.tar.bz2 -C punc --strip-components=1
```

根据需要也可以换成你自己的 sherpa-onnx 模型，只要保证采样率为 16 kHz 并且文件名与代码匹配。

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

运行一键脚本或按需手动下载模型：

```bash
bun run download-models
```

下载完成后可执行一次检查：

```bash
bun run check-models
```

### 启动开发模式

```bash
bun run tauri dev
```

### 构建生产安装包

```bash
bun run tauri build
```

---

## 🧠 运行时流程

```plaintext
麦克风 → CPAL 录音线程
       → Silero VAD 分段检测
       → Paraformer ASR 离线识别
       → ct-transformer 标点补全
       → （可选）LLM 润色
       → 剪贴板/自动粘贴 → 目标应用
```

- 录音采样率自动降采样到 16 kHz，以兼容 ONNX 模型。
- VAD 采用 512 帧滑窗，尾部补 3 秒静音，确保检测到语音结束。
- 若 VAD 或 ASR 未识别到语音，会提示“未检测到语音，请检查麦克风并在录音时保持发声”。
- 全流程在本地运行，不上传任何语音或文本。

---

## 🧩 常见问题

| 问题 | 排查建议 |
|------|----------|
| 提示 “未找到 VAD 模型” | 确认 `src-tauri/models/vad/silero_vad.onnx` 是否存在且未被重命名。 |
| 识别为空或全是静音 | 检查外接麦克风音量，或在设置中关闭降噪软件。必要时可调低 `threshold`。 |
| 构建缓慢 | `sherpa-rs` 首次编译会下载并编译原生依赖，耐心等待即可。 |

更多调试日志可在 `src-tauri/tauri.conf.json` 中开启。

---

## 🛠️ 技术栈

- **前端**：React 19、Vite 7、Tailwind CSS 4
- **桌面容器**：Tauri 2、Rust 2021
- **音频录制**：cpal、rodio
- **本地语音识别**：sherpa-rs（Paraformer、Silero VAD、ct-transformer）
- **日志**：tracing、tracing-subscriber
- **打包**：Tauri Bundler（macOS / Windows / Linux）

---

## 🤝 贡献

欢迎提交 Issue 或 PR。
在提交之前请执行：

```bash
bun run lint
cargo fmt
cargo check
```

期待与你一起把妙语打造成更好用的中文语音工作流工具。
