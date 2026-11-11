//! Hold-to-talk hotkey.
//!
//! The Tauri global-shortcut plugin cannot bind a lone modifier key such as
//! Right Option, so Loudr uses a small native keyboard listener per OS.
//!
//! The hold key is one key, identified by the platform's own key code (a
//! macOS virtual key code or a Windows virtual-key code). It can be any
//! modifier key or function key. Ordinary typing keys are not allowed,
//! because the hold key stops reaching other apps.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Pressed,
    Released,
}

/// What happened while the settings window was waiting for a new hold key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureResult {
    /// The user pressed a usable key (its key code).
    Captured(u32),
    /// The user pressed a key that can't be a hold key; still waiting.
    Unsupported,
    /// The user pressed Escape.
    Cancelled,
}

/// Right Option on macOS, Right Alt on Windows.
pub const DEFAULT_KEY: u32 = platform::DEFAULT_KEY;

/// The key currently bound, and whether we are waiting for the user to press
/// a new one. Atomics, so changes apply without restarting the listener thread.
static BOUND_KEY: AtomicU32 = AtomicU32::new(DEFAULT_KEY);
static CAPTURING: AtomicBool = AtomicBool::new(false);

pub fn set_key(code: u32) {
    // A settings file from the other OS could hold a code that means
    // nothing here; fall back to the default.
    let code = if platform::is_supported(code) { code } else { DEFAULT_KEY };
    BOUND_KEY.store(code, Ordering::SeqCst);
    platform::refresh();
}

fn bound_key() -> u32 {
    BOUND_KEY.load(Ordering::SeqCst)
}

/// A human-readable name for a key code, e.g. "Right Option" or "F5".
pub fn label(code: u32) -> String {
    platform::label(code).unwrap_or_else(|| platform::label(DEFAULT_KEY).unwrap())
}

/// Wait for the next key press and report it as the new hold key instead of
/// treating it as a hotkey. Dictation is paused until the capture ends.
pub fn start_capture() {
    CAPTURING.store(true, Ordering::SeqCst);
    platform::refresh();
}

pub fn stop_capture() {
    CAPTURING.store(false, Ordering::SeqCst);
    platform::refresh();
}

fn capturing() -> bool {
    CAPTURING.load(Ordering::SeqCst)
}

/// Start listening on a background thread.
///
/// `on_event` is called exactly once per press and once per release (no key
/// repeat). `on_capture` is called while a capture is running. Both are
/// called from the listener thread and must return quickly.
pub fn start(
    on_event: impl Fn(HotkeyEvent) + Send + Sync + 'static,
    on_capture: impl Fn(CaptureResult) + Send + Sync + 'static,
) {
    std::thread::Builder::new()
        .name("hotkey".into())
        .spawn(move || platform::run(on_event, on_capture))
        .expect("failed to spawn hotkey thread");
}
