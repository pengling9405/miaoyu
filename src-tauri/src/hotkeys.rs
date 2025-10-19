use global_hotkey::HotKeyState;
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use specta::Type;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_store::StoreExt;
use tauri_specta::Event;

use crate::{
    audio::{
        cancel_dictating, get_audio_state, start_dictating, stop_dictating, AudioState,
        DictatingMode,
    },
    windows::ShowAppWindow,
};

#[derive(Serialize, Deserialize, Type, PartialEq, Clone, Copy)]
pub struct Hotkey {
    #[specta(type = String)]
    code: Code,
    meta: bool,
    ctrl: bool,
    alt: bool,
    shift: bool,
}

impl From<Hotkey> for Shortcut {
    fn from(hotkey: Hotkey) -> Self {
        let mut modifiers = Modifiers::empty();

        if hotkey.meta {
            modifiers |= Modifiers::META;
        }
        if hotkey.ctrl {
            modifiers |= Modifiers::CONTROL;
        }
        if hotkey.alt {
            modifiers |= Modifiers::ALT;
        }
        if hotkey.shift {
            modifiers |= Modifiers::SHIFT;
        }

        Shortcut::new(Some(modifiers), hotkey.code)
    }
}

#[derive(Serialize, Deserialize, Type, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::enum_variant_names)]
pub enum HotkeyAction {
    StartDictating,
    #[serde(other)]
    Other,
}

#[derive(Serialize, Deserialize, Type)]
pub struct HotkeysStore {
    hotkeys: HashMap<HotkeyAction, Hotkey>,
}

impl HotkeysStore {
    pub fn get(app: &AppHandle) -> Result<Option<Self>, String> {
        let Ok(Some(store)) = app.store("store").map(|s| s.get("hotkeys")) else {
            return Ok(None);
        };

        serde_json::from_value(store).map_err(|e| e.to_string())
    }
}

impl Default for HotkeysStore {
    fn default() -> Self {
        let mut hotkeys = HashMap::new();
        hotkeys.insert(
            HotkeyAction::StartDictating,
            Hotkey {
                code: Code::Space,
                meta: false,
                ctrl: false,
                alt: true,
                shift: false,
            },
        );

        Self { hotkeys }
    }
}

#[derive(Serialize, Type, tauri_specta::Event, Debug, Clone)]
pub struct OnEscapePress;

pub type HotkeysState = Mutex<HotkeysStore>;

#[derive(Default)]
pub struct EscapeShortcutState {
    enabled: Mutex<bool>,
}

fn escape_shortcut() -> Shortcut {
    Shortcut::new(None, Code::Escape)
}

pub fn init(app: &AppHandle) {
    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                if !matches!(event.state(), HotKeyState::Pressed) {
                    return;
                }

                if shortcut.key == Code::Escape {
                    let app_clone = app.clone();
                    tokio::spawn(async move {
                        let _ = cancel_dictating(app_clone.clone()).await;
                        OnEscapePress.emit(&app_clone).ok();
                    });
                    return;
                }

                if shortcut.key == Code::Comma && shortcut.mods == Modifiers::META {
                    let app = app.clone();
                    tokio::spawn(async move {
                        let _ = ShowAppWindow::Settings.show(&app).await;
                    });
                }

                let state = app.state::<HotkeysState>();
                let store = state.lock().unwrap();
                for (action, hotkey) in &store.hotkeys {
                    if &Shortcut::from(*hotkey) == shortcut {
                        tokio::spawn(handle_hotkey(app.clone(), *action));
                    }
                }
            })
            .build(),
    )
    .unwrap();

    let store = match HotkeysStore::get(app) {
        Ok(Some(s)) => s,
        Ok(None) => HotkeysStore::default(),
        Err(e) => {
            eprintln!("Failed to load hotkeys store: {e}");
            HotkeysStore::default()
        }
    };

    let global_shortcut = app.global_shortcut();
    for hotkey in store.hotkeys.values() {
        global_shortcut.register(Shortcut::from(*hotkey)).ok();
    }

    app.manage(Mutex::new(store));
    app.manage(EscapeShortcutState::default());
}

async fn handle_hotkey(app: AppHandle, action: HotkeyAction) -> Result<(), String> {
    match action {
        HotkeyAction::StartDictating => {
            // 获取当前音频状态
            let current_state = get_audio_state().await;

            match current_state {
                AudioState::Idle => {
                    // 空闲状态：开始录音（快捷键模式）
                    start_dictating(app, DictatingMode::Hotkey).await
                }
                AudioState::Dictating { mode } => {
                    if mode == DictatingMode::Normal {
                        // Normal 模式的录音：忽略快捷键
                        tracing::debug!(target = "miaoyu_audio", "Normal 模式录音中，忽略快捷键");
                        Ok(())
                    } else {
                        // Hotkey 模式的录音：停止录音并转写
                        stop_dictating(app, false).await.map(|_| ())
                    }
                }
                AudioState::Transcribing => {
                    // 转写状态：忽略快捷键（避免中断转写）
                    tracing::debug!(target = "miaoyu_audio", "正在转写中，忽略快捷键");
                    Ok(())
                }
            }
        }
        HotkeyAction::Other => Ok(()),
    }
}

#[tauri::command(async)]
#[specta::specta]
pub fn set_hotkey(app: AppHandle, action: HotkeyAction, hotkey: Option<Hotkey>) -> Result<(), ()> {
    let global_shortcut = app.global_shortcut();
    let state = app.state::<HotkeysState>();
    let mut store = state.lock().unwrap();

    let prev = store.hotkeys.get(&action).cloned();

    if let Some(hotkey) = hotkey {
        store.hotkeys.insert(action, hotkey);
    } else {
        store.hotkeys.remove(&action);
    }

    if let Some(prev) = prev {
        let prev_still_in_use = store.hotkeys.values().any(|h| h == &prev);
        if !prev_still_in_use {
            global_shortcut.unregister(Shortcut::from(prev)).ok();
        }
    }

    if let Some(hotkey) = hotkey {
        global_shortcut.register(Shortcut::from(hotkey)).ok();
    }

    if let Ok(plugin_store) = app.store("store") {
        if let Ok(value) = to_value(&*store) {
            plugin_store.set("hotkeys", value);
            plugin_store.save().ok();
        }
    }

    Ok(())
}

pub fn set_escape_shortcut_enabled(app: &AppHandle, enabled: bool) {
    let state = app.state::<EscapeShortcutState>();
    let mut guard = state.enabled.lock().unwrap();
    let global_shortcut = app.global_shortcut();

    if enabled {
        if *guard {
            return;
        }
        match global_shortcut.register(escape_shortcut()) {
            Ok(_) => {
                *guard = true;
            }
            Err(error) => {
                tracing::warn!(
                    target = "miaoyu_hotkeys",
                    error = %error,
                    "注册 ESC 全局快捷键失败"
                );
            }
        }
    } else {
        if !*guard {
            return;
        }
        if let Err(error) = global_shortcut.unregister(escape_shortcut()) {
            tracing::warn!(
                target = "miaoyu_hotkeys",
                error = %error,
                "注销 ESC 全局快捷键失败"
            );
        }
        *guard = false;
    }
}
