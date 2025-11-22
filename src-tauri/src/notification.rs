use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Wry};
use tauri_specta::Event;
use tokio::task::JoinHandle;

use crate::{audio::dictating::AudioDictating, windows::AppWindowId, AppState, AudioState};

// 全局状态：跟踪自动隐藏任务
static AUTO_HIDE_TASK: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));
const NOTIFICATION_BOTTOM_MARGIN: i32 = 40;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotificationType {
    Error,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ShowNotification {
    pub message: String,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
}

/// 显示居于屏幕底部的全局 Notification，并在数秒后自动关闭
#[tauri::command(async)]
#[specta::specta]
pub async fn show_notification(
    app: AppHandle<Wry>,
    message: String,
    notification_type: NotificationType,
    offset_x: Option<f64>,
) -> Result<(), String> {
    tracing::debug!(
        target = "miaoyu_notification",
        message = %message,
        notification_type = ?notification_type,
        "显示通知消息"
    );

    // 所有通知类型都播放提示音
    match tokio::task::spawn_blocking(AudioDictating::play_notification_sound).await {
        Ok(Ok(())) => {}
        Ok(Err(play_error)) => {
            tracing::warn!(
                target = "miaoyu_notification",
                error = %play_error,
                "播放通知音效失败"
            );
        }
        Err(join_error) => {
            tracing::warn!(
                target = "miaoyu_notification",
                error = %join_error,
                "播放通知音效任务失败"
            );
        }
    };

    // 获取或创建 Notification 窗口
    let notification_window = match AppWindowId::Notification.get(&app) {
        Some(window) => window,
        None => crate::windows::ShowAppWindow::Notification
            .show(&app)
            .await
            .map_err(|e| e.to_string())?,
    };

    // 发送消息事件给前端
    ShowNotification {
        message: message.clone(),
        notification_type: notification_type.clone(),
    }
    .emit(&app)
    .map_err(|e| e.to_string())?;

    // 等待一小段时间，确保窗口可用
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 重新定位
    if let Err(e) = position_notification_window(&app, offset_x).await {
        tracing::warn!(
            target = "miaoyu_notification",
            error = %e,
            "重新定位通知窗口失败"
        );
    }

    notification_window.show().map_err(|e| e.to_string())?;

    // 取消之前的自动隐藏任务（如果存在）
    {
        let mut task = AUTO_HIDE_TASK.lock().unwrap();
        if let Some(old_task) = task.take() {
            old_task.abort();
        }

        // 根据通知类型设置显示时长
        let duration_ms = match notification_type {
            NotificationType::Info => 2500,
            NotificationType::Error => 3000,
        };

        // 创建自动隐藏任务
        let app_clone = app.clone();
        let new_task = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;

            if let Some(window) = AppWindowId::Notification.get(&app_clone) {
                if let Err(e) = window.hide() {
                    tracing::warn!(
                        target = "miaoyu_notification",
                        error = ?e,
                        "隐藏通知窗口失败"
                    );
                }
            }
        });

        *task = Some(new_task);
    }

    Ok(())
}

/// 隐藏通知窗口
#[tauri::command]
#[specta::specta]
pub fn hide_notification(app: AppHandle<Wry>) -> Result<(), String> {
    {
        let mut task = AUTO_HIDE_TASK.lock().unwrap();
        if let Some(old_task) = task.take() {
            old_task.abort();
        }
    }

    if let Some(window) = AppWindowId::Notification.get(&app) {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 定位 Notification 窗口到屏幕底部
async fn position_notification_window(
    app: &AppHandle<Wry>,
    offset_x: Option<f64>,
) -> Result<(), String> {
    // 确定当前音频状态，以选取合适的参考窗口
    let app_state = app.state::<AppState>();
    let current_state = {
        let guard = app_state.audio.lock().await;
        guard.state.clone()
    };
    let anchor_id = match current_state {
        AudioState::Idle => AppWindowId::Dashboard,
        AudioState::Recording => AppWindowId::AudioRecording,
        AudioState::Transcribing => AppWindowId::AudioTranscribing,
    };

    let anchor_window = anchor_id.get(app).or_else(|| {
        [
            AppWindowId::AudioRecording,
            AppWindowId::AudioTranscribing,
            AppWindowId::Dashboard,
        ]
        .into_iter()
        .find_map(|id| id.get(app))
    });

    let notification_window = AppWindowId::Notification
        .get(app)
        .ok_or("Notification 窗口不存在")?;

    let notification_size = notification_window
        .outer_size()
        .map_err(|e| e.to_string())?;
    let scale_factor = notification_window
        .scale_factor()
        .map_err(|e| e.to_string())?;

    // 选择用于定位的显示器（优先使用参考窗口所在显示器）
    let monitor = anchor_window
        .as_ref()
        .and_then(|window| window.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());

    let (monitor_pos, monitor_size) =
        monitor
            .map(|m| (*m.position(), *m.size()))
            .unwrap_or_else(|| {
                tracing::warn!(
                    target = "miaoyu_notification",
                    "无法获取显示器信息，使用默认位置显示通知"
                );
                (
                    tauri::PhysicalPosition::new(0, 0),
                    tauri::PhysicalSize::new(1920, 1080),
                )
            });

    let offset_physical = offset_x
        .map(|offset| (offset * scale_factor) as i32)
        .unwrap_or_default();

    let notification_x = monitor_pos.x
        + (monitor_size.width as i32 - notification_size.width as i32) / 2
        + offset_physical;

    #[cfg(target_os = "macos")]
    let notification_y = {
        use cidre::ns;

        if let Some(screen) = ns::Screen::main() {
            let frame = screen.frame();
            let visible = screen.visible_frame();
            let visible_bottom_from_top = (frame.size.height - visible.origin.y) * scale_factor;
            (visible_bottom_from_top as i32)
                - notification_size.height as i32
                - NOTIFICATION_BOTTOM_MARGIN
        } else {
            monitor_pos.y + monitor_size.height as i32
                - notification_size.height as i32
                - NOTIFICATION_BOTTOM_MARGIN
        }
    };

    #[cfg(not(target_os = "macos"))]
    let notification_y = monitor_pos.y + monitor_size.height as i32
        - notification_size.height as i32
        - NOTIFICATION_BOTTOM_MARGIN;

    let position = tauri::PhysicalPosition::new(notification_x, notification_y);
    notification_window
        .set_position(position)
        .map_err(|e| e.to_string())?;

    tracing::debug!(
        target = "miaoyu_notification",
        monitor_x = monitor_pos.x,
        monitor_y = monitor_pos.y,
        monitor_width = monitor_size.width,
        monitor_height = monitor_size.height,
        scale_factor = scale_factor,
        offset_x_logical = ?offset_x,
        offset_x_physical = offset_physical,
        notification_x = notification_x,
        notification_y = notification_y,
        notification_width = notification_size.width,
        notification_height = notification_size.height,
        "通知窗口已重定位为底部 Notification"
    );

    Ok(())
}
