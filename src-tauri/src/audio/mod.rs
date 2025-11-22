pub(crate) mod dictating;
pub(crate) mod local_models;
mod transcribing;

pub use transcribing::TranscriptionResult;

use dictating::{AudioDictating, DictatingStream};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager, Wry};
use tauri_specta::Event;
use tracing::warn;

use crate::clipboard;
use crate::history::{self, HistoryKind, LlmPolishStatus, NewHistoryEntry};
use crate::hotkeys;
use crate::llm::LLMService;
use crate::models;
use crate::notification::{self, NotificationType};
use crate::windows::{self, AppWindowId, ShowAppWindow};
use crate::{AppState, AudioState};

pub use local_models::{download_offline_models, get_offline_models_status};

#[tauri::command(async)]
#[specta::specta]
pub async fn start_dictating(app: AppHandle) -> Result<(), String> {
    start_recording(app, HistoryKind::Dictation).await
}

#[tauri::command(async)]
#[specta::specta]
pub async fn start_voice_diary(app: AppHandle) -> Result<(), String> {
    start_recording(app, HistoryKind::Diary).await
}

#[derive(Serialize, Type, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum TranscribingStage {
    Asr,
    Polishing,
}

#[derive(Serialize, Type, tauri_specta::Event, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OnTranscribingStage {
    pub stage: TranscribingStage,
}

async fn start_recording(app: AppHandle, history_kind: HistoryKind) -> Result<(), String> {
    ensure_model_downloaded(&app).await?;

    let state = app.state::<AppState>();
    {
        let mut guard = state.audio.lock().await;
        if guard.state == AudioState::Recording {
            return Err("当前已有录音进行中".to_string());
        }
        guard.state = AudioState::Recording;
        guard.dictating_stream = None;
        guard.history_kind = history_kind;
    }
    hotkeys::set_escape_shortcut_enabled(&app, true);

    if let Some(window) = AppWindowId::Dashboard.get(&app) {
        if let Err(error) = window.destroy() {
            warn!(
                target = "miaoyu_audio",
                error = %error,
                "关闭 Dashboard 窗口失败"
            );
        }
    }

    if let Err(error) = ShowAppWindow::AudioRecording.show(&app).await {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "显示录音窗口失败"
        );
    }
    let _ = windows::sync_audio_overlay(&app, AudioState::Recording).await;

    if let Err(error) = play_sound_blocking(AudioDictating::play_start_sound).await {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "播放开始录音音效失败"
        );
    }

    let stream = match DictatingStream::new() {
        Ok(stream) => stream,
        Err(error) => {
            set_idle_state(&app).await;
            return Err(error);
        }
    };

    {
        let mut guard = state.audio.lock().await;
        guard.dictating_stream = Some(stream);
    }

    Ok(())
}

async fn ensure_model_downloaded(app: &AppHandle<Wry>) -> Result<(), String> {
    let active_entry = models::active_asr_entry(app, None, None).map_err(|err| err.to_string())?;
    if let Some(entry) = active_entry {
        if entry.offline {
            local_models::ensure_model_ready(app, &entry.model_id)
        } else {
            Ok(())
        }
    } else {
        local_models::ensure_model_ready(app, local_models::DEFAULT_MODEL_ID)
    }
}

