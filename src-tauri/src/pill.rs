//! The floating pill: a small transparent window at the bottom centre of the
//! screen that shows the waveform / spinner.
//!
//! The window is shown once at startup and then never hidden. When idle it
//! simply draws nothing. It is only ever brought forward without activating
//! Loudr, so it can never steal focus from the app the user is typing in,
//! and because it ignores the mouse, clicks pass straight through it.

use tauri::{AppHandle, Manager};

/// Gap between the pill window and the bottom of the usable screen area.
const BOTTOM_MARGIN: f64 = 12.0;

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("pill") else {
        return Ok(());
    };
    window.set_ignore_cursor_events(true)?;

    #[cfg(target_os = "macos")]
    if std::env::var("LOUDR_PANEL").as_deref() != Ok("0") {
        make_panel(&window);
    }
    float_on_top(&window);
    reposition(app);
    Ok(())
}

/// Turn the pill into a non-activating panel.
///
/// An ordinary window can't be shown in another app's fullscreen Space, no
/// matter its level; macOS only lets panels (the kind of window used for
/// Spotlight or the emoji picker) join it. So the window's class is switched
/// to NSPanel and it is marked as never activating its app.
#[cfg(target_os = "macos")]
fn make_panel(window: &tauri::WebviewWindow) {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    const NONACTIVATING_PANEL: usize = 1 << 7;
    let (Ok(ns_window), Some(panel)) = (window.ns_window(), AnyClass::get(c"NSPanel")) else {
        return;
    };
    let ns_window = ns_window as *mut AnyObject;
    unsafe {
        objc2::ffi::object_setClass(ns_window.cast(), (panel as *const AnyClass).cast());
        let mask: usize = msg_send![ns_window, styleMask];
        let _: () = msg_send![ns_window, setStyleMask: mask | NONACTIVATING_PANEL];
        let _: () = msg_send![ns_window, setFloatingPanel: true];
        let _: () = msg_send![ns_window, setBecomesKeyOnlyIfNeeded: true];
    }
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
        let before: (bool, bool, usize) = (
            msg_send![ns_window, isVisible],
            msg_send![ns_window, isOnActiveSpace],
            msg_send![ns_window, occlusionState],
        );
        let _: () = msg_send![ns_window, orderFrontRegardless];
        let after: (bool, bool, usize, isize, usize, usize, f64) = (
            msg_send![ns_window, isVisible],
            msg_send![ns_window, isOnActiveSpace],
            msg_send![ns_window, occlusionState],
            msg_send![ns_window, level],
            msg_send![ns_window, collectionBehavior],
            msg_send![ns_window, styleMask],
            msg_send![ns_window, alphaValue],
        );
        log::info!("pill: before (visible, active space, occlusion) {before:?}; after (.., level, behavior, style, alpha) {after:?}");
    }
}

#[cfg(not(target_os = "macos"))]
fn float_on_top(window: &tauri::WebviewWindow) {
    let _ = window.set_always_on_top(true);
}

/// Make sure the pill is visible on the screen the user is working on.
/// Call this whenever the pill is about to show something.
pub fn bring_forward(app: &AppHandle) {
    let handle = app.clone();
    // Window calls must happen on the main thread.
    let _ = app.run_on_main_thread(move || {
        reposition(&handle);
        if let Some(window) = handle.get_webview_window("pill") {
            float_on_top(&window);
        }
    });
}

/// Move the pill to the bottom centre of the screen the mouse is on.
///
/// This asks macOS directly, in its own point coordinates. Going through
/// pixel positions goes wrong when two monitors have different pixel
/// densities (a Retina laptop next to an ordinary external screen).
#[cfg(target_os = "macos")]
pub fn reposition(app: &AppHandle) {
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2::msg_send;
    use objc2_foundation::{NSPoint, NSRect};

    let Some(window) = app.get_webview_window("pill") else {
        return;
    };
    let (Ok(ns_window), Some(event), Some(screen_class)) = (
        window.ns_window(),
        AnyClass::get(c"NSEvent"),
        AnyClass::get(c"NSScreen"),
    ) else {
        return;
    };
    let ns_window = ns_window as *mut AnyObject;
    unsafe {
        let mouse: NSPoint = msg_send![event, mouseLocation];
        let screens: *mut AnyObject = msg_send![screen_class, screens];
        let count: usize = msg_send![screens, count];
        for index in 0..count {
            let screen: *mut AnyObject = msg_send![screens, objectAtIndex: index];
            let frame: NSRect = msg_send![screen, frame];
            let inside = mouse.x >= frame.origin.x
                && mouse.x < frame.origin.x + frame.size.width
                && mouse.y >= frame.origin.y
                && mouse.y < frame.origin.y + frame.size.height;
            if !inside {
                continue;
            }
            // The visible frame leaves out the Dock and the menu bar, so the
            // pill sits just above the Dock. Here y grows upwards.
            let area: NSRect = msg_send![screen, visibleFrame];
            let size: NSRect = msg_send![ns_window, frame];
            let origin = NSPoint::new(
                area.origin.x + (area.size.width - size.size.width) / 2.0,
                area.origin.y + BOTTOM_MARGIN,
            );
            let _: () = msg_send![ns_window, setFrameOrigin: origin];
            return;
        }
    }
}

/// Move the pill to the bottom centre of the screen under the mouse cursor.
#[cfg(not(target_os = "macos"))]
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
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}
