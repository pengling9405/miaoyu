mod dictating;
mod transcribing;

pub use transcribing::TranscriptionResult;

use crate::{
    clipboard,
    feedback::{show_feedback, FeedbackType},
    hotkeys,
    llm::LLMService,
    permissions::check_os_permissions,
    windows::{resize_main_window, AppWindowId, ShowAppWindow},
};

/// 根据错误信息生成用户友好的提示
fn get_user_friendly_error_message(error: &anyhow::Error) -> String {
    let error_str = error.to_string();

    // 检查是否是配置相关错误
    if error_str.contains("未配置")
        || error_str.contains("API Key")
        || error_str.contains("App ID")
        || error_str.contains("Access Token")
    {
        return "未配置 API 密钥，请在设置中配置".to_string();
    }

    // 检查是否是网络相关错误
    if error_str.contains("网络")
        || error_str.contains("连接")
        || error_str.contains("timeout")
        || error_str.contains("network")
    {
        return "网络连接失败，请检查网络".to_string();
    }

    // 检查是否是 API 调用失败
    if error_str.contains("API") || error_str.contains("调用失败") {
        return format!("服务调用失败: {}", error_str);
    }

    // VAD 或录音相关提示
    if error_str.contains("未检测到语音") {
        return "未检测到语音，请检查麦克风并在录音时保持发声".to_string();
    }

    // 默认返回原始错误信息
    format!("操作失败: {}", error_str)
}
use anyhow::{Context, Result};
use dictating::{AudioDictating, DictatingStream};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter, Wry};
use tokio::sync::Mutex as AsyncMutex;
use transcribing::AudioTranscribing;

// 录音模式
#[derive(Clone, Serialize, Deserialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DictatingMode {
    Normal,
    Hotkey,
}

// 可序列化的状态，用于发送给前端
#[derive(Clone, Serialize, Deserialize, Type, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum AudioState {
    Idle,
    #[serde(rename_all = "camelCase")]
    Dictating {
        mode: DictatingMode,
    },
    Transcribing,
}

// 应用状态管理
pub struct AppState {
    pub audio_state: AudioState,
    pub(crate) dictating_stream: Option<DictatingStream>,
}

impl AppState {
    fn new() -> Self {
        Self {
            audio_state: AudioState::Idle,
            dictating_stream: None,
        }
    }
}

static APP_STATE: Lazy<AsyncMutex<AppState>> = Lazy::new(|| AsyncMutex::new(AppState::new()));

// 辅助函数：发送状态变化事件并调整窗口大小（保持位置）
async fn emit_state_and_resize(
    app: &AppHandle<Wry>,
    state: AudioState,
    width: f64,
    height: f64,
) -> Result<()> {
    // 发送状态变化事件给前端
    app.emit("audio-state-changed", &state)
        .context("Failed to emit audio state")?;

    // 调整窗口大小并保持位置
    if let Some(window) = AppWindowId::Main.get(app) {
        resize_main_window(&window, app, width, height).map_err(|e| anyhow::anyhow!(e))?;
    }

    Ok(())
}

// 获取当前音频状态（用于快捷键处理）
pub async fn get_audio_state() -> AudioState {
    let state = APP_STATE.lock().await;
    state.audio_state.clone()
}

