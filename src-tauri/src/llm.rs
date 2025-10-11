use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Wry};

use crate::settings::SettingsStore;

const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/chat/completions";
const DEEPSEEK_MODEL: &str = "deepseek-chat";

/// 获取 DeepSeek API Key
/// 优先级：用户设置 > 运行时环境变量 > 编译时默认值
fn get_api_key(settings: &Option<SettingsStore>) -> Result<String> {
    settings
        .as_ref()
        .and_then(|s| s.llm_api_key.clone())
        .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
        .or_else(|| option_env!("DEEPSEEK_API_KEY").map(String::from))
        .ok_or_else(|| anyhow::anyhow!("未配置 DeepSeek API Key，请在设置中配置"))
}

// 默认系统提示词（可以暴露给用户自定义）
pub const DEFAULT_SYSTEM_PROMPT: &str =
    "你是一个专业的文字润色助手。请对用户提供的语音识别文本进行智能优化：
1. 修正语音识别可能出现的错误
2. 添加合适的标点符号
3. 优化语句使其更加通顺自然
4. 保持原意不变，不要添加或删除关键信息
5. 直接返回优化后的文本，不要添加任何解释或前缀";

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

pub struct LLMService;

impl LLMService {
    pub async fn polish_text(app: &AppHandle<Wry>, text: &str) -> Result<String> {
        // 如果文本为空，直接返回
        if text.trim().is_empty() {
            return Ok(text.to_string());
        }

        // 读取用户配置
        let settings = SettingsStore::get(app).ok().flatten();
        let api_key = get_api_key(&settings)?;
        let system_prompt = settings
            .as_ref()
            .and_then(|s| s.llm_system_prompt.as_deref())
            .unwrap_or(DEFAULT_SYSTEM_PROMPT);

        // 构建请求
        let request = ChatRequest {
            model: DEEPSEEK_MODEL.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: text.to_string(),
                },
            ],
            stream: false,
        };

        // 发送请求
        let client = reqwest::Client::new();
        let response = client
            .post(DEEPSEEK_API_URL)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("调用 DeepSeek API 失败")?;

        // 检查状态码
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                target = "miaoyu_llm",
                status = %status,
                error = %error_text,
                "DeepSeek API 返回错误"
            );
            anyhow::bail!("DeepSeek API 调用失败: {}", status);
        }

        // 解析响应
        let chat_response: ChatResponse = response
            .json()
            .await
            .context("解析 DeepSeek API 响应失败")?;

        // 提取润色后的文本
        let polished_text = chat_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_else(|| text.to_string());

        tracing::info!(
            target = "miaoyu_llm",
            original_length = text.len(),
            polished_length = polished_text.len(),
            "文本润色完成"
        );

        Ok(polished_text)
    }
}