#[tauri::command(async)]
#[specta::specta]
pub async fn stop_dictating(app: AppHandle) -> Result<transcribing::TranscriptionResult, String> {
    let state = app.state::<AppState>();
    let (stream, history_kind) = {
        let mut guard = state.audio.lock().await;
        if guard.state != AudioState::Recording {
            return Err("当前没有正在进行的录音".to_string());
        }
        guard.state = AudioState::Transcribing;
        let stream = guard
            .dictating_stream
            .take()
            .ok_or_else(|| "录音准备中，请稍候再试".to_string())?;
        (stream, guard.history_kind)
    };

    if let Err(error) = ShowAppWindow::AudioTranscribing.show(&app).await {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "显示转写窗口失败"
        );
    }
    OnTranscribingStage {
        stage: TranscribingStage::Asr,
    }
    .emit(&app)
    .ok();
    let _ = windows::sync_audio_overlay(&app, AudioState::Transcribing).await;

    // 用户第二次按下快捷键时立即播结束音效（而不是等转写完成）
    if let Err(error) = play_sound_blocking(AudioDictating::play_stop_sound).await {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "播放结束录音音效失败"
        );
    }

    let (samples, sample_rate) = stream.into_samples();
    let active_asr_entry = match models::active_asr_entry(&app, None, None) {
        Ok(entry) => entry,
        Err(error) => {
            warn!(
                target = "miaoyu_audio",
                error = %error,
                "获取当前语音模型失败，使用默认模型继续转写"
            );
            None
        }
    };
    let active_model_id = active_asr_entry
        .as_ref()
        .map(|entry| entry.model_id.as_str())
        .unwrap_or(local_models::DEFAULT_MODEL_ID);

    let mut transcription = match transcribing::AudioTranscribing::transcribe(
        &app,
        samples.clone(),
        sample_rate,
        active_model_id,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            warn!(target = "miaoyu_audio", error = %error, "离线识别失败");
            set_idle_state(&app).await;
            return Err(error.to_string());
        }
    };

    OnTranscribingStage {
        stage: TranscribingStage::Polishing,
    }
    .emit(&app)
    .ok();
    let llm_outcome = polish_transcription(&app, &transcription.text).await;
    transcription.text = llm_outcome.text.clone();
    transcription.llm_polish_status = llm_outcome.status;
    transcription.llm_polish_error = llm_outcome.error.clone();

    if matches!(
        llm_outcome.status,
        LlmPolishStatus::Failed | LlmPolishStatus::QuotaExceeded
    ) {
        let message = llm_outcome
            .error
            .clone()
            .unwrap_or_else(|| "AI 润色失败，请稍后再试".to_string());
        let _ =
            notification::show_notification(app.clone(), message, NotificationType::Error, None)
                .await;
    }

    let audio_file_path = match history::save_history_audio_clip(&app, &samples, sample_rate).await
    {
        Ok(path) => Some(path),
        Err(error) => {
            warn!(
                target = "miaoyu_audio",
                error = %error,
                "保存历史音频失败"
            );
            None
        }
    };

    if let Err(error) = log_history_entry(
        &app,
        &transcription,
        history_kind,
        active_asr_entry.as_ref(),
        &llm_outcome,
        audio_file_path.clone(),
    )
    .await
    {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "保存历史记录失败"
        );
    }

    if let Err(error) = clipboard::paste(transcription.text.clone(), &app) {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "自动粘贴失败"
        );
        let _ = notification::show_notification(
            app.clone(),
            "自动粘贴失败，内容已复制到剪贴板".to_string(),
            NotificationType::Error,
            None,
        )
        .await;
    }

    set_idle_state(&app).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;

    if let Err(error) = clipboard::paste(transcription.text.clone(), &app) {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "自动粘贴失败"
        );
        let _ = notification::show_notification(
            app.clone(),
            "自动粘贴失败，内容已复制到剪贴板".to_string(),
            NotificationType::Error,
            None,
        )
        .await;
    }

    Ok(transcription)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn cancel_dictating(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let mut guard = state.audio.lock().await;
        guard.dictating_stream = None;
        guard.state = AudioState::Idle;
    }
    if let Err(error) = play_sound_blocking(AudioDictating::play_stop_sound).await {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "播放结束录音音效失败"
        );
    }
    let _ = windows::sync_audio_overlay(&app, AudioState::Idle).await;
    Ok(())
}

async fn set_idle_state(app: &AppHandle<Wry>) {
    let state = app.state::<AppState>();
    {
        let mut guard = state.audio.lock().await;
        guard.state = AudioState::Idle;
    }
    let _ = windows::sync_audio_overlay(app, AudioState::Idle).await;
    if let Some(window) = AppWindowId::AudioRecording.get(app) {
        let _ = window.hide();
    }
    if let Some(window) = AppWindowId::AudioTranscribing.get(app) {
        let _ = window.hide();
    }
    hotkeys::set_escape_shortcut_enabled(app, false);
}

