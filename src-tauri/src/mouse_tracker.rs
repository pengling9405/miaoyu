use cidre::ns;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Wry};
use tokio::time::{interval, Duration};

use crate::windows::AppWindowId;

static IS_TRACKING: AtomicBool = AtomicBool::new(false);

/// Start tracking global mouse position to detect hover over Main window
/// This allows hover detection even when the window doesn't have focus
pub fn start_mouse_tracking(app: AppHandle<Wry>) {
    if IS_TRACKING.swap(true, Ordering::SeqCst) {
        return;
    }

    tokio::spawn(async move {
        let mut check_interval = interval(Duration::from_millis(50));
        let mut last_hover_state = false;
        let mut tick_count = 0u32;

        loop {
            check_interval.tick().await;
            tick_count += 1;

            // Get Main window
            let Some(window) = AppWindowId::Main.get(&app) else {
                if tick_count.is_multiple_of(20) {
                    tracing::warn!(target: "miaoyu_audio", "Main window not found");
                }
                continue;
            };

            // Get global mouse position using NSEvent (origin: bottom-left)
            let mouse_ns = ns::Event::mouse_location();

            // Get window bounds in Tauri coordinates (origin: top-left)
            let Ok(window_pos) = window.outer_position() else {
                if tick_count.is_multiple_of(20) {
                    tracing::error!(target: "miaoyu_audio", "Failed to get window position");
                }
                continue;
            };
            let Ok(window_size) = window.outer_size() else {
                if tick_count.is_multiple_of(20) {
                    tracing::error!(target: "miaoyu_audio", "Failed to get window size");
                }
                continue;
            };

            // Get monitor info for coordinate conversion
            let Ok(Some(_monitor)) = window.current_monitor() else {
                if tick_count.is_multiple_of(20) {
                    tracing::error!(target: "miaoyu_audio", "Failed to get monitor");
                }
                continue;
            };

            // NSEvent returns global coordinates (all screens combined, origin at bottom-left of primary screen)
            // We need to convert to window's screen coordinates (top-left origin)

            // For multi-monitor setups, we need the primary monitor to get the global screen space
            let Some(primary_monitor) = window
                .available_monitors()
                .ok()
                .and_then(|monitors| monitors.into_iter().next())
            else {
                if tick_count.is_multiple_of(20) {
                    tracing::error!(target: "miaoyu_audio", "Failed to get primary monitor");
                }
                continue;
            };
            let primary_size = primary_monitor.size();
            let scale_factor = primary_monitor.scale_factor();

            // NSEvent returns logical coordinates (points), but Tauri uses physical pixels
            // Convert mouse position from points to pixels, and flip Y axis
            let mouse_x = mouse_ns.x * scale_factor;
            let mouse_y = (primary_size.height as f64 / scale_factor - mouse_ns.y) * scale_factor;

            // Calculate window bounds in Tauri coordinates
            let window_x = window_pos.x as f64;
            let window_y = window_pos.y as f64;
            let window_width = window_size.width as f64;
            let window_height = window_size.height as f64;

            // Check if mouse is inside window bounds (both in Tauri coordinates now)
            let is_hovering = mouse_x >= window_x
                && mouse_x <= window_x + window_width
                && mouse_y >= window_y
                && mouse_y <= window_y + window_height;

            // Emit event on state change
            if is_hovering != last_hover_state {
                if let Err(e) = app.emit("audio-panel-hover", is_hovering) {
                    tracing::error!(
                        target: "miaoyu_audio",
                        error = ?e,
                        "Failed to emit hover event"
                    );
                }

                tracing::info!(
                    target: "miaoyu_audio",
                    is_hovering = is_hovering,
                    mouse_x = mouse_x,
                    mouse_y = mouse_y,
                    window_x = window_x,
                    window_y = window_y,
                    window_width = window_width,
                    window_height = window_height,
                    "⭐ Mouse hover state changed ⭐"
                );

                last_hover_state = is_hovering;
            }
        }
    });
}
