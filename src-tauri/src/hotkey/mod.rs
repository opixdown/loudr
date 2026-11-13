//! The two global keys: hold to talk, and tap to read the selection aloud.
//!
//! The Tauri global-shortcut plugin cannot bind a lone modifier key such as
//! Right Option, so Loudr uses a small native keyboard listener per OS.
//!
//! The hold key is one key, identified by the platform's own key code (a
//! macOS virtual key code or a Windows virtual-key code). It can be any
//! modifier key or function key. Ordinary typing keys are not allowed,
//! because the hold key stops reaching other apps.
//!
//! The read-aloud key follows the same rules. A modifier key only counts
//! when it is tapped on its own, so using it in a shortcut does nothing.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    /// The hold-to-talk key went down.
    Pressed,
    /// The hold-to-talk key came back up.
    Released,
    /// The read-aloud key was tapped.
    Speak,
}

/// Which key a capture is choosing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureTarget {
    Hold,
    Speak,
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
/// Right Command on macOS, Right Ctrl on Windows.
pub const DEFAULT_SPEAK_KEY: u32 = platform::DEFAULT_SPEAK_KEY;
/// A modifier read-aloud key must be released within this long to count as
/// a tap.
const TAP_MAX: std::time::Duration = std::time::Duration::from_millis(400);

/// The key currently bound, and whether we are waiting for the user to press
/// a new one. Atomics, so changes apply without restarting the listener thread.
static BOUND_KEY: AtomicU32 = AtomicU32::new(DEFAULT_KEY);
static SPEAK_KEY: AtomicU32 = AtomicU32::new(DEFAULT_SPEAK_KEY);
static CAPTURING: AtomicBool = AtomicBool::new(false);
static CAPTURING_SPEAK_KEY: AtomicBool = AtomicBool::new(false);

pub fn set_key(code: u32) {
    // A settings file from the other OS could hold a code that means
    // nothing here; fall back to the default.
    let code = if platform::is_supported(code) { code } else { DEFAULT_KEY };
    BOUND_KEY.store(code, Ordering::SeqCst);
    platform::refresh();
}

pub fn set_speak_key(code: u32) {
    let code = if platform::is_supported(code) { code } else { DEFAULT_SPEAK_KEY };
    SPEAK_KEY.store(code, Ordering::SeqCst);
    platform::refresh();
}

/// The read-aloud key, or `None` when it clashes with the hold key (the
/// hold key wins).
fn speak_key() -> Option<u32> {
    let code = SPEAK_KEY.load(Ordering::SeqCst);
    (code != bound_key()).then_some(code)
}

fn bound_key() -> u32 {
    BOUND_KEY.load(Ordering::SeqCst)
}

/// A human-readable name for a key code, e.g. "Right Option" or "F5".
pub fn label(code: u32) -> String {
    platform::label(code).unwrap_or_else(|| platform::label(DEFAULT_KEY).unwrap())
}

/// Wait for the next key press and report it as the new key for `target`
/// instead of treating it as a hotkey. Both keys are paused until the
/// capture ends.
pub fn start_capture(target: CaptureTarget) {
    CAPTURING_SPEAK_KEY.store(target == CaptureTarget::Speak, Ordering::SeqCst);
    CAPTURING.store(true, Ordering::SeqCst);
    platform::refresh();
}

pub fn stop_capture() {
    CAPTURING.store(false, Ordering::SeqCst);
    platform::refresh();
}

/// Which key the current (or most recent) capture is for.
pub fn capture_target() -> CaptureTarget {
    if CAPTURING_SPEAK_KEY.load(Ordering::SeqCst) {
        CaptureTarget::Speak
    } else {
        CaptureTarget::Hold
    }
}

fn capturing() -> bool {
    CAPTURING.load(Ordering::SeqCst)
}

/// Start listening on a background thread.
///
/// `on_event` is called exactly once per press and once per release of the
/// hold key (no key repeat), and once per tap of the read-aloud key. `on_capture` is called while a capture is running. Both are
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
