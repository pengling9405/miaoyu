<p align="center">
  <p align="center">
    <img width="150" height="150" src="https://github.com/zhanyuilong/miaoyu/blob/main/src-tauri/icons/Square310x310Logo.png" alt="妙语 Logo">
  </p>
  <h1 align="center"><b>妙语</b></h1>
  <p align="center">
    智能语音输入，妙语亦可生花。
  </p>
</p>

<br/>

[![License](https://img.shields.io/badge/license-Apache_2.0-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-macOS%20|%20Windows%20|%20Linux-lightgrey)
![Release](https://img.shields.io/badge/release-v0.2.0-orange)
![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)

---

## 🪶 简介

**妙语** 智能语音输入，妙语亦可生花。
与传统云端语音服务不同，妙语默认在本地推理完成整个 ASR 流程，确保隐私、安全和低延迟以及智能AI 润色，让你的语音输入妙语生花。

---

## ✨ 核心能力

| 能力 | 说明 |
|------|------|
| 📴 **全离线语音识别** | 内置 Paraformer Small + sherpa-rs，在本地 CPU 上完成推理，无需网络。 |
| ⌨️ **跨应用输入** | 通过全局快捷键触发录音，识别结果自动写入剪贴板并粘贴到光标所在位置。 |
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
MODELSCOPE_ACCESS_TOKEN=your_api_key_here
```

### 离线模型下载与存放

- 安装包不包含语音模型；首次使用请在应用内「模型管理」页点击下载，模型会自动写入系统的应用数据目录（如 Windows 的 AppData、macOS 的 Application Support 等）。
- 开发模式同样使用系统数据目录缓存模型。
- 如下载失败，可将同名模型手动放入应用数据目录后重启应用。

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
| 为什么没有语音识别入口？ | 确认「模型管理」已下载离线模型；安装包不带模型，需联网下载一次，模型存放在系统应用数据目录。 |

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
