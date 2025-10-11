use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;
use tauri::{AppHandle, Wry};
use tauri_specta::Event;
use tokio::task::JoinHandle;

use crate::{
    settings::{AudioFlowPanelPosition, SettingsStore},
    windows::AppWindowId,
};

// 全局状态：跟踪自动隐藏任务
static AUTO_HIDE_TASK: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FeedbackType {
    Tooltip,
    Error,
    Toast,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ShowFeedback {
    pub message: String,
    #[serde(rename = "type")]
    pub feedback_type: FeedbackType,
}

/// 显示反馈消息
///
/// 在 Main 窗口上方或下方显示反馈信息，3 秒后自动关闭
#[tauri::command(async)]
#[specta::specta]
pub async fn show_feedback(
    app: AppHandle<Wry>,
    message: String,
    feedback_type: FeedbackType,
    offset_x: Option<f64>,
) -> Result<(), String> {
    tracing::debug!(
        target = "miaoyu_feedback",
        message = %message,
        feedback_type = ?feedback_type,
        "显示反馈消息"
    );

    // 获取或创建 Feedback 窗口
    let feedback_window = match AppWindowId::Feedback.get(&app) {
        Some(window) => window,
        None => {
            // 窗口不存在，需要先创建
            crate::windows::ShowAppWindow::Feedback
                .show(&app)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    // 发送消息事件给前端
    ShowFeedback {
        message: message.clone(),
        feedback_type: feedback_type.clone(),
    }
    .emit(&app)
    .map_err(|e| e.to_string())?;

    // 等待一小段时间，确保 Main 窗口的 resize 动画完成
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 重新定位窗口（每次都重新计算，因为 Main 窗口可能移动了）
    if let Err(e) = position_feedback_window(&app, offset_x).await {
        tracing::warn!(
            target = "miaoyu_feedback",
            error = %e,
            "重新定位反馈窗口失败"
        );
    }

    // 显示窗口
    feedback_window.show().map_err(|e| e.to_string())?;

    // 取消之前的自动隐藏任务（如果存在）
    {
        let mut task = AUTO_HIDE_TASK.lock().unwrap();
        if let Some(old_task) = task.take() {
            old_task.abort();
        }

        // 根据反馈类型设置不同的显示时长
        let duration_ms = match feedback_type {
            FeedbackType::Tooltip => 1200, // tooltip: 1.2 秒
            FeedbackType::Toast => 2500,   // toast: 2.5 秒
            FeedbackType::Error => 3000,   // error: 3 秒
        };

        // 创建自动隐藏任务
        let app_clone = app.clone();
        let new_task = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;

            if let Some(window) = AppWindowId::Feedback.get(&app_clone) {
                if let Err(e) = window.hide() {
                    tracing::warn!(
                        target = "miaoyu_feedback",
                        error = ?e,
                        "隐藏反馈窗口失败"
                    );
                }
            }
        });

        *task = Some(new_task);
    }

    Ok(())
}

/// 隐藏反馈窗口
#[tauri::command]
#[specta::specta]
pub fn hide_feedback(app: AppHandle<Wry>) -> Result<(), String> {
    // 取消自动隐藏任务
    {
        let mut task = AUTO_HIDE_TASK.lock().unwrap();
        if let Some(old_task) = task.take() {
            old_task.abort();
        }
    }

    // 隐藏窗口
    if let Some(window) = AppWindowId::Feedback.get(&app) {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 定位反馈窗口到 Main 窗口上方或下方
async fn position_feedback_window(
    app: &AppHandle<Wry>,
    offset_x: Option<f64>,
) -> Result<(), String> {
    // 获取 Main 窗口
    let main_window = AppWindowId::Main.get(app).ok_or("Main 窗口不存在")?;

    // 获取 Feedback 窗口
    let feedback_window = AppWindowId::Feedback
        .get(app)
        .ok_or("Feedback 窗口不存在")?;

    // 获取 Main 窗口位置和大小
    let main_pos = main_window.outer_position().map_err(|e| e.to_string())?;
    let main_size = main_window.outer_size().map_err(|e| e.to_string())?;

    // 获取 Feedback 窗口大小
    let feedback_size = feedback_window.outer_size().map_err(|e| e.to_string())?;

    // 获取缩放因子
    let scale_factor = main_window.scale_factor().map_err(|e| e.to_string())?;

    // 获取面板位置配置
    let panel_position = SettingsStore::get(app)
        .ok()
        .flatten()
        .map(|s| s.audio_flow_panel_position)
        .unwrap_or(AudioFlowPanelPosition::BottomCenter);

    // 计算 Feedback 窗口位置
    let margin = 4; // 减小间距，让 feedback 更贴近 Main 窗口
    let feedback_y = match panel_position {
        AudioFlowPanelPosition::BottomCenter => {
            // Main 在底部，Feedback 在上方
            main_pos.y - feedback_size.height as i32 - margin
        }
        AudioFlowPanelPosition::TopCenter => {
            // Main 在顶部，Feedback 在下方
            main_pos.y + main_size.height as i32 + margin
        }
    };

    // 水平居中
    let feedback_x = if let Some(offset) = offset_x {
        // 如果提供了 offset_x，则以该位置为中心
        // offset_x 是 CSS 像素，需要转换为物理像素
        let offset_physical = (offset * scale_factor) as i32;
        main_pos.x + offset_physical - (feedback_size.width as i32 / 2)
    } else {
        // 否则居中于 Main 窗口
        main_pos.x + (main_size.width as i32 - feedback_size.width as i32) / 2
    };

    // 设置位置
    let position = tauri::PhysicalPosition::new(feedback_x, feedback_y);
    feedback_window
        .set_position(position)
        .map_err(|e| e.to_string())?;

    tracing::debug!(
        target = "miaoyu_feedback",
        main_x = main_pos.x,
        main_y = main_pos.y,
        main_width = main_size.width,
        main_height = main_size.height,
        scale_factor = scale_factor,
        offset_x_logical = ?offset_x,
        offset_x_physical = offset_x.map(|o| (o * scale_factor) as i32),
        feedback_x = feedback_x,
        feedback_y = feedback_y,
        feedback_width = feedback_size.width,
        panel_position = ?panel_position,
        "反馈窗口已定位"
    );

    Ok(())
}
