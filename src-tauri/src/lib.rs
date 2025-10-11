mod audio;
mod clipboard;
mod feedback;
mod hotkeys;
mod llm;
#[cfg(target_os = "macos")]
mod mouse_tracker;

#[cfg(not(target_os = "macos"))]
mod mouse_tracker {
    use tauri::{AppHandle, Wry};

    pub fn start_mouse_tracking(_app: AppHandle<Wry>) {}
}
mod permissions;
mod settings;
mod tray;
mod windows;

use crate::audio::{cancel_dictating, start_dictating, stop_dictating, AudioState};
use crate::settings::SettingsStore;
use crate::windows::{AppWindowId, ShowAppWindow};
use std::str::FromStr;
use tauri::{Manager, WindowEvent};

/// 检查是否已配置 API 密钥
/// 返回 true 表示需要配置
fn check_api_config(app: &tauri::AppHandle) -> bool {
    let settings = SettingsStore::get(app).ok().flatten();

    // 检查 ASR 配置（用户设置或环境变量）
    let has_asr = settings
        .as_ref()
        .and_then(|s| s.asr_app_id.as_ref())
        .is_some()
        || std::env::var("VOLCENGINE_APP_ID").is_ok();

    // 检查 LLM 配置（用户设置或环境变量）
    let has_llm = settings
        .as_ref()
        .and_then(|s| s.llm_api_key.as_ref())
        .is_some()
        || std::env::var("DEEPSEEK_API_KEY").is_ok();

    // 如果两者都没配置，返回 true（需要配置）
    !has_asr || !has_llm
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
            windows::resize_audio_panel,
            permissions::request_permission,
            permissions::check_os_permissions,
            permissions::open_permission_settings,
            hotkeys::set_hotkey,
            start_dictating,
            cancel_dictating,
            stop_dictating,
            feedback::show_feedback,
            feedback::hide_feedback,
            settings::get_autostart_enabled,
            settings::set_autostart_enabled,
        ])
        .events(tauri_specta::collect_events![
            hotkeys::OnEscapePress,
            settings::AudioFlowPanelPositionChanged,
            feedback::ShowFeedback,
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .typ::<hotkeys::HotkeysStore>()
        .typ::<settings::SettingsStore>()
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

    builder
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
            settings::register_listeners(&app_handle);
            let permissions = permissions::check_os_permissions(false);

            tokio::spawn({
                let app = app_handle.clone();
                async move {
                    // 检查权限
                    if !permissions.microphone.permitted() || !permissions.accessibility.permitted()
                    {
                        let _ = ShowAppWindow::Setup.show(&app).await;
                    }

                    // 检查是否已配置 API 密钥（仅用于日志记录）
                    let _needs_config = check_api_config(&app);
                    // 有编译时默认值，不需要提示

                    let _ = ShowAppWindow::Main.show(&app).await;

                    // Start global mouse tracking for hover detection (works without focus)
                    mouse_tracker::start_mouse_tracking(app.clone());

                    // Start observing screen changes (Dock show/hide) to reposition windows
                    windows::start_screen_observer(app.clone());
                }
            });

            tray::create_tray(&app_handle).unwrap();

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
        .run(move |_handle, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::ExitRequested { code, api, .. } => {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
            _ => {}
        });
}