// Tauri commands
#[tauri::command(async)]
#[specta::specta]
pub async fn start_dictating(app: AppHandle, mode: DictatingMode) -> Result<(), String> {
    let permissions = check_os_permissions(false);

    if !permissions.microphone.permitted() || !permissions.accessibility.permitted() {
        tracing::warn!(
            target = "miaoyu_audio",
            ?permissions,
            "缺少所需权限，无法开始语音识别"
        );

        if let Err(error) = ShowAppWindow::Setup.show(&app).await {
            tracing::error!(
                target = "miaoyu_audio",
                error = ?error,
                "显示权限设置窗口失败"
            );
            return Err(error.to_string());
        }

        return Err("请完成必需权限授权后重试".to_string());
    }

    close_window(&app, AppWindowId::Setup);

    // 初始化录音流
    let stream = DictatingStream::new().map_err(|err| {
        tracing::error!(target = "miaoyu_audio", error = ?err, "初始化录音失败");
        err.to_string()
    })?;

    {
        let mut state = APP_STATE.lock().await;
        if !matches!(state.audio_state, AudioState::Idle) {
            return Err("当前已在录音或转写中".to_string());
        }

        state.audio_state = AudioState::Dictating { mode: mode.clone() };
        state.dictating_stream = Some(stream);
    }

    // 播放开始声音
    AudioDictating::play_start_sound();

    // 根据模式调整窗口大小
    let width = match mode {
        DictatingMode::Normal => 160.0,
        DictatingMode::Hotkey => 120.0,
    };

    // 发送状态并调整窗口到 Dictating 大小
    if let Err(err) = emit_state_and_resize(&app, AudioState::Dictating { mode }, width, 32.0).await
    {
        tracing::error!(target = "miaoyu_audio", error = ?err, "更新状态失败");
        reset_to_idle(&app).await;
        return Err(err.to_string());
    }

    hotkeys::set_escape_shortcut_enabled(&app, true);

    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub async fn cancel_dictating(app: AppHandle) -> Result<(), String> {
    let was_recording = {
        let mut state = APP_STATE.lock().await;
        if let AudioState::Dictating { .. } = state.audio_state {
            if let Some(stream) = state.dictating_stream.take() {
                stream.cancel();
                state.audio_state = AudioState::Idle;
                true
            } else {
                false
            }
        } else {
            false
        }
    };

    if was_recording {
        tracing::debug!(target = "miaoyu_audio", "语音识别已取消");
        emit_state_and_resize(&app, AudioState::Idle, 40.0, 8.0)
            .await
            .map_err(|err| err.to_string())?;

        hotkeys::set_escape_shortcut_enabled(&app, false);

        // 播放通知声音
        dictating::AudioDictating::play_notification_sound();

        // 显示取消提示
        let app_clone = app.clone();
        tokio::spawn(async move {
            let _ = show_feedback(
                app_clone,
                "语音识别已取消".to_string(),
                FeedbackType::Toast,
                None,
            )
            .await;
        });
    }

    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub async fn stop_dictating(
    app: AppHandle,
    triggered_by_ui: bool,
) -> Result<TranscriptionResult, String> {
    let recording = {
        let mut state = APP_STATE.lock().await;
        if let AudioState::Dictating { .. } = state.audio_state {
            state.dictating_stream.take()
        } else {
            None
        }
    };

    let Some(recording) = recording else {
        return Err("当前没有正在进行的录音".to_string());
    };

    // 播放停止声音
    AudioDictating::play_stop_sound();

    // 切换到 Transcribing 状态
    {
        let mut state = APP_STATE.lock().await;
        state.audio_state = AudioState::Transcribing;
    }

    emit_state_and_resize(&app, AudioState::Transcribing, 72.0, 32.0)
        .await
        .map_err(|err| {
            tracing::error!(target = "miaoyu_audio", error = ?err, "切换到转写状态失败");
            err.to_string()
        })?;

    // 完成录音并获取 WAV 数据
    let (wav_data, duration) = recording.finish().map_err(|err| {
        tracing::error!(target = "miaoyu_audio", error = ?err, "完成录音失败");
        err.to_string()
    })?;

    // 验证录音时长
    if let Err(err) = AudioDictating::validate_duration(duration) {
        tracing::warn!(target = "miaoyu_audio", error = ?err, "录音时长不足");

        // 播放通知声音
        dictating::AudioDictating::play_notification_sound();

        // 显示错误反馈
        let app_clone = app.clone();
        tokio::spawn(async move {
            let _ = show_feedback(
                app_clone,
                "录音时间过短，请重试".to_string(),
                FeedbackType::Error,
                None,
            )
            .await;
        });

        reset_to_idle(&app).await;
        return Err(err.to_string());
    }

    // 执行转写
    let result = AudioTranscribing::transcribe(&app, wav_data).await;

    // 处理结果
    match &result {
        Ok(transcription) => {
            tracing::info!(
                target = "miaoyu_audio",
                text = %transcription.text,
                duration_ms = transcription.duration_ms,
                utterance_count = transcription.utterances.len(),
                "语音识别完成"
            );

            // 尝试使用 LLM 润色文本
            let polished_text = match LLMService::polish_text(&app, &transcription.text).await {
                Ok(text) => {
                    tracing::info!(
                        target = "miaoyu_llm",
                        original = %transcription.text,
                        polished = %text,
                        "文本润色成功"
                    );
                    text
                }
                Err(ref error) => {
                    tracing::warn!(
                        target = "miaoyu_llm",
                        error = %error,
                        "文本润色失败，使用原始文本"
                    );

                    // 播放通知声音
                    dictating::AudioDictating::play_notification_sound();

                    // 显示错误反馈
                    let app_clone = app.clone();
                    let error_msg = get_user_friendly_error_message(error);
                    tokio::spawn(async move {
                        let _ = show_feedback(
                            app_clone,
                            format!("文本润色失败: {}", error_msg),
                            FeedbackType::Error,
                            None,
                        )
                        .await;
                    });

                    transcription.text.clone()
                }
            };

            // 自动粘贴到剪贴板（使用润色后的文本）
            if triggered_by_ui {
                // 点击 UI 按钮结束：焦点已转移，无法粘贴到输入框，总是显示 toast
                tracing::info!(
                    target = "miaoyu_audio",
                    "通过 UI 结束录音，内容已复制到剪贴板"
                );

                // 仍然尝试写入剪贴板
                if let Err(error) = clipboard::paste(polished_text, &app) {
                    tracing::warn!(target = "miaoyu_audio", error = %error, "写入剪贴板失败");
                }

                // 播放通知声音
                dictating::AudioDictating::play_notification_sound();

                // 显示 toast
                let app_clone = app.clone();
                tokio::spawn(async move {
                    let _ = show_feedback(
                        app_clone,
                        "识别内容已复制到系统粘贴板".to_string(),
                        FeedbackType::Toast,
                        None,
                    )
                    .await;
                });
            } else {
                // 快捷键结束：焦点未变，根据粘贴结果决定
                match clipboard::paste(polished_text, &app) {
                    Ok(_) => {
                        // 成功粘贴到输入框
                        tracing::info!(target = "miaoyu_audio", "识别内容已自动粘贴");
                    }
                    Err(error) => {
                        // 粘贴失败，但内容已在剪贴板，显示 toast
                        tracing::warn!(target = "miaoyu_audio", error = %error, "自动粘贴失败，内容已复制到剪贴板");

                        // 播放通知声音
                        dictating::AudioDictating::play_notification_sound();

                        let app_clone = app.clone();
                        tokio::spawn(async move {
                            let _ = show_feedback(
                                app_clone,
                                "识别内容已复制到系统粘贴板".to_string(),
                                FeedbackType::Toast,
                                None,
                            )
                            .await;
                        });
                    }
                }
            }
        }
        Err(ref error) => {
            tracing::error!(target = "miaoyu_audio", error = %error, "语音识别失败");

            // 播放通知声音
            dictating::AudioDictating::play_notification_sound();

            // 显示错误反馈
            let app_clone = app.clone();
            let error_msg = get_user_friendly_error_message(error);
            tokio::spawn(async move {
                let _ = show_feedback(app_clone, error_msg, FeedbackType::Error, None).await;
            });
        }
    }

    // 恢复到 Idle 状态
    reset_to_idle(&app).await;

    result.map_err(|err| err.to_string())
}

async fn reset_to_idle(app: &AppHandle<Wry>) {
    hotkeys::set_escape_shortcut_enabled(app, false);

    let mut state = APP_STATE.lock().await;
    state.audio_state = AudioState::Idle;
    state.dictating_stream = None;

    if let Err(err) = emit_state_and_resize(app, AudioState::Idle, 40.0, 8.0).await {
        tracing::error!(target = "miaoyu_audio", error = ?err, "恢复到 Idle 状态失败");
    }
}

fn close_window(app: &AppHandle<Wry>, id: AppWindowId) {
    if let Some(window) = id.get(app) {
        if let Err(error) = window.close() {
            tracing::warn!(
                target = "miaoyu_audio",
                error = ?error,
                window = %id.to_string(),
                "关闭窗口失败"
            );
        }
    }
}
