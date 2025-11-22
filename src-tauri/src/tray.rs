use crate::{windows::ShowAppWindow, AppState};
use serde::Serialize;
use tauri::menu::MenuId;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

pub enum TrayItem {
    Home,
    Models,
    Settings,
    Quit,
}

impl From<TrayItem> for MenuId {
    fn from(value: TrayItem) -> Self {
        match value {
            TrayItem::Home => "home",
            TrayItem::Models => "models",
            TrayItem::Settings => "settings",
            TrayItem::Quit => "quit",
        }
        .into()
    }
}

impl TryFrom<MenuId> for TrayItem {
    type Error = String;

    fn try_from(value: MenuId) -> Result<Self, Self::Error> {
        match value.0.as_str() {
            "home" => Ok(TrayItem::Home),
            "models" => Ok(TrayItem::Models),
            "settings" => Ok(TrayItem::Settings),
            "quit" => Ok(TrayItem::Quit),
            value => Err(format!("Invalid tray item id {value}")),
        }
    }
}

#[derive(Clone, Serialize)]
struct NavigationPayload {
    path: String,
}

fn emit_navigation(app_handle: &AppHandle, path: &str) {
    {
        let state = app_handle.state::<AppState>();
        let mut pending_guard = match state.pending_navigation.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        *pending_guard = Some(path.to_string());
    }

    if let Err(error) = app_handle.emit_to(
        "dashboard",
        "navigate",
        NavigationPayload {
            path: path.to_string(),
        },
    ) {
        tracing::warn!(
            target = "miaoyu_tray",
            error = %error,
            path,
            "Failed to emit navigation event",
        );
    }
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let home_item = MenuItem::with_id(app, TrayItem::Home, "首页", true, None::<&str>)?;
    let models_item = MenuItem::with_id(app, TrayItem::Models, "模型管理", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(
        app,
        TrayItem::Settings,
        "应用设置",
        true,
        Some("CmdOrCtrl+,"),
    )?;

    let quit_item = MenuItem::with_id(app, TrayItem::Quit, "退出应用", true, Some("CmdOrCtrl+Q"))?;

    let menu = Menu::with_items(app, &[&home_item, &models_item, &settings_item, &quit_item])?;
    let app = app.clone();
    TrayIconBuilder::with_id("tray")
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event({
            move |app: &AppHandle, event| match TrayItem::try_from(event.id) {
                Ok(TrayItem::Home) => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = ShowAppWindow::Dashboard.show(&app_handle).await {
                            tracing::error!(
                                target = "miaoyu_tray",
                                error = ?error,
                                "Failed to open dashboard window",
                            );
                            return;
                        }
                        emit_navigation(&app_handle, "/");
                    });
                }
                Ok(TrayItem::Models) => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = ShowAppWindow::Dashboard.show(&app_handle).await {
                            tracing::error!(
                                target = "miaoyu_tray",
                                error = ?error,
                                "Failed to open dashboard window",
                            );
                            return;
                        }
                        emit_navigation(&app_handle, "/models");
                    });
                }
                Ok(TrayItem::Settings) => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = ShowAppWindow::Dashboard.show(&app_handle).await {
                            tracing::error!(
                                target = "miaoyu_tray",
                                error = ?error,
                                "Failed to open dashboard window",
                            );
                            return;
                        }
                        emit_navigation(&app_handle, "/settings");
                    });
                }
                Ok(TrayItem::Quit) => {
                    app.exit(0);
                }
                Err(error) => {
                    tracing::warn!(target = "miaoyu_tray", error = %error, "Unhandled tray menu id");
                }
            }
        })
        .on_tray_icon_event({
            move |tray, event| {
                if let tauri::tray::TrayIconEvent::Click { .. } = event {
                    let _ = tray.set_visible(true);
                }
            }
        })
        .build(&app)?;

    Ok(())
}
