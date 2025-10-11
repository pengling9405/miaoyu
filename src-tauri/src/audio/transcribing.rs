use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Wry};
use uuid::Uuid;

use crate::settings::SettingsStore;

const AUC_API_URL: &str = "https://openspeech.bytedance.com/api/v3/auc/bigmodel/recognize/flash";

/// 获取火山引擎配置
/// 优先级：用户设置 > 运行时环境变量 > 编译时默认值
fn get_volcengine_config(settings: &Option<SettingsStore>) -> Result<(String, String)> {
    // App ID
    let app_id = settings
        .as_ref()
        .and_then(|s| s.asr_app_id.clone())
        .or_else(|| std::env::var("VOLCENGINE_APP_ID").ok())
        .or_else(|| option_env!("VOLCENGINE_APP_ID").map(String::from))
        .ok_or_else(|| anyhow::anyhow!("未配置火山引擎 App ID，请在设置中配置"))?;

    // Access Token
    let access_token = settings
        .as_ref()
        .and_then(|s| s.asr_access_token.clone())
        .or_else(|| std::env::var("VOLCENGINE_ACCESS_TOKEN").ok())
        .or_else(|| option_env!("VOLCENGINE_ACCESS_TOKEN").map(String::from))
        .ok_or_else(|| anyhow::anyhow!("未配置火山引擎 Access Token，请在设置中配置"))?;

    Ok((app_id, access_token))
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: Option<u32>,
    pub utterances: Vec<TranscriptionUtterance>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionUtterance {
    pub text: String,
    pub start_time: u32,
    pub end_time: u32,
}

#[derive(Debug, Deserialize)]
struct VolcRecognizeResponse {
    #[serde(default)]
    audio_info: Option<VolcAudioInfo>,
    #[serde(default)]
    result: Option<VolcResult>,
}

#[derive(Debug, Deserialize)]
struct VolcAudioInfo {
    #[serde(default)]
    duration: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct VolcResult {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    utterances: Option<Vec<VolcUtterance>>,
    #[serde(default)]
    additions: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct VolcUtterance {
    #[serde(default)]
    end_time: Option<u64>,
    #[serde(default)]
    start_time: Option<u64>,
    #[serde(default)]
    text: Option<String>,
}

pub struct AudioTranscribing;

impl AudioTranscribing {
    pub async fn transcribe(
        app: &AppHandle<Wry>,
        wav_data: Vec<u8>,
    ) -> Result<TranscriptionResult> {
        // 读取用户配置或环境变量
        let settings = SettingsStore::get(app).ok().flatten();
        let (app_id, access_token) = get_volcengine_config(&settings)?;

        let audio = Base64.encode(wav_data);
        let request_id = Uuid::new_v4().to_string();

        let client = reqwest::Client::new();

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "X-Api-App-Key",
            reqwest::header::HeaderValue::from_str(&app_id).context("无效的 X-Api-App-Key")?,
        );
        headers.insert(
            "X-Api-Access-Key",
            reqwest::header::HeaderValue::from_str(&access_token)
                .context("无效的 X-Api-Access-Key")?,
        );
        headers.insert(
            "X-Api-Resource-Id",
            reqwest::header::HeaderValue::from_static("volc.bigasr.auc_turbo"),
        );
        headers.insert(
            "X-Api-Request-Id",
            reqwest::header::HeaderValue::from_str(&request_id)
                .context("无效的 X-Api-Request-Id")?,
        );
        headers.insert(
            "X-Api-Sequence",
            reqwest::header::HeaderValue::from_static("-1"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let body = serde_json::json!({
            "user": { "uid": app_id },
            "audio": { "data": audio },
            "request": { "model_name": "bigmodel" },
        });

        let response = client
            .post(AUC_API_URL)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .context("调用语音识别服务失败")?;

        let log_id = response
            .headers()
            .get("X-Tt-Logid")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());
        let status_header = response
            .headers()
            .get("X-Api-Status-Code")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "未返回状态码".to_string());

        let status = response.status();
        let payload: VolcRecognizeResponse =
            response.json().await.context("解析语音识别响应失败")?;

        if status != reqwest::StatusCode::OK || status_header != "20000000" {
            tracing::error!(
                target = "miaoyu_audio",
                ?status,
                status_header = status_header.as_str(),
                log_id,
                response = ?payload,
                "语音识别接口返回错误"
            );
            bail!("语音识别失败，请稍后重试");
        }

        let mut result = payload
            .result
            .ok_or_else(|| anyhow!("语音识别响应缺少结果字段"))?;

        let text = result.text.take().unwrap_or_default();
        let utterances = result
            .utterances
            .take()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|utterance| {
                let text = utterance.text.unwrap_or_default();
                let start_time = u32::try_from(utterance.start_time?).ok()?;
                let end_time = u32::try_from(utterance.end_time?).ok()?;
                Some(TranscriptionUtterance {
                    text,
                    start_time,
                    end_time,
                })
            })
            .collect::<Vec<_>>();

        let duration_ms_raw = payload
            .audio_info
            .and_then(|info| info.duration)
            .or_else(|| {
                result
                    .additions
                    .as_mut()
                    .and_then(|map| map.remove("duration"))
                    .and_then(|value| match value {
                        serde_json::Value::Number(number) => number.as_u64(),
                        serde_json::Value::String(text) => text.parse::<u64>().ok(),
                        _ => None,
                    })
            });

        let duration_ms = duration_ms_raw.and_then(|value| u32::try_from(value).ok());

        Ok(TranscriptionResult {
            text,
            duration_ms,
            utterances,
        })
    }
}
