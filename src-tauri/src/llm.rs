use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Wry};

use crate::{
    models::{self, LlmProviderConfig},
    settings::SettingsStore,
};

const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/v1/chat/completions";
const DEEPSEEK_MODEL: &str = "deepseek-chat";

struct LlmRuntimeConfig {
    api_key: String,
    api_url: String,
    model_name: String,
}

fn resolve_llm_runtime_config(
    app: &AppHandle<Wry>,
    model_override: Option<&str>,
    provider_override: Option<&str>,
    api_key_override: Option<String>,
) -> Result<LlmRuntimeConfig> {
    let entry = models::active_llm_entry(app, model_override, provider_override)
        .map_err(|e| anyhow!("读取文本模型配置失败: {e}"))?
        .ok_or_else(|| anyhow!("未配置文本模型，请先在“模型管理”中设置 API 密钥"))?;

    let config = models::supported_models();
    let model = config
        .llm_models
        .iter()
        .find(|model| model.id == entry.text_model_id)
        .ok_or_else(|| anyhow!("未知的文本模型: {}", entry.text_model_id))?;
    let provider = model
        .providers
        .iter()
        .find(|provider| provider.id == entry.provider)
        .or_else(|| model.providers.first())
        .ok_or_else(|| anyhow!("文本模型 {} 缺少提供商配置", model.id))?;

    let api_key = resolve_api_key(api_key_override, entry.api_key.clone(), provider)?;

    let api_url = provider
        .api_base_url
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(DEEPSEEK_API_URL)
        .to_string();

    let model_name = provider
        .model
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(DEEPSEEK_MODEL)
        .to_string();

    Ok(LlmRuntimeConfig {
        api_key,
        api_url,
        model_name,
    })
}

/// 获取 API Key
/// 优先级：参数覆盖 > 模型存储配置 > 运行时环境变量 > 编译时默认值
fn resolve_api_key(
    api_key_override: Option<String>,
    entry_value: Option<String>,
    provider: &LlmProviderConfig,
) -> Result<String> {
    if let Some(key) = api_key_override.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }) {
        return Ok(key);
    }

    if let Some(key) = entry_value.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }) {
        return Ok(key);
    }

    if let Some(env_var) = provider.api_key_env.as_deref() {
        if let Some(value) = resolve_env_api_key(env_var) {
            return Ok(value);
        }
    }

    if let Some(value) = resolve_env_api_key("DEEPSEEK_API_KEY") {
        return Ok(value);
    }

    Err(anyhow!("未配置文本模型 API Key，请在设置中配置"))
}

fn resolve_env_api_key(var: &str) -> Option<String> {
    let normalize = |value: String| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    };

    if let Ok(value) = std::env::var(var) {
        if let Some(result) = normalize(value) {
            return Some(result);
        }
    }

    match var {
        "DEEPSEEK_API_KEY" => option_env!("DEEPSEEK_API_KEY")
            .map(|value| value.to_string())
            .and_then(normalize),
        "MODELSCOPE_ACCESS_TOKEN" => option_env!("MODELSCOPE_ACCESS_TOKEN")
            .map(|value| value.to_string())
            .and_then(normalize),
        _ => None,
    }
}