async fn log_history_entry(
    app: &AppHandle<Wry>,
    transcription: &TranscriptionResult,
    history_kind: HistoryKind,
    active_asr_entry: Option<&models::AsrModelStore>,
    llm_outcome: &LlmPolishOutcome,
    audio_file_path: Option<String>,
) -> Result<(), String> {
    let asr_model_id = active_asr_entry
        .as_ref()
        .map(|entry| entry.model_id.clone())
        .unwrap_or_else(|| local_models::DEFAULT_MODEL_ID.to_string());
    let asr_variant_id = active_asr_entry
        .as_ref()
        .map(|entry| entry.id.clone())
        .unwrap_or_else(|| local_models::DEFAULT_MODEL_ID.to_string());

    let duration_seconds = transcription
        .duration_ms
        .map(|ms| (ms as u64).div_ceil(1000).min(u32::MAX as u64) as u32)
        .unwrap_or(0);

    let words = transcription.text.chars().count() as u32;
    let entry = NewHistoryEntry {
        id: None,
        text: transcription.text.clone(),
        kind: history_kind,
        title: None,
        duration_seconds,
        created_at: None,
        audio_file_path,
        llm_model: llm_outcome.llm_model.clone(),
        llm_variant_id: llm_outcome.llm_variant_id.clone(),
        asr_model: Some(asr_model_id),
        asr_variant_id: Some(asr_variant_id.clone()),
        total_words: Some(words),
        total_tokens: Some(words),
        llm_total_tokens: llm_outcome.llm_total_tokens,
        source_app: None,
        llm_polish_status: llm_outcome.status,
        llm_polish_error: llm_outcome.error.clone(),
    };

    history::add_history_entry(app.clone(), entry).await?;

    if let Err(error) = models::record_asr_usage(app, &asr_variant_id, duration_seconds) {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "记录语音模型使用统计失败"
        );
    }

    Ok(())
}

struct LlmPolishOutcome {
    text: String,
    llm_model: Option<String>,
    llm_variant_id: Option<String>,
    llm_total_tokens: Option<u32>,
    status: LlmPolishStatus,
    error: Option<String>,
}

impl LlmPolishOutcome {
    fn from_error(
        text: String,
        status: LlmPolishStatus,
        error: Option<String>,
        model: Option<String>,
        variant: Option<String>,
    ) -> Self {
        Self {
            text,
            llm_model: model,
            llm_variant_id: variant,
            llm_total_tokens: None,
            status,
            error,
        }
    }
}

async fn polish_transcription(app: &AppHandle<Wry>, text: &str) -> LlmPolishOutcome {
    let original_text = text.to_string();
    let llm_entry = match models::active_llm_entry(app, None, None) {
        Ok(Some(entry)) => entry,
        Ok(None) => {
            return LlmPolishOutcome::from_error(
                original_text,
                LlmPolishStatus::Skipped,
                None,
                None,
                None,
            )
        }
        Err(error) => {
            warn!(
                target = "miaoyu_llm",
                error = %error,
                "读取文本模型配置失败，跳过润色"
            );
            return LlmPolishOutcome::from_error(
                original_text,
                LlmPolishStatus::Failed,
                Some(error),
                None,
                None,
            );
        }
    };

    match LLMService::polish_text(app, text).await {
        Ok(result) => {
            if let Some(tokens) = result.total_tokens {
                if let Err(error) = models::record_llm_usage(app, &llm_entry.id, tokens) {
                    warn!(
                        target = "miaoyu_llm",
                        error = %error,
                        "记录文本模型使用统计失败"
                    );
                }
            }
            LlmPolishOutcome {
                text: result.text,
                llm_model: Some(llm_entry.text_model_id.clone()),
                llm_variant_id: Some(llm_entry.id.clone()),
                llm_total_tokens: result.total_tokens,
                status: LlmPolishStatus::Success,
                error: None,
            }
        }
        Err(err) => {
            let message = err.to_string();
            let status = if message.contains("额度已用完") {
                LlmPolishStatus::QuotaExceeded
            } else if message.contains("未配置文本模型") {
                LlmPolishStatus::Skipped
            } else {
                LlmPolishStatus::Failed
            };
            LlmPolishOutcome::from_error(
                original_text,
                status,
                if status == LlmPolishStatus::Skipped {
                    None
                } else {
                    Some(message)
                },
                Some(llm_entry.text_model_id.clone()),
                Some(llm_entry.id.clone()),
            )
        }
    }
}

async fn play_sound_blocking<F>(play_fn: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    tokio::task::spawn_blocking(play_fn)
        .await
        .map_err(|e| format!("播放音效失败: {e}"))?
}
