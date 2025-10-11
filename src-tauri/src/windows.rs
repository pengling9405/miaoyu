use anyhow::anyhow;
#[cfg(target_os = "macos")]
use cidre::ns;
use serde::Deserialize;
use specta::Type;
use std::{path::PathBuf, str::FromStr};
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Wry,
};
use tracing::warn;

use crate::settings::{AppTheme, AudioFlowPanelPosition, SettingsStore};

#[derive(Clone, Deserialize, Type)]
pub enum AppWindowId {
    Setup,
    Settings,
    Main,
    Feedback,
}

impl FromStr for AppWindowId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "setup" => Self::Setup,
            "settings" => Self::Settings,
            "main" => Self::Main,
            "feedback" => Self::Feedback,
            _ => return Err(format!("unknown window label: {s}")),
        })
    }
}

impl std::fmt::Display for AppWindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Setup => write!(f, "setup"),
            Self::Settings => write!(f, "settings"),
            Self::Main => write!(f, "main"),
            Self::Feedback => write!(f, "feedback"),
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
        matches!(self, Self::Setup | Self::Settings | Self::Main)
    }
}

#[derive(Clone, Type, Deserialize)]
pub enum ShowAppWindow {
    Setup,
    Settings,
    Main,
    Feedback,
}

impl ShowAppWindow {
    pub async fn show(&self, app: &AppHandle<Wry>) -> tauri::Result<WebviewWindow> {
        if let Some(window) = self.id(app).get(app) {
            window.set_focus().ok();
            return Ok(window);
        }

        let _id = self.id(app);

        let window = match self {
            Self::Setup => {
                let window = self
                    .window_builder(app, "/setup")
                    .resizable(false)
                    .maximized(false)
                    .center()
                    .focused(true)
                    .inner_size(420.0, 360.0)
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
            Self::Main => {
                #[cfg(target_os = "macos")]
                let window = {
                    use tauri::TitleBarStyle;
                    self.window_builder(app, "/")
                        .inner_size(40.0, 8.0)
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
                        .build()?
                };

                #[cfg(not(target_os = "macos"))]
                let window = self
                    .window_builder(app, "/")
                    .inner_size(40.0, 8.0)
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

                reposition_audio_bar_with_monitor(&window, current_audio_panel_position(app))?;
                window.show()?;

                window
            }
            Self::Feedback => {
                #[cfg(target_os = "macos")]
                let window = {
                    use tauri::TitleBarStyle;
                    self.window_builder(app, "/feedback")
                        .inner_size(320.0, 48.0)
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
                        .build()?
                };

                #[cfg(not(target_os = "macos"))]
                let window = self
                    .window_builder(app, "/feedback")
                    .inner_size(320.0, 48.0)
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

                // Feedback 窗口不立即显示，由 feedback 模块控制
                window
            }
        };

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
            ShowAppWindow::Setup => AppWindowId::Setup,
            ShowAppWindow::Settings => AppWindowId::Settings,
            ShowAppWindow::Main => AppWindowId::Main,
            ShowAppWindow::Feedback => AppWindowId::Feedback,
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

pub fn reposition_audio_bars(app: &AppHandle<Wry>, position: AudioFlowPanelPosition) {
    if let Some(window) = AppWindowId::Main.get(app) {
        if let Err(error) = reposition_audio_bar_with_monitor(&window, position) {
            warn!(
                target = "miaoyu_audio",
                ?error,
                "Failed to reposition Main window",
            );
        }
    }
}

fn current_audio_panel_position(app: &AppHandle<Wry>) -> AudioFlowPanelPosition {
    SettingsStore::get(app)
        .ok()
        .flatten()
        .map(|settings| settings.audio_flow_panel_position)
        .unwrap_or(AudioFlowPanelPosition::BottomCenter)
}

fn reposition_audio_bar_with_monitor(
    window: &WebviewWindow,
    position: AudioFlowPanelPosition,
) -> tauri::Result<()> {
    let app = window.app_handle();
    let monitor = app
        .primary_monitor()?
        .ok_or_else(|| tauri::Error::Anyhow(anyhow!("Failed to get primary monitor")))?;

    let pos = monitor.position();
    let size = monitor.size();
    let logical_pos = PhysicalPosition::new(pos.x, pos.y);
    let logical_size = PhysicalSize::new(size.width, size.height);

    position_audio_bar(window, logical_pos, logical_size, position)
}

#[cfg(target_os = "macos")]
fn position_audio_bar(
    window: &WebviewWindow,
    monitor_pos: PhysicalPosition<i32>,
    monitor_size: tauri::PhysicalSize<u32>,
    position: AudioFlowPanelPosition,
) -> tauri::Result<()> {
    let window_size = window.outer_size()?;
    let margin = 16;
    let scale_factor = window.scale_factor()?;

    let centered_x = monitor_pos.x + (monitor_size.width as i32 - window_size.width as i32) / 2;

    let y = match position {
        AudioFlowPanelPosition::BottomCenter => {
            // Use visible frame if available (excludes Dock area)
            if let Some(screen) = ns::Screen::main() {
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
            }
        }
        AudioFlowPanelPosition::TopCenter => {
            // Top position: account for menu bar via visible frame
            if let Some(screen) = ns::Screen::main() {
                let visible = screen.visible_frame();
                let frame = screen.frame();

                // visible.origin.y + visible.size.height is the top of visible area from screen bottom
                // Convert to top-left origin
                let visible_top_from_top =
                    (frame.size.height - (visible.origin.y + visible.size.height)) * scale_factor;

                tracing::debug!(
                    target: "miaoyu_audio",
                    visible_top = visible_top_from_top,
                    "Positioning below menu bar"
                );

                (visible_top_from_top as i32) + margin
            } else {
                // Fallback
                monitor_pos.y + margin
            }
        }
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
    position: AudioFlowPanelPosition,
) -> tauri::Result<()> {
    let window_size = window.outer_size()?;
    let margin = 16;

    let centered_x = monitor_pos.x + (monitor_size.width as i32 - window_size.width as i32) / 2;

    let y = match position {
        AudioFlowPanelPosition::BottomCenter => {
            monitor_pos.y + monitor_size.height as i32 - window_size.height as i32 - margin
        }
        AudioFlowPanelPosition::TopCenter => monitor_pos.y + margin,
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

// 内部函数：调整窗口大小并保持位置
pub fn resize_main_window(
    window: &WebviewWindow,
    app: &AppHandle<Wry>,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let old_size = window.outer_size().map_err(|e| e.to_string())?;
    let old_pos = window.outer_position().map_err(|e| e.to_string())?;

    let panel_position = current_audio_panel_position(app);

    tracing::info!(
        target: "miaoyu_audio",
        panel_position = ?panel_position,
        old_width = old_size.width,
        old_height = old_size.height,
        old_x = old_pos.x,
        old_y = old_pos.y,
        new_width = width,
        new_height = height,
        "Window state before resize"
    );

    // For bottom panel: window should expand UPWARD (top edge moves up, bottom edge stays)
    // For top panel: window should expand DOWNWARD (top edge stays, bottom edge moves down)
    let (anchor_y, is_bottom) = match panel_position {
        AudioFlowPanelPosition::BottomCenter => {
            // Keep bottom edge Y fixed, expand upward
            let bottom_y = old_pos.y + old_size.height as i32;
            tracing::info!(
                target: "miaoyu_audio",
                bottom_y = bottom_y,
                "Bottom panel: keeping bottom edge fixed"
            );
            (bottom_y, true)
        }
        AudioFlowPanelPosition::TopCenter => {
            // Keep top edge Y fixed, expand downward
            tracing::info!(
                target: "miaoyu_audio",
                top_y = old_pos.y,
                "Top panel: keeping top edge fixed"
            );
            (old_pos.y, false)
        }
    };

    // Calculate horizontal center
    let center_x = old_pos.x + (old_size.width as i32) / 2;

    // Get scale factor for HiDPI calculation
    let scale_factor = window.scale_factor().map_err(|e| e.to_string())?;
    tracing::info!(
        target: "miaoyu_audio",
        scale_factor = scale_factor,
        "Display scale factor"
    );

    // Calculate physical size (logical size * scale_factor)
    let new_physical_width = (width * scale_factor) as u32;
    let new_physical_height = (height * scale_factor) as u32;

    tracing::info!(
        target: "miaoyu_audio",
        new_physical_width = new_physical_width,
        new_physical_height = new_physical_height,
        "Calculated physical size"
    );

    // Calculate new position BEFORE resizing (to avoid timing issues)
    let new_x = center_x - (new_physical_width as i32) / 2;
    let new_y = if is_bottom {
        anchor_y - new_physical_height as i32
    } else {
        anchor_y
    };

    tracing::info!(
        target: "miaoyu_audio",
        old_x = old_pos.x,
        old_y = old_pos.y,
        new_x = new_x,
        new_y = new_y,
        anchor_y = anchor_y,
        "Position change (bottom edge should stay at anchor_y)"
    );

    // Apply size and position atomically on main thread
    let new_size = LogicalSize::new(width, height);
    let new_pos = PhysicalPosition::new(new_x, new_y);
    let window_clone = window.clone();

    window
        .run_on_main_thread(move || {
            // Set position FIRST, then size - this prevents visual jump
            let _ = window_clone.set_position(new_pos);
            let _ = window_clone.set_size(new_size);
        })
        .ok();

    // Don't set focus - global mouse tracking handles hover detection without focus

    tracing::info!(
        target: "miaoyu_audio",
        "Main window resize completed"
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn resize_audio_panel(app: AppHandle<Wry>, width: f64, height: f64) -> Result<(), String> {
    tracing::info!(
        target: "miaoyu_audio",
        width = width,
        height = height,
        "Resizing Main window"
    );

    let window = AppWindowId::Main
        .get(&app)
        .ok_or_else(|| "Main window not found".to_string())?;

    resize_main_window(&window, &app, width, height)
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

                        let position = current_audio_panel_position(&app);
                        reposition_audio_bars(&app, position);
                    }
                }

                last_visible_height = Some(current_height);
            }
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn start_screen_observer(_app: AppHandle<Wry>) {}
