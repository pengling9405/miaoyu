use std::{thread, time::Duration};
use tauri::{AppHandle, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[cfg(target_os = "macos")]
fn send_paste() -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|_| {
        tracing::error!(target = "miaoyu_clipboard", "创建事件源失败");
        "创建事件源失败".to_string()
    })?;

    let cmd_key: CGKeyCode = 55;
    let v_key: CGKeyCode = 9;

    let cmd_down = CGEvent::new_keyboard_event(source.clone(), cmd_key, true).map_err(|_| {
        tracing::error!(target = "miaoyu_clipboard", "创建 Command 按下事件失败");
        "创建 Command 按下事件失败".to_string()
    })?;
    cmd_down.set_flags(CGEventFlags::CGEventFlagCommand);

    let v_down = CGEvent::new_keyboard_event(source.clone(), v_key, true).map_err(|_| {
        tracing::error!(target = "miaoyu_clipboard", "创建 V 按下事件失败");
        "创建 V 按下事件失败".to_string()
    })?;
    v_down.set_flags(CGEventFlags::CGEventFlagCommand);

    let v_up = CGEvent::new_keyboard_event(source.clone(), v_key, false).map_err(|_| {
        tracing::error!(target = "miaoyu_clipboard", "创建 V 弹起事件失败");
        "创建 V 弹起事件失败".to_string()
    })?;
    v_up.set_flags(CGEventFlags::CGEventFlagCommand);

    let cmd_up = CGEvent::new_keyboard_event(source, cmd_key, false).map_err(|_| {
        tracing::error!(target = "miaoyu_clipboard", "创建 Command 弹起事件失败");
        "创建 Command 弹起事件失败".to_string()
    })?;
    cmd_up.set_flags(CGEventFlags::CGEventFlagCommand);

    cmd_down.post(CGEventTapLocation::HID);
    v_down.post(CGEventTapLocation::HID);
    v_up.post(CGEventTapLocation::HID);
    cmd_up.post(CGEventTapLocation::HID);

    Ok(())
}

#[cfg(target_os = "windows")]
fn send_paste() -> Result<(), String> {
    use std::mem::size_of;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VIRTUAL_KEY, VK_CONTROL, VK_V,
    };

    fn key_input(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let inputs = [
        key_input(VK_CONTROL, KEYBD_EVENT_FLAGS(0)),
        key_input(VK_V, KEYBD_EVENT_FLAGS(0)),
        key_input(VK_V, KEYEVENTF_KEYUP),
        key_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];

    unsafe {
        let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
        if sent == inputs.len() as u32 {
            Ok(())
        } else {
            tracing::error!(
                target = "miaoyu_clipboard",
                sent,
                expected = inputs.len(),
                "发送粘贴按键失败"
            );
            Err("发送粘贴按键失败".to_string())
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn send_paste() -> Result<(), String> {
    Err("当前平台暂未实现自动粘贴".into())
}

pub fn paste<R: Runtime>(text: String, app_handle: &AppHandle<R>) -> Result<(), String> {
    let clipboard = app_handle.clipboard();

    // 总是先写入剪贴板
    clipboard.write_text(&text).map_err(|e| {
        tracing::error!(target = "miaoyu_clipboard", error = %e, "写入剪贴板失败");
        "写入剪贴板失败".to_string()
    })?;

    // 检查辅助功能权限（macOS）
    #[cfg(target_os = "macos")]
    if unsafe { !AXIsProcessTrusted() } {
        tracing::warn!(
            target = "miaoyu_clipboard",
            "未获得辅助功能权限，无法自动粘贴，内容已复制到剪贴板"
        );
        return Err("未获得辅助功能权限，内容已复制到剪贴板".to_string());
    }

    // 等待剪贴板写入完成
    thread::sleep(Duration::from_millis(60));

    // 发送粘贴按键
    send_paste()?;

    Ok(())
}
