mod audio;
mod clipboard;
mod history;
mod hotkeys;
mod llm;
mod models;
mod notification;
mod permissions;
mod settings;
mod tray;
mod windows;

use crate::audio::{
    cancel_dictating, dictating::DictatingStream, download_offline_models,
    get_offline_models_status, start_dictating, start_voice_diary, stop_dictating,
};
use crate::history::HistoryKind;
use crate::settings::SettingsStore;
use crate::windows::{AppWindowId, ShowAppWindow};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{Manager, WindowEvent};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Clone, Serialize, Deserialize, Type, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AudioState {
    Idle,
    Recording,
    Transcribing,
}

pub struct AudioRuntimeState {
    pub state: AudioState,
    pub dictating_stream: Option<DictatingStream>,
    pub history_kind: HistoryKind,
}

pub struct AppState {
    pub audio: AsyncMutex<AudioRuntimeState>,
    pub pending_navigation: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            audio: AsyncMutex::new(AudioRuntimeState {
                state: AudioState::Idle,
                dictating_stream: None,
                history_kind: HistoryKind::Dictation,
            }),
            pending_navigation: Mutex::new(None),
        }
    }
}

/// 检查是否已配置 API 密钥
/// 返回 true 表示需要配置
fn check_api_config(app: &tauri::AppHandle) -> bool {
    !llm::has_configured_api_key(app)
}

pub type EnvFilteredRegistry =
    tracing_subscriber::layer::Layered<tracing_subscriber::EnvFilter, tracing_subscriber::Registry>;

pub type FilteredRegistry = tracing_subscriber::layer::Layered<
    tracing_subscriber::filter::FilterFn<fn(m: &tracing::Metadata) -> bool>,
    EnvFilteredRegistry,
>;

pub type DynLoggingLayer = Box<dyn tracing_subscriber::Layer<FilteredRegistry> + Send + Sync>;
pub type LoggingHandle =
    tracing_subscriber::reload::Handle<Option<DynLoggingLayer>, FilteredRegistry>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run(_logging_handle: LoggingHandle) {
    // 加载 .env 文件（如果存在）
    dotenvy::dotenv().ok();

    let tauri_context = tauri::generate_context!();

    let specta_builder = tauri_specta::Builder::new()
        .commands(tauri_specta::collect_commands![
            windows::set_theme,
            windows::take_pending_navigation,
            permissions::request_permission,
            permissions::check_os_permissions,
            permissions::open_permission_settings,
            hotkeys::set_hotkey,
            start_dictating,
            start_voice_diary,
            cancel_dictating,
            stop_dictating,
            notification::show_notification,
            notification::hide_notification,
            settings::get_autostart_enabled,
            settings::set_autostart_enabled,
            settings::set_onboarding_completed,
            llm::test_llm_api_key,
            models::get_supported_models,
            models::get_models_store,
            models::set_active_text_model,
            models::update_text_model_credentials,
            models::set_active_asr_model,
            models::update_asr_credentials,
            get_offline_models_status,
            download_offline_models,
            history::list_history_entries,
            history::add_history_entry,
            history::delete_history_entry,
            history::clear_history_entries,
            history::get_history_stats,
            history::load_history_audio,
        ])
        .events(tauri_specta::collect_events![
            hotkeys::OnEscapePress,
            notification::ShowNotification,
            audio::OnTranscribingStage,
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .typ::<hotkeys::HotkeysStore>()
        .typ::<SettingsStore>()
        .typ::<AudioState>();

    #[cfg(debug_assertions)]
    specta_builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/lib/tauri.ts",
        )
        .expect("Failed to export typescript bindings");

    tauri::async_runtime::set(tokio::runtime::Handle::current());

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    let resume_flag = Arc::new(AtomicBool::new(false));
    let resume_flag_run = Arc::clone(&resume_flag);

    builder
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--flag1", "--flag2"]),
        ))
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            let app_handle = app.handle().clone();

            specta_builder.mount_events(&app_handle);
            hotkeys::init(&app_handle);
            settings::init(&app_handle);
            let onboarding_completed = settings::is_onboarding_completed(&app_handle);
            if onboarding_completed {
                tray::create_tray(&app_handle).ok();
            }
            let permissions = permissions::check_os_permissions(false);

            tokio::spawn({
                let app = app_handle.clone();
                async move {
                    let onboarding_completed = settings::is_onboarding_completed(&app);
                    let permissions_ready = permissions.microphone.permitted()
                        && permissions.accessibility.permitted();
                    if onboarding_completed && permissions_ready {
                        let _ = ShowAppWindow::Dashboard.show(&app).await;
                    } else {
                        let _ = ShowAppWindow::Onboarding.show(&app).await;
                    }

                    // 检查是否已配置 API 密钥（仅用于日志记录）
                    let _needs_config = check_api_config(&app);
                    // 有编译时默认值，不需要提示

                    let _ = windows::sync_audio_overlay(&app, AudioState::Idle).await;

                    // Start observing screen changes (Dock show/hide) to reposition windows
                    windows::start_screen_observer(app.clone());

                    // 自动检查更新
                    match app.updater() {
                        Ok(updater) => {
                            if let Err(error) = updater.check().await {
                                tracing::debug!(
                                    target = "miaoyu_updater",
                                    error = %error,
                                    "自动检查更新失败"
                                );
                            }
                        }
                        Err(error) => {
                            tracing::debug!(
                                target = "miaoyu_updater",
                                error = %error,
                                "获取更新器实例失败"
                            );
                        }
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label();
            let app = window.app_handle();

            if let WindowEvent::Destroyed = event {
                if let Ok(AppWindowId::Settings) = AppWindowId::from_str(label) {
                    for (label, window) in app.webview_windows() {
                        if let Ok(_id) = AppWindowId::from_str(&label) {
                            let _ = window.show();
                        }
                    }
                }
            }

            #[cfg(target_os = "macos")]
            if let WindowEvent::Focused(focused) = event {
                if *focused {
                    if let Ok(window_id) = AppWindowId::from_str(label) {
                        if window_id.activates_dock() {
                            app.set_activation_policy(tauri::ActivationPolicy::Regular)
                                .ok();
                        }
                    }
                }
            }
        })
        .build(tauri_context)
        .expect("error while running tauri application")
        .run(move |handle, event| match event {
            tauri::RunEvent::Resumed { .. } => {
                if !resume_flag_run.swap(true, Ordering::SeqCst) {
                    let app = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if settings::is_onboarding_completed(&app) {
                            let _ = ShowAppWindow::Dashboard.show(&app).await;
                        } else {
                            let _ = ShowAppWindow::Onboarding.show(&app).await;
                        }
                    });
                }
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                let app = handle.clone();
                tauri::async_runtime::spawn(async move {
                    if settings::is_onboarding_completed(&app) {
                        let _ = ShowAppWindow::Dashboard.show(&app).await;
                    } else {
                        let _ = ShowAppWindow::Onboarding.show(&app).await;
                    }
                });
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::ExitRequested { code, api, .. } => {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
            _ => {}
        });
}
