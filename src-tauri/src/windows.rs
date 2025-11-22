use anyhow::anyhow;
#[cfg(target_os = "macos")]
use cidre::ns;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
use serde::Deserialize;
use specta::Type;
use std::{path::PathBuf, str::FromStr};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Wry,
};
use tracing::warn;

use crate::{settings::AppTheme, AppState, AudioState};

const AUDIO_BAR_BOTTOM_MARGIN: i32 = 40;

#[derive(Clone, Deserialize, Type, PartialEq, Eq)]
pub enum AppWindowId {
    Notification,
    Settings,
    Dashboard,
    Onboarding,
    AudioRecording,
    AudioTranscribing,
}

impl FromStr for AppWindowId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "settings" => Self::Settings,
            "notification" => Self::Notification,
            "dashboard" => Self::Dashboard,
            "onboarding" => Self::Onboarding,
            "recording" => Self::AudioRecording,
            "transcribing" => Self::AudioTranscribing,
            _ => return Err(format!("unknown window label: {s}")),
        })
    }
}

impl std::fmt::Display for AppWindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Settings => write!(f, "settings"),
            Self::Notification => write!(f, "notification"),
            Self::Dashboard => write!(f, "dashboard"),
            Self::Onboarding => write!(f, "onboarding"),
            Self::AudioRecording => write!(f, "recording"),
            Self::AudioTranscribing => write!(f, "transcribing"),
        }
    }
}

impl AppWindowId {
    pub fn label(&self) -> String {
        self.to_string()
    }

    pub fn get(&self, app: &AppHandle<Wry>) -> Option<WebviewWindow> {
        let label = self.label();
        app.get_webview_window(&label)
    }

    pub fn activates_dock(&self) -> bool {
        matches!(self, Self::Onboarding | Self::Settings | Self::Dashboard)
    }
}

#[derive(Clone, Type, Deserialize)]
pub enum ShowAppWindow {
    Settings,
    Notification,
    Dashboard,
    Onboarding,
    AudioRecording,
    AudioTranscribing,
}

impl ShowAppWindow {
    pub async fn show(&self, app: &AppHandle<Wry>) -> tauri::Result<WebviewWindow> {
        let should_recreate = matches!(self, Self::AudioRecording);
        if let Some(window) = self.id(app).get(app) {
            if should_recreate {
                window.destroy().ok();
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            } else {
                if matches!(self, Self::AudioRecording | Self::AudioTranscribing) {
                    #[cfg(target_os = "macos")]
                    ensure_overlay_in_active_space(&window);
                    if let Err(error) = reposition_audio_bar_with_monitor(&window) {
                        warn!(
                            target = "miaoyu_audio",
                            window = %self.id(app),
                            ?error,
                            "Failed to reposition existing audio overlay"
                        );
                    }
                }
                window.show().ok();
                window.set_focus().ok();
                return Ok(window);
            }
        }

        let _id = self.id(app);

        let window = match self {
            Self::Onboarding => {
                let window = self
                    .window_builder(app, "/onboarding")
                    .resizable(false)
                    .maximized(false)
                    .center()
                    .focused(true)
                    .inner_size(960.0, 720.0)
                    .maximizable(false)
                    .shadow(true)
                    .build()?;

                window.show()?;
                window
            }
            Self::Settings => {
                let window = self
                    .window_builder(app, "/settings")
                    .resizable(false)
                    .maximized(false)
                    .center()
                    .focused(true)
                    .inner_size(425.0, 400.0)
                    .maximizable(false)
                    .shadow(true)
                    .build()?;

                window.show()?;
                window
            }
            Self::Dashboard => {
                #[cfg(target_os = "macos")]
                let window = {
                    self.window_builder(app, "/")
                        .inner_size(1400.0, 1200.0)
                        .min_inner_size(1400.0, 1200.0)
                        .resizable(true)
                        .maximized(false)
                        .center()
                        .focused(true)
                        .decorations(true)
                        .transparent(false)
                        .maximizable(true)
                        .shadow(true)
                        .visible(false)
                        .hidden_title(true)
                        .title("")
                        .build()?
                };

                #[cfg(not(target_os = "macos"))]
                let window = self
                    .window_builder(app, "/")
                    .inner_size(1400.0, 1200.0)
                    .min_inner_size(1400.0, 1200.0)
                    .resizable(true)
                    .maximized(false)
                    .center()
                    .focused(true)
                    .decorations(true)
                    .transparent(false)
                    .maximizable(true)
                    .shadow(true)
                    .visible(false)
                    .title("")
                    .build()?;

                window.show()?;
                window
            }
            Self::Notification => {
                #[cfg(target_os = "macos")]
                let window = {
                    use tauri::TitleBarStyle;
                    let window = self
                        .window_builder(app, "/notification")
                        .inner_size(400.0, 96.0)
                        .resizable(false)
                        .maximized(false)
                        .decorations(false)
                        .transparent(true)
                        .focused(false)
                        .maximizable(false)
                        .shadow(false)
                        .visible(false)
                        .always_on_top(true)
                        .title_bar_style(TitleBarStyle::Overlay)
                        .hidden_title(true)
                        .build()?;
                    window
                };

                #[cfg(not(target_os = "macos"))]
                let window = self
                    .window_builder(app, "/notification")
                    .inner_size(400.0, 96.0)
                    .resizable(false)
                    .maximized(false)
                    .decorations(false)
                    .transparent(true)
                    .focused(false)
                    .maximizable(false)
                    .shadow(false)
                    .visible(false)
                    .always_on_top(true)
                    .build()?;

                // Notification 窗口不立即显示，由 notification 模块控制
                window
            }
            Self::AudioRecording => self.build_audio_overlay(app, "/recording", 120.0, 32.0)?,
            Self::AudioTranscribing => {
                self.build_audio_overlay(app, "/transcribing", 120.0, 32.0)?
            }
        };

        Ok(window)
    }

