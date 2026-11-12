//! The floating pill: a small transparent window at the bottom centre of the
//! screen that shows the waveform / spinner.
//!
//! The window is shown once at startup and then never hidden. When idle it
//! simply draws nothing. It is only ever brought forward without activating
//! Loudr, so it can never steal focus from the app the user is typing in,
//! and because it ignores the mouse, clicks pass straight through it.

use tauri::{AppHandle, Manager, PhysicalPosition};

/// Gap between the pill window and the bottom of the usable screen area.
const BOTTOM_MARGIN: f64 = 12.0;

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("pill") else {
        return Ok(());
    };
    window.set_ignore_cursor_events(true)?;

    float_on_top(&window);
    reposition(app);
    Ok(())
}

/// Float above everything, on every Space, including over other apps'
/// fullscreen windows, and come to the front of that level.
///
/// This is applied again each time the pill has something to show: the
/// window system can drop these settings (for example when Spaces change),
/// and then the pill would sit unseen behind the app being used. Bringing
/// the window forward this way does not activate Loudr or take keyboard
/// focus.
#[cfg(target_os = "macos")]
fn float_on_top(window: &tauri::WebviewWindow) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    // Above the Dock and menu bar (the screen saver level is 1000).
    const POP_UP_MENU_LEVEL: isize = 101;
    // canJoinAllSpaces | stationary | ignoresCycle | fullScreenAuxiliary
    const BEHAVIOR: usize = (1 << 0) | (1 << 4) | (1 << 6) | (1 << 8);
    let Ok(ns_window) = window.ns_window() else {
        return;
    };
    let ns_window = ns_window as *mut AnyObject;
    unsafe {
        let _: () = msg_send![ns_window, setLevel: POP_UP_MENU_LEVEL];
        let _: () = msg_send![ns_window, setCollectionBehavior: BEHAVIOR];
        let _: () = msg_send![ns_window, setHidesOnDeactivate: false];
        let _: () = msg_send![ns_window, orderFrontRegardless];
    }
}

#[cfg(not(target_os = "macos"))]
fn float_on_top(window: &tauri::WebviewWindow) {
    let _ = window.set_always_on_top(true);
}

/// Make sure the pill is visible on the screen the user is working on.
/// Call this whenever the pill is about to show something.
pub fn bring_forward(app: &AppHandle) {
    reposition(app);
    let handle = app.clone();
    // Window calls must happen on the main thread.
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("pill") {
            float_on_top(&window);
        }
    });
}

/// Move the pill to the bottom centre of the screen under the mouse cursor.
pub fn reposition(app: &AppHandle) {
    let Some(window) = app.get_webview_window("pill") else {
        return;
    };
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else {
        return;
    };

    // The work area excludes the Dock / taskbar, so the pill sits above it.
    let area = monitor.work_area();
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y + area.size.height as i32
        - size.height as i32
        - (BOTTOM_MARGIN * monitor.scale_factor()) as i32;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}
