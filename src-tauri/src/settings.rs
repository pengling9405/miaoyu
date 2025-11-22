use serde::{Deserialize, Serialize};
use serde_json::json;
use specta::Type;
use tauri::{AppHandle, Wry};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_store::StoreExt;
use tracing::error;

use crate::{llm::DEFAULT_SYSTEM_PROMPT, tray, windows::ShowAppWindow};

#[derive(Serialize, Deserialize, Type, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SettingsStore {
    #[serde(default)]
    pub theme: AppTheme,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_system_prompt: Option<String>,
    #[serde(default)]
    pub autostart_enabled: bool,
    #[serde(default)]
    pub onboarding_completed: bool,
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self {
            theme: AppTheme::System,
            llm_system_prompt: Some(DEFAULT_SYSTEM_PROMPT.to_string()),
            autostart_enabled: false,
            onboarding_completed: false,
        }
    }
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AppTheme {
    #[default]
    System,
    Light,
    Dark,
}

impl SettingsStore {
    pub fn get(app: &AppHandle<Wry>) -> Result<Option<Self>, String> {
        match app.store("store").map(|s| s.get("settings")) {
            Ok(Some(store)) => match serde_json::from_value(store) {
                Ok(settings) => Ok(Some(settings)),
                Err(e) => Err(format!("Failed to deserialize general settings store: {e}")),
            },
            _ => Ok(None),
        }
    }

    fn save(&self, app: &AppHandle) -> Result<(), String> {
        let store = match app.store("store") {
            Ok(store) => store,
            Err(_) => return Err("Store not found".to_string()),
        };

        store.set("settings", json!(self));
        store.save().map_err(|e| e.to_string())
    }
}

pub fn init(app: &AppHandle) {
    let store = match SettingsStore::get(app) {
        Ok(Some(store)) => store,
        Ok(None) => SettingsStore::default(),
        Err(e) => {
            error!("Failed to deserialize general settings store: {}", e);
            SettingsStore::default()
        }
    };
    store.save(app).unwrap();
}

/// Onboarding 已完成
#[tauri::command]
#[specta::specta]
pub fn set_onboarding_completed(app: AppHandle, completed: bool) -> Result<bool, String> {
    let mut settings = SettingsStore::get(&app).ok().flatten().unwrap_or_default();
    settings.onboarding_completed = completed;
    settings.save(&app)?;
    if completed {
        if app.tray_by_id("tray").is_none() {
            let _ = tray::create_tray(&app);
        }
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = ShowAppWindow::Dashboard.show(&app_handle).await;
        });
    }
    Ok(settings.onboarding_completed)
}

pub fn is_onboarding_completed(app: &AppHandle<Wry>) -> bool {
    SettingsStore::get(app)
        .ok()
        .flatten()
        .unwrap_or_default()
        .onboarding_completed
}

/// 获取开机自启动状态
#[tauri::command]
#[specta::specta]
pub fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    let manager = app.autolaunch();
    manager
        .is_enabled()
        .map_err(|e| format!("Failed to get autostart status: {}", e))
}

/// 设置开机自启动
#[tauri::command]
#[specta::specta]
pub fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();

    if enabled {
        manager
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
    } else {
        manager
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {}", e))?;
    }

    // 保存到设置
    let mut settings = SettingsStore::get(&app).ok().flatten().unwrap_or_default();
    settings.autostart_enabled = enabled;
    settings.save(&app)?;

    Ok(())
}

// Doubao 相关设置已移除