    fn build_audio_overlay(
        &self,
        app: &AppHandle<Wry>,
        route: &str,
        width: f64,
        height: f64,
    ) -> tauri::Result<WebviewWindow> {
        #[cfg(target_os = "macos")]
        let window = {
            use tauri::TitleBarStyle;
            let window = self
                .window_builder(app, route)
                .inner_size(width, height)
                .resizable(false)
                .maximized(false)
                .decorations(false)
                .transparent(true)
                .focused(false)
                .maximizable(false)
                .shadow(false)
                .visible(false)
                .always_on_top(true)
                .title_bar_style(TitleBarStyle::Overlay)
                .hidden_title(true)
                .visible_on_all_workspaces(true)
                .build()?;

            ensure_overlay_in_active_space(&window);
            window
        };

        #[cfg(not(target_os = "macos"))]
        let window = self
            .window_builder(app, route)
            .inner_size(width, height)
            .resizable(false)
            .maximized(false)
            .decorations(false)
            .transparent(true)
            .focused(false)
            .maximizable(false)
            .shadow(false)
            .visible(false)
            .always_on_top(true)
            .build()?;

        reposition_audio_bar_with_monitor(&window)?;
        window.show()?;
        Ok(window)
    }

    fn window_builder<'a>(
        &'a self,
        app: &'a AppHandle<Wry>,
        url: impl Into<PathBuf>,
    ) -> WebviewWindowBuilder<'a, Wry, AppHandle<Wry>> {
        let id = self.id(app);

        WebviewWindow::builder(app, id.label(), WebviewUrl::App(url.into()))
            .title("")
            .visible(false)
            .accept_first_mouse(true)
            .shadow(true)
    }

    pub fn id(&self, _app: &AppHandle) -> AppWindowId {
        match self {
            ShowAppWindow::Notification => AppWindowId::Notification,
            ShowAppWindow::Settings => AppWindowId::Settings,
            ShowAppWindow::Dashboard => AppWindowId::Dashboard,
            ShowAppWindow::Onboarding => AppWindowId::Onboarding,
            ShowAppWindow::AudioRecording => AppWindowId::AudioRecording,
            ShowAppWindow::AudioTranscribing => AppWindowId::AudioTranscribing,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn set_theme(window: tauri::Window, theme: AppTheme) {
    let _ = window.set_theme(match theme {
        AppTheme::System => None,
        AppTheme::Light => Some(tauri::Theme::Light),
        AppTheme::Dark => Some(tauri::Theme::Dark),
    });
}

#[tauri::command]
#[specta::specta]
pub fn take_pending_navigation(app: AppHandle) -> Option<String> {
    let state = app.state::<AppState>();
    let mut pending = state.pending_navigation.lock().unwrap();
    pending.take()
}

pub fn reposition_audio_bars(app: &AppHandle<Wry>) {
    for id in [AppWindowId::AudioRecording, AppWindowId::AudioTranscribing] {
        if let Some(window) = id.get(app) {
            if let Err(error) = reposition_audio_bar_with_monitor(&window) {
                warn!(
                    target = "miaoyu_audio",
                    window = %id.to_string(),
                    ?error,
                    "Failed to reposition audio overlay window",
                );
            }
        }
    }
}

pub async fn sync_audio_overlay(app: &AppHandle<Wry>, state: AudioState) -> tauri::Result<()> {
    let overlays = [AppWindowId::AudioRecording, AppWindowId::AudioTranscribing];

    let target = match state {
        AudioState::Idle => None,
        AudioState::Recording => Some(ShowAppWindow::AudioRecording),
        AudioState::Transcribing => Some(ShowAppWindow::AudioTranscribing),
    };

    for id in overlays {
        let should_keep = target
            .as_ref()
            .map(|variant| variant.id(app) == id)
            .unwrap_or(false);
        if should_keep {
            continue;
        }

        if let Some(window) = id.get(app) {
            window.hide().ok();
        }
    }

    if let Some(target_variant) = target {
        target_variant.show(app).await?;
    }

    Ok(())
}

fn reposition_audio_bar_with_monitor(window: &WebviewWindow) -> tauri::Result<()> {
    let app = window.app_handle();
    let monitor = app
        .primary_monitor()?
        .ok_or_else(|| tauri::Error::Anyhow(anyhow!("Failed to get primary monitor")))?;

    let pos = monitor.position();
    let size = monitor.size();
    let logical_pos = PhysicalPosition::new(pos.x, pos.y);
    let logical_size = PhysicalSize::new(size.width, size.height);

    position_audio_bar(window, logical_pos, logical_size)
}

#[cfg(target_os = "macos")]
fn ensure_overlay_in_active_space(window: &WebviewWindow) {
    let window_clone = window.clone();
    window
        .run_on_main_thread(move || {
            let result = ns::try_catch(|| unsafe {
                if let Ok(ns_window_ptr) = window_clone.ns_window() {
                    let ns_window = &*(ns_window_ptr as *mut NSWindow);
                    let mut behavior = ns_window.collectionBehavior();
                    behavior.remove(NSWindowCollectionBehavior::CanJoinAllSpaces);
                    behavior.insert(NSWindowCollectionBehavior::MoveToActiveSpace);
                    ns_window.setCollectionBehavior(behavior);
                }
            });

            if let Err(error) = result {
                warn!(
                    target = "miaoyu_audio",
                    reason = ?error.reason(),
                    "Failed to update overlay space behavior"
                );
            }
        })
        .ok();
}

#[cfg(not(target_os = "macos"))]
fn ensure_overlay_in_active_space(_window: &WebviewWindow) {}

#[cfg(target_os = "macos")]
fn position_audio_bar(
    window: &WebviewWindow,
    monitor_pos: PhysicalPosition<i32>,
    monitor_size: tauri::PhysicalSize<u32>,
) -> tauri::Result<()> {
    let window_size = window.outer_size()?;
    let margin = AUDIO_BAR_BOTTOM_MARGIN;
    let scale_factor = window.scale_factor()?;

    let centered_x = monitor_pos.x + (monitor_size.width as i32 - window_size.width as i32) / 2;

    // Use visible frame if available (excludes Dock area)
    let y = if let Some(screen) = ns::Screen::main() {
        let visible = screen.visible_frame();
        let frame = screen.frame();

        // Convert NSScreen coordinates (origin: bottom-left) to Tauri coordinates (origin: top-left)
        // visible.origin.y is the distance from screen bottom to visible area bottom
        let visible_bottom_from_top = (frame.size.height - visible.origin.y) * scale_factor;

        tracing::debug!(
            target: "miaoyu_audio",
            visible_bottom = visible_bottom_from_top,
            frame_height = frame.size.height,
            visible_y = visible.origin.y,
            visible_height = visible.size.height,
            scale = scale_factor,
            "Positioning with Dock awareness"
        );

        // Position above the visible frame bottom (above Dock if visible)
        (visible_bottom_from_top as i32) - window_size.height as i32 - margin
    } else {
        // Fallback: use monitor size (screen bottom)
        monitor_pos.y + monitor_size.height as i32 - window_size.height as i32 - margin
    };

    let target_position = PhysicalPosition::new(centered_x, y);
    let window_clone = window.clone();
    window
        .run_on_main_thread(move || {
            let _ = window_clone.set_position(target_position);
        })
        .ok();

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn position_audio_bar(
    window: &WebviewWindow,
    monitor_pos: PhysicalPosition<i32>,
    monitor_size: tauri::PhysicalSize<u32>,
) -> tauri::Result<()> {
    let window_size = window.outer_size()?;
    let margin = AUDIO_BAR_BOTTOM_MARGIN;

    let centered_x = monitor_pos.x + (monitor_size.width as i32 - window_size.width as i32) / 2;

    let y = monitor_pos.y + monitor_size.height as i32 - window_size.height as i32 - margin;

    let target_position = PhysicalPosition::new(centered_x, y);
    let window_clone = window.clone();
    window
        .run_on_main_thread(move || {
            let _ = window_clone.set_position(target_position);
        })
        .ok();

    Ok(())
}

/// Start observing screen parameter changes (Dock show/hide, etc.)
/// and automatically reposition audio bars when changes are detected
#[cfg(target_os = "macos")]
pub fn start_screen_observer(app: AppHandle<Wry>) {
    tracing::info!(
        target: "miaoyu_audio",
        "Starting screen parameter observer for Dock changes"
    );

    tokio::spawn(async move {
        use tokio::time::{interval, Duration};

        let mut check_interval = interval(Duration::from_millis(500));
        let mut last_visible_height: Option<f64> = None;

        loop {
            check_interval.tick().await;

            // Check if visible frame height has changed (indicates Dock show/hide)
            if let Some(screen) = ns::Screen::main() {
                let visible = screen.visible_frame();
                let current_height = visible.size.height;

                if let Some(last_height) = last_visible_height {
                    if (current_height - last_height).abs() > 0.1 {
                        tracing::info!(
                            target: "miaoyu_audio",
                            old_height = last_height,
                            new_height = current_height,
                            "Screen visible frame changed - repositioning audio bars"
                        );

                        reposition_audio_bars(&app);
                    }
                }

                last_visible_height = Some(current_height);
            }
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn start_screen_observer(_app: AppHandle<Wry>) {}
