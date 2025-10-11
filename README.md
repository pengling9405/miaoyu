# 妙语 - 智能语音输入桌面应用

基于 Tauri + React + TypeScript 的智能语音输入工具，支持语音识别和 AI 文本润色。

## 功能特性

- 🎤 **语音识别**: 基于火山引擎 ASR
- ✨ **智能润色**: 使用 DeepSeek AI 优化文本
- ⌨️ **Hands-Free 模式**: 快捷键 `Option + Space` 启动/停止
- 🎯 **自动粘贴**: 识别完成后自动输入到活跃应用

## 环境配置

### 开发环境配置

#### 1. 复制环境变量模板

```bash
cd src-tauri
cp .env.example .env
```

#### 2. 配置 API 密钥

编辑 `src-tauri/.env` 文件：

```env
# 火山引擎语音识别配置
# 获取地址: https://console.volcengine.com/speech/service/8
VOLCENGINE_APP_ID=your_app_id_here
VOLCENGINE_ACCESS_TOKEN=your_access_token_here

# DeepSeek AI 配置
# 获取地址: https://platform.deepseek.com/api_keys
DEEPSEEK_API_KEY=your_api_key_here
```

### 生产环境配置

✅ **重要**：`.env` 文件中的值会在**构建时编译进二进制文件**作为默认值！

#### 构建时行为

当运行 `bun tauri build` 时：
- ✅ `.env` 文件在**构建阶段**被读取
- ✅ 环境变量值被**编译进二进制文件**作为默认值
- 📦 用户安装后可以**直接使用**，无需配置
- 🔧 用户可以在设置界面**覆盖默认值**

**验证构建时是否包含环境变量**：
```bash
# 构建时会显示警告信息
bun tauri build
# 输出应该包含：
# warning: 已设置 VOLCENGINE_APP_ID
# warning: 已设置 VOLCENGINE_ACCESS_TOKEN
# warning: 已设置 DEEPSEEK_API_KEY
```

#### 用户体验

**默认情况（开箱即用）**：
- 用户安装 DMG → 直接使用 → 使用编译时的默认 API 密钥

**自定义配置（可选）**：
- 打开设置界面（`Cmd + ,`）
- 填入自己的 API 密钥
- 覆盖默认配置

#### GitHub Actions 自动发布

在 GitHub Secrets 中配置环境变量：

```yaml
# .github/workflows/release.yml
- name: Create .env file
  run: |
    cd src-tauri
    cat > .env << EOF
    VOLCENGINE_APP_ID=${{ secrets.VOLCENGINE_APP_ID }}
    VOLCENGINE_ACCESS_TOKEN=${{ secrets.VOLCENGINE_ACCESS_TOKEN }}
    DEEPSEEK_API_KEY=${{ secrets.DEEPSEEK_API_KEY }}
    EOF

- name: Build
  run: bun tauri build
```

在 GitHub 仓库设置中添加 Secrets：
- `VOLCENGINE_APP_ID`
- `VOLCENGINE_ACCESS_TOKEN`
- `DEEPSEEK_API_KEY`

### 配置优先级

```
用户设置 (UI) > 运行时环境变量 > 编译时默认值
```

1. **用户设置** - 应用内设置界面（用户自定义）
2. **运行时环境变量** - 系统环境变量（高级用户）
3. **编译时默认值** - `.env` 文件编译进二进制（开箱即用）

> 💡 **开发提示**：`.env` 文件的值会被编译进二进制文件。
> 🔒 **安全提示**：`.env` 已在 `.gitignore` 中，不会被提交到版本控制。
> ⚠️ **发布注意**：GitHub Actions 需要配置 Secrets 来提供默认 API 密钥。

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