pub fn has_configured_api_key(app: &AppHandle<Wry>) -> bool {
    if let Ok(Some(entry)) = models::active_llm_entry(app, None, None) {
        if entry
            .api_key
            .as_ref()
            .map(|key| !key.trim().is_empty())
            .unwrap_or(false)
        {
            return true;
        }

        if let Some(provider_env) = models::supported_models()
            .llm_models
            .iter()
            .find(|model| model.id == entry.text_model_id)
            .and_then(|model| {
                model
                    .providers
                    .iter()
                    .find(|provider| provider.id == entry.provider)
            })
            .and_then(|provider| provider.api_key_env.as_deref())
        {
            if resolve_env_api_key(provider_env).is_some() {
                return true;
            }
        }
    }

    resolve_env_api_key("DEEPSEEK_API_KEY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_thinking: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    #[serde(default)]
    total_tokens: Option<u32>,
}

#[derive(Debug)]
struct ChatResult {
    content: String,
    total_tokens: Option<u32>,
}

pub struct LLMService;

#[derive(Debug)]
pub struct PolishResult {
    pub text: String,
    pub total_tokens: Option<u32>,
}

impl LLMService {
    pub async fn polish_text(app: &AppHandle<Wry>, text: &str) -> Result<PolishResult> {
        // 如果文本为空，直接返回
        if text.trim().is_empty() {
            return Ok(PolishResult {
                text: text.to_string(),
                total_tokens: None,
            });
        }

        let active_entry = models::active_llm_entry(app, None, None)
            .map_err(|e| anyhow!("读取文本模型配置失败: {e}"))?
            .ok_or_else(|| anyhow!("未配置文本模型，请先在“模型管理”中设置 API 密钥"))?;
        models::check_llm_quota(app, &active_entry).map_err(|msg| anyhow!(msg))?;

        // 读取用户配置
        let settings = SettingsStore::get(app).ok().flatten();
        let runtime = resolve_llm_runtime_config(app, None, None, None)?;
        let system_prompt = settings
            .as_ref()
            .and_then(|s| s.llm_system_prompt.as_deref())
            .unwrap_or(DEFAULT_SYSTEM_PROMPT);

        let chat_result = Self::send_chat_request(
            &runtime.api_url,
            &runtime.api_key,
            &runtime.model_name,
            system_prompt,
            text,
            text,
        )
        .await?;

        tracing::info!(
            target = "miaoyu_llm",
            original_length = text.len(),
            polished_length = chat_result.content.len(),
            "文本润色完成"
        );

        Ok(PolishResult {
            text: chat_result.content,
            total_tokens: chat_result.total_tokens,
        })
    }

    pub async fn test_api_key(
        app: &AppHandle<Wry>,
        model_override: Option<&str>,
        provider_override: Option<&str>,
        api_key_override: Option<String>,
    ) -> Result<()> {
        if api_key_override.is_none() {
            if let Some(entry) = models::active_llm_entry(app, model_override, provider_override)
                .map_err(|e| anyhow!("读取文本模型配置失败: {e}"))?
            {
                models::check_llm_quota(app, &entry).map_err(|msg| anyhow!(msg))?;
            }
        }
        let settings = SettingsStore::get(app).ok().flatten();
        let runtime =
            resolve_llm_runtime_config(app, model_override, provider_override, api_key_override)?;
        let system_prompt = settings
            .as_ref()
            .and_then(|s| s.llm_system_prompt.as_deref())
            .unwrap_or(DEFAULT_SYSTEM_PROMPT);

        let _ = Self::send_chat_request(
            &runtime.api_url,
            &runtime.api_key,
            &runtime.model_name,
            system_prompt,
            "ping",
            "ping",
        )
        .await?;

        tracing::info!(
            target = "miaoyu_llm",
            model = runtime.model_name,
            "LLM API 密钥测试成功"
        );

        Ok(())
    }

    async fn send_chat_request(
        api_url: &str,
        api_key: &str,
        model_name: &str,
        system_prompt: &str,
        user_text: &str,
        fallback: &str,
    ) -> Result<ChatResult> {
        // ModelScope 的 Qwen 接口要求在非流式调用里显式关闭 enable_thinking
        let should_disable_thinking = api_url.contains("modelscope.cn");
        let request = ChatRequest {
            model: model_name.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_text.to_string(),
                },
            ],
            stream: false,
            enable_thinking: should_disable_thinking.then_some(false),
        };

        let client = reqwest::Client::new();
        let response = client
            .post(api_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("调用 DeepSeek API 失败")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let request_body = serde_json::to_string(&request).unwrap_or_default();
            tracing::error!(
                target = "miaoyu_llm",
                status = %status,
                error = %error_text,
                api_url,
                api_key,
                model_name,
                request_body,
                "DeepSeek API 返回错误"
            );
            anyhow::bail!("DeepSeek API 调用失败: {}", status);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("解析 DeepSeek API 响应失败")?;

        let content = chat_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_else(|| fallback.to_string());
        let total_tokens = chat_response.usage.and_then(|usage| usage.total_tokens);

        Ok(ChatResult {
            content,
            total_tokens,
        })
    }
}

#[tauri::command]
#[specta::specta]
pub async fn test_llm_api_key(
    app: AppHandle,
    model: Option<String>,
    provider: Option<String>,
    api_key: Option<String>,
) -> Result<(), String> {
    LLMService::test_api_key(&app, model.as_deref(), provider.as_deref(), api_key)
        .await
        .map_err(|e| e.to_string())
}
