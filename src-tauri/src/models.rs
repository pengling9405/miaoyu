use std::sync::Arc;

use chrono::Local;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json;
use specta::Type;
use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};

use crate::audio::local_models;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(rename = "apiKeyUrl", alias = "api_key_url", default)]
    pub api_key_url: Option<String>,
    #[serde(rename = "apiBaseUrl", alias = "api_base_url", default)]
    pub api_base_url: Option<String>,
    #[serde(rename = "apiKeyEnv", alias = "api_key_env", default)]
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LlmModelConfig {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub providers: Vec<LlmProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SupportedModels {
    #[serde(rename = "llmModels", alias = "llm-models")]
    pub llm_models: Vec<LlmModelConfig>,
    #[serde(rename = "asrModels", alias = "asr-models")]
    pub asr_models: Vec<AsrModelConfig>,
}

pub static SUPPORTED_MODELS: Lazy<SupportedModels> = Lazy::new(|| SupportedModels {
    llm_models: vec![
        LlmModelConfig {
            id: "deepseek".to_string(),
            title: "DeepSeek".to_string(),
            providers: vec![
                LlmProviderConfig {
                    id: "deepseek".to_string(),
                    name: "DeepSeek".to_string(),
                    model: Some("deepseek-chat".to_string()),
                    api_key_url: Some("https://platform.deepseek.com/api_keys".to_string()),
                    api_base_url: Some("https://api.deepseek.com/v1/chat/completions".to_string()),
                    api_key_env: Some("DEEPSEEK_API_KEY".to_string()),
                },
                LlmProviderConfig {
                    id: "modelscope".to_string(),
                    name: "魔搭社区".to_string(),
                    model: Some("deepseek-ai/DeepSeek-V3.2-Exp".to_string()),
                    api_key_url: Some("https://modelscope.cn/my/myaccesstoken".to_string()),
                    api_base_url: Some(
                        "https://api-inference.modelscope.cn/v1/chat/completions".to_string(),
                    ),
                    api_key_env: Some("MODELSCOPE_ACCESS_TOKEN".to_string()),
                },
            ],
        },
        LlmModelConfig {
            id: "qwen".to_string(),
            title: "通义千问".to_string(),
            providers: vec![LlmProviderConfig {
                id: "modelscope".to_string(),
                name: "魔搭社区".to_string(),
                model: Some("Qwen/Qwen3-32B".to_string()),
                api_key_url: Some("https://modelscope.cn/my/myaccesstoken".to_string()),
                api_base_url: Some(
                    "https://api-inference.modelscope.cn/v1/chat/completions".to_string(),
                ),
                api_key_env: Some("MODELSCOPE_ACCESS_TOKEN".to_string()),
            }],
        },
    ],
    asr_models: vec![
        AsrModelConfig {
            id: local_models::PARAFORMER_MODEL_ID.to_string(),
            title: "Paraformer 中文通用离线轻量版".to_string(),
            offline: true,
            size: "83.4 MB".to_string(),
            providers: vec![AsrProviderConfig {
                id: "local".to_string(),
                name: "本地".to_string(),
                model: None,
            }],
        },
        AsrModelConfig {
            id: local_models::SENSEVOICE_MODEL_ID.to_string(),
            title: "SenseVoice 中英日韩粤语离线轻量版".to_string(),
            size: "244 MB".to_string(),
            offline: true,
            providers: vec![AsrProviderConfig {
                id: "local".to_string(),
                name: "本地".to_string(),
                model: None,
            }],
        },
    ],
});

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AsrProviderConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AsrModelConfig {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub offline: bool,
    pub size: String,
    #[serde(default)]
    pub providers: Vec<AsrProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmModelStore {
    pub id: String,
    #[serde(rename = "textModelId", alias = "text-model-id", default)]
    pub text_model_id: String,
    pub provider: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub free_total_requests: u32,
    #[serde(default)]
    pub free_total_token_usage: u32,
    #[serde(default)]
    pub total_requests: u32,
    #[serde(default)]
    pub total_token_usage: u32,
    #[serde(default)]
    pub active: bool,
    #[serde(rename = "usageDate", alias = "usage-date", default)]
    pub usage_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelsStore {
    #[serde(rename = "llmModels", alias = "llm-models", default)]
    pub llm_models: Vec<LlmModelStore>,
    #[serde(rename = "activeLlmModel", alias = "active-llm-model", default)]
    pub active_llm_model: Option<String>,
    #[serde(rename = "asrModels", alias = "asr-models", default)]
    pub asr_models: Vec<AsrModelStore>,
    #[serde(rename = "activeAsrModel", alias = "active-asr-model", default)]
    pub active_asr_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct AsrModelStore {
    pub id: String,
    #[serde(rename = "modelId", alias = "model-id", default)]
    pub model_id: String,
    pub provider: String,
    #[serde(rename = "appId", alias = "app-id", default)]
    pub app_id: Option<String>,
    #[serde(rename = "accessToken", alias = "access-token", default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub offline: bool,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub total_requests: u32,
    #[serde(default)]
    pub total_hours: f32,
}

const MODELS_STORE_NAME: &str = "store";
const MODELS_STORE_KEY: &str = "models";
const LLM_DAILY_TOKEN_LIMIT: u32 = 5_000;

fn read_store(app: &AppHandle<Wry>) -> Result<(Arc<Store<Wry>>, ModelsStore), String> {
    let handle = app
        .store(MODELS_STORE_NAME)
        .map_err(|e| format!("加载模型存储失败: {e}"))?;

    let mut data = handle
        .get(MODELS_STORE_KEY)
        .and_then(|value| serde_json::from_value::<ModelsStore>(value).ok())
        .unwrap_or_default();

    if data.llm_models.is_empty() && data.asr_models.is_empty() {
        data = ModelsStore::default();
    }

    Ok((handle, data))
}

fn persist_store(handle: &Arc<Store<Wry>>, data: &ModelsStore) -> Result<(), String> {
    handle.set(
        MODELS_STORE_KEY,
        serde_json::to_value(data).map_err(|e| e.to_string())?,
    );
    handle.save().map_err(|e| e.to_string())
}

fn resolve_variant_id(model: &LlmModelConfig, provider: &LlmProviderConfig) -> String {
    provider
        .model
        .clone()
        .unwrap_or_else(|| format!("{}::{}", model.id, provider.id))
}

fn resolve_asr_variant_id(model: &AsrModelConfig, provider: &AsrProviderConfig) -> String {
    provider
        .model
        .clone()
        .unwrap_or_else(|| format!("{}::{}", model.id, provider.id))
}

fn ensure_llm_defaults(data: &mut ModelsStore, config: &SupportedModels) {
    for model in &config.llm_models {
        for provider in &model.providers {
            let variant_id = resolve_variant_id(model, provider);
            if let Some(entry) = data
                .llm_models
                .iter_mut()
                .find(|entry| entry.id == variant_id)
            {
                entry.text_model_id = model.id.clone();
                entry.provider = provider.id.clone();
            } else {
                data.llm_models.push(LlmModelStore {
                    id: variant_id,
                    text_model_id: model.id.clone(),
                    provider: provider.id.clone(),
                    ..Default::default()
                });
            }
        }

        let indices: Vec<_> = data
            .llm_models
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if entry.text_model_id == model.id {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();
        if indices.is_empty() {
            continue;
        }
        let mut has_active = false;
        for idx in &indices {
            let entry = &mut data.llm_models[*idx];
            if entry.active {
                if !has_active {
                    has_active = true;
                } else {
                    entry.active = false;
                }
            }
        }
        if !has_active {
            if let Some(idx) = indices.first() {
                if let Some(entry) = data.llm_models.get_mut(*idx) {
                    entry.active = true;
                }
            }
        }
    }

    if data
        .active_llm_model
        .as_ref()
        .map(|id| config.llm_models.iter().any(|model| &model.id == id))
        != Some(true)
    {
        data.active_llm_model = config.llm_models.first().map(|model| model.id.clone());
    }
}

fn ensure_asr_defaults(data: &mut ModelsStore, config: &SupportedModels) {
    for model in &config.asr_models {
        for provider in &model.providers {
            let variant_id = resolve_asr_variant_id(model, provider);
            if let Some(entry) = data
                .asr_models
                .iter_mut()
                .find(|entry| entry.id == variant_id)
            {
                entry.model_id = model.id.clone();
                entry.provider = provider.id.clone();
                entry.offline = model.offline;
            } else {
                data.asr_models.push(AsrModelStore {
                    id: variant_id,
                    model_id: model.id.clone(),
                    provider: provider.id.clone(),
                    offline: model.offline,
                    active: true,
                    ..Default::default()
                });
            }
        }

        let indices: Vec<_> = data
            .asr_models
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| (entry.model_id == model.id).then_some(index))
            .collect();

        if indices.is_empty() {
            continue;
        }

        let mut has_active = false;
        for idx in &indices {
            if let Some(entry) = data.asr_models.get_mut(*idx) {
                if entry.active {
                    if has_active {
                        entry.active = false;
                    } else {
                        has_active = true;
                    }
                }
            }
        }

        if !has_active {
            if let Some(first) = indices
                .first()
                .and_then(|idx| data.asr_models.get_mut(*idx))
            {
                first.active = true;
            }
        }
    }

    if data
        .active_asr_model
        .as_ref()
        .map(|id| config.asr_models.iter().any(|model| &model.id == id))
        != Some(true)
    {
        data.active_asr_model = config.asr_models.first().map(|model| model.id.clone());
    }
}

fn hydrate_models(data: &mut ModelsStore, config: &SupportedModels) {
    ensure_llm_defaults(data, config);
    ensure_asr_defaults(data, config);
}

fn with_models_store<F>(app: &AppHandle<Wry>, mutator: F) -> Result<ModelsStore, String>
where
    F: FnOnce(&SupportedModels, &mut ModelsStore) -> Result<(), String>,
{
    let config = supported_models();
    let (handle, mut data) = read_store(app)?;
    hydrate_models(&mut data, config);
    mutator(config, &mut data)?;
    hydrate_models(&mut data, config);
    persist_store(&handle, &data)?;
    Ok(data)
}

fn today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn reset_llm_daily_usage(entry: &mut LlmModelStore, today: &str) {
    if entry.usage_date.as_deref() != Some(today) {
        entry.usage_date = Some(today.to_string());
        entry.free_total_requests = 0;
        entry.free_total_token_usage = 0;
    }
}

fn sanitize_api_key(value: Option<String>) -> Option<String> {
    value.and_then(|key| {
        let trimmed = key.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub fn supported_models() -> &'static SupportedModels {
    &SUPPORTED_MODELS
}

#[tauri::command]
#[specta::specta]
pub fn get_supported_models() -> SupportedModels {
    SUPPORTED_MODELS.clone()
}

fn load(app: &AppHandle<Wry>) -> Result<ModelsStore, String> {
    with_models_store(app, |_, _| Ok(()))
}

#[tauri::command]
#[specta::specta]
pub fn get_models_store(app: AppHandle) -> Result<ModelsStore, String> {
    load(&app)
}

pub fn active_llm_entry(
    app: &AppHandle<Wry>,
    override_model: Option<&str>,
    override_provider: Option<&str>,
) -> Result<Option<LlmModelStore>, String> {
    let data = load(app)?;
    let target = override_model
        .map(|value| value.to_string())
        .or(data.active_llm_model.clone());

    let Some(model_id) = target else {
        return Ok(None);
    };

    if let Some(provider_id) = override_provider {
        if let Some(entry) = data
            .llm_models
            .iter()
            .find(|entry| entry.text_model_id == model_id && entry.provider == provider_id)
        {
            return Ok(Some(entry.clone()));
        }
    }

    if let Some(entry) = data
        .llm_models
        .iter()
        .find(|entry| entry.text_model_id == model_id && entry.active)
    {
        return Ok(Some(entry.clone()));
    }

    Ok(data
        .llm_models
        .into_iter()
        .find(|entry| entry.text_model_id == model_id))
}

pub fn active_asr_entry(
    app: &AppHandle<Wry>,
    override_model: Option<&str>,
    override_provider: Option<&str>,
) -> Result<Option<AsrModelStore>, String> {
    let data = load(app)?;
    let target = override_model
        .map(|value| value.to_string())
        .or(data.active_asr_model.clone());

    let Some(model_id) = target else {
        return Ok(None);
    };

    if let Some(provider_id) = override_provider {
        if let Some(entry) = data
            .asr_models
            .iter()
            .find(|entry| entry.model_id == model_id && entry.provider == provider_id)
        {
            return Ok(Some(entry.clone()));
        }
    }

    if let Some(entry) = data
        .asr_models
        .iter()
        .find(|entry| entry.model_id == model_id && entry.active)
    {
        return Ok(Some(entry.clone()));
    }

    Ok(data
        .asr_models
        .into_iter()
        .find(|entry| entry.model_id == model_id))
}

#[tauri::command]
#[specta::specta]
pub fn set_active_text_model(app: AppHandle, model_id: String) -> Result<ModelsStore, String> {
    with_models_store(&app, |config, data| {
        if config.llm_models.iter().any(|model| model.id == model_id) {
            data.active_llm_model = Some(model_id);
        }
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn set_active_asr_model(app: AppHandle, model_id: String) -> Result<ModelsStore, String> {
    with_models_store(&app, |config, data| {
        if config.asr_models.iter().any(|model| model.id == model_id) {
            data.active_asr_model = Some(model_id.clone());
            for entry in &mut data.asr_models {
                entry.active = entry.model_id == model_id;
            }
        }
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_text_model_credentials(
    app: AppHandle,
    model_id: String,
    provider_id: String,
    api_key: Option<String>,
) -> Result<ModelsStore, String> {
    let sanitized = sanitize_api_key(api_key);

    with_models_store(&app, |config, data| {
        let Some(model_config) = config.llm_models.iter().find(|model| model.id == model_id) else {
            return Err("未知文本模型".to_string());
        };

        let Some(provider_config) = model_config
            .providers
            .iter()
            .find(|provider| provider.id == provider_id)
        else {
            return Err("未知文本模型提供商".to_string());
        };

        let variant_id = resolve_variant_id(model_config, provider_config);
        if let Some(entry) = data
            .llm_models
            .iter_mut()
            .find(|entry| entry.id == variant_id)
        {
            entry.api_key = sanitized.clone();
            entry.active = true;
        } else {
            data.llm_models.push(LlmModelStore {
                id: variant_id,
                text_model_id: model_config.id.clone(),
                provider: provider_config.id.clone(),
                api_key: sanitized.clone(),
                active: true,
                ..Default::default()
            });
        }

        if !data
            .llm_models
            .iter()
            .any(|entry| entry.text_model_id == model_config.id && entry.active)
        {
            if let Some(entry) = data
                .llm_models
                .iter_mut()
                .find(|entry| entry.text_model_id == model_config.id)
            {
                entry.active = true;
            }
        }

        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_asr_credentials(
    app: AppHandle,
    model_id: String,
    provider_id: String,
    app_id: Option<String>,
    access_token: Option<String>,
) -> Result<ModelsStore, String> {
    with_models_store(&app, |config, data| {
        let Some(model_config) = config.asr_models.iter().find(|model| model.id == model_id) else {
            return Err("未知语音识别模型".to_string());
        };

        let Some(provider_config) = model_config
            .providers
            .iter()
            .find(|provider| provider.id == provider_id)
        else {
            return Err("未知语音识别提供商".to_string());
        };

        let variant_id = resolve_asr_variant_id(model_config, provider_config);
        if let Some(entry) = data
            .asr_models
            .iter_mut()
            .find(|entry| entry.id == variant_id)
        {
            entry.app_id = sanitize_api_key(app_id.clone());
            entry.access_token = sanitize_api_key(access_token.clone());
            entry.active = true;
        } else {
            data.asr_models.push(AsrModelStore {
                id: variant_id,
                model_id: model_config.id.clone(),
                provider: provider_config.id.clone(),
                app_id: sanitize_api_key(app_id.clone()),
                access_token: sanitize_api_key(access_token.clone()),
                offline: model_config.offline,
                active: true,
                ..Default::default()
            });
        }

        if !data
            .asr_models
            .iter()
            .any(|entry| entry.model_id == model_config.id && entry.active)
        {
            if let Some(entry) = data
                .asr_models
                .iter_mut()
                .find(|entry| entry.model_id == model_config.id)
            {
                entry.active = true;
            }
        }

        Ok(())
    })
}

fn has_user_llm_key(entry: &LlmModelStore) -> bool {
    entry
        .api_key
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn check_llm_quota(app: &AppHandle<Wry>, entry: &LlmModelStore) -> Result<(), String> {
    if has_user_llm_key(entry) {
        return Ok(());
    }

    let entry_id = entry.id.clone();
    with_models_store(app, |_, data| {
        let Some(target) = data.llm_models.iter_mut().find(|item| item.id == entry_id) else {
            return Err("未知文本模型".to_string());
        };
        let today = today_string();
        reset_llm_daily_usage(target, &today);
        if target.free_total_token_usage >= LLM_DAILY_TOKEN_LIMIT {
            return Err("体验额度已用完，请在“模型管理”配置 API 密钥。".to_string());
        }
        Ok(())
    })
    .map(|_| ())
}

pub fn record_llm_usage(
    app: &AppHandle<Wry>,
    entry_id: &str,
    token_usage: u32,
) -> Result<(), String> {
    let entry_id = entry_id.to_string();
    with_models_store(app, |_, data| {
        if let Some(entry) = data
            .llm_models
            .iter_mut()
            .find(|entry| entry.id == entry_id)
        {
            let today = today_string();
            reset_llm_daily_usage(entry, &today);
            entry.total_requests = entry.total_requests.saturating_add(1);
            entry.total_token_usage = entry.total_token_usage.saturating_add(token_usage);
            entry.free_total_requests = entry.free_total_requests.saturating_add(1);
            entry.free_total_token_usage = entry.free_total_token_usage.saturating_add(token_usage);
        }
        Ok(())
    })
    .map(|_| ())
}

pub fn revert_llm_usage(
    app: &AppHandle<Wry>,
    entry_id: &str,
    token_usage: u32,
) -> Result<(), String> {
    let entry_id = entry_id.to_string();
    with_models_store(app, |_, data| {
        if let Some(entry) = data
            .llm_models
            .iter_mut()
            .find(|entry| entry.id == entry_id)
        {
            entry.total_requests = entry.total_requests.saturating_sub(1);
            entry.total_token_usage = entry.total_token_usage.saturating_sub(token_usage);
            entry.free_total_requests = entry.free_total_requests.saturating_sub(1);
            entry.free_total_token_usage = entry.free_total_token_usage.saturating_sub(token_usage);
        }
        Ok(())
    })
    .map(|_| ())
}

pub fn record_asr_usage(
    app: &AppHandle<Wry>,
    entry_id: &str,
    duration_seconds: u32,
) -> Result<(), String> {
    let entry_id = entry_id.to_string();
    let hours = duration_seconds as f32 / 3600.0;
    with_models_store(app, |_, data| {
        if let Some(entry) = data
            .asr_models
            .iter_mut()
            .find(|entry| entry.id == entry_id)
        {
            entry.total_requests = entry.total_requests.saturating_add(1);
            entry.total_hours = (entry.total_hours + hours).max(0.0);
        }
        Ok(())
    })
    .map(|_| ())
}

pub fn revert_asr_usage(
    app: &AppHandle<Wry>,
    entry_id: &str,
    duration_seconds: u32,
) -> Result<(), String> {
    let entry_id = entry_id.to_string();
    let hours = duration_seconds as f32 / 3600.0;
    with_models_store(app, |_, data| {
        if let Some(entry) = data
            .asr_models
            .iter_mut()
            .find(|entry| entry.id == entry_id)
        {
            entry.total_requests = entry.total_requests.saturating_sub(1);
            entry.total_hours = (entry.total_hours - hours).max(0.0);
        }
        Ok(())
    })
    .map(|_| ())
}

pub fn reset_usage_stats(app: &AppHandle<Wry>) -> Result<(), String> {
    with_models_store(app, |_, data| {
        for entry in &mut data.llm_models {
            entry.total_requests = 0;
            entry.total_token_usage = 0;
            entry.free_total_requests = 0;
            entry.free_total_token_usage = 0;
            entry.usage_date = None;
        }
        for entry in &mut data.asr_models {
            entry.total_requests = 0;
            entry.total_hours = 0.0;
        }
        Ok(())
    })
    .map(|_| ())
}
