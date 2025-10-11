use crate::windows::ShowAppWindow;
use tauri::menu::MenuId;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

pub enum TrayItem {
    Settings,
    Quit,
}

impl From<TrayItem> for MenuId {
    fn from(value: TrayItem) -> Self {
        match value {
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
            "settings" => Ok(TrayItem::Settings),
            "quit" => Ok(TrayItem::Quit),
            value => Err(format!("Invalid tray item id {value}")),
        }
    }
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let settings_item = MenuItem::with_id(
        app,
        TrayItem::Settings,
        "应用设置",
        true,
        Some("CmdOrCtrl+,"),
    )?;

    let quit_item = MenuItem::with_id(app, TrayItem::Quit, "退出应用", true, Some("CmdOrCtrl+Q"))?;

    let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;
    let app = app.clone();
    let _ = TrayIconBuilder::with_id("tray")
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event({
            move |app: &AppHandle, event| match TrayItem::try_from(event.id) {
                Ok(TrayItem::Settings) => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = ShowAppWindow::Settings.show(&app_handle).await {
                            tracing::error!(
                                target = "miaoyu_tray",
                                error = ?error,
                                "Failed to open settings window",
                            );
                        }
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
        .build(&app);

    Ok(())
}
