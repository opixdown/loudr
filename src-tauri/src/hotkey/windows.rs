//! Windows: a low-level keyboard hook.
//!
//! Unlike the default macOS listener this one swallows the hold key, so other
//! apps never see it. Otherwise tapping Right Alt would focus the menu bar of
//! the app being typed into.

use super::{bound_key, capturing, CaptureResult, HotkeyEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

pub const DEFAULT_KEY: u32 = 0xA5; // VK_RMENU, Right Alt
const KEY_ESCAPE: u32 = 0x1B;

/// Modifier keys: (virtual-key code, name).
const MODIFIERS: &[(u32, &str)] = &[
    (0xA5, "Right Alt"),
    (0xA4, "Left Alt"),
    (0xA3, "Right Ctrl"),
    (0xA2, "Left Ctrl"),
    (0xA1, "Right Shift"),
    (0xA0, "Left Shift"),
    (0x5C, "Right Windows key"),
    (0x5B, "Left Windows key"),
];
/// VK_F1 ..= VK_F24
const FUNCTION_KEYS: std::ops::RangeInclusive<u32> = 0x70..=0x87;

pub fn is_supported(code: u32) -> bool {
    label(code).is_some()
}

pub fn label(code: u32) -> Option<String> {
    if let Some(m) = MODIFIERS.iter().find(|m| m.0 == code) {
        return Some(m.1.to_string());
    }
    FUNCTION_KEYS
        .contains(&code)
        .then(|| format!("F{}", code - FUNCTION_KEYS.start() + 1))
}

/// The hook reads the bound key on every event, so there is nothing to rebuild.
pub fn refresh() {}

// The hook procedure is a plain C callback with no user data pointer, so the
// handlers and key state have to live in statics.
type Handler<T> = OnceLock<Box<dyn Fn(T) + Send + Sync>>;
static ON_EVENT: Handler<HotkeyEvent> = OnceLock::new();
static ON_CAPTURE: Handler<CaptureResult> = OnceLock::new();
static HELD: AtomicBool = AtomicBool::new(false);

pub fn run(
    on_event: impl Fn(HotkeyEvent) + Send + Sync + 'static,
    on_capture: impl Fn(CaptureResult) + Send + Sync + 'static,
) {
    let _ = ON_EVENT.set(Box::new(on_event));
    let _ = ON_CAPTURE.set(Box::new(on_capture));

    unsafe {
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), std::ptr::null_mut(), 0);
        if hook.is_null() {
            log::error!("hotkey: SetWindowsHookExW failed");
            return;
        }
        log::info!("hotkey: listening");
        // A low-level hook is only called while its thread pumps messages.
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let key = unsafe { (*(lparam as *const KBDLLHOOKSTRUCT)).vkCode };
        let down = match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => Some(true),
            WM_KEYUP | WM_SYSKEYUP => Some(false),
            _ => None,
        };

        if capturing() {
            if let (Some(true), Some(on_capture)) = (down, ON_CAPTURE.get()) {
                on_capture(if key == KEY_ESCAPE {
                    CaptureResult::Cancelled
                } else if is_supported(key) {
                    CaptureResult::Captured(key)
                } else {
                    CaptureResult::Unsupported
                });
            }
            return 1; // keep keys pressed during capture out of other apps
        }

        if key == bound_key() {
            if let (Some(down), Some(on_event)) = (down, ON_EVENT.get()) {
                // Holding a key sends repeated key-downs; only fire on a change.
                if HELD.swap(down, Ordering::SeqCst) != down {
                    on_event(if down {
                        HotkeyEvent::Pressed
                    } else {
                        HotkeyEvent::Released
                    });
                }
            }
            return 1; // swallow the hold key
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}
