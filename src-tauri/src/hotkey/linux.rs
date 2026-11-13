//! Linux: read keyboards directly from /dev/input (evdev).
//!
//! This works on both X11 and Wayland, because it does not ask the display
//! server for key events. The price is that the user must be allowed to read
//! /dev/input/event* (normally: be in the `input` group).
//!
//! Unlike the Windows backend, this one only *watches* keys. It cannot stop
//! them reaching other apps, so the hold key is still seen by whatever app has
//! focus. Key codes are Linux evdev codes (KEY_RIGHTALT = 100, and so on).

use super::{bound_key, capturing, speak_key, CaptureResult, HotkeyEvent, TAP_MAX};
use evdev::{Device, EventType, KeyCode};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const DEFAULT_KEY: u32 = 100; // KEY_RIGHTALT
pub const DEFAULT_SPEAK_KEY: u32 = 97; // KEY_RIGHTCTRL
const KEY_ESCAPE: u32 = 1;

/// Modifier keys: (evdev key code, name).
const MODIFIERS: &[(u32, &str)] = &[
    (100, "Right Alt"),
    (56, "Left Alt"),
    (97, "Right Ctrl"),
    (29, "Left Ctrl"),
    (54, "Right Shift"),
    (42, "Left Shift"),
    (126, "Right Super"),
    (125, "Left Super"),
];

/// F1..F24 are not one block of codes in evdev, so map them by hand.
fn function_key_number(code: u32) -> Option<u32> {
    match code {
        59..=68 => Some(code - 58),   // KEY_F1 ..= KEY_F10
        87 => Some(11),               // KEY_F11
        88 => Some(12),               // KEY_F12
        183..=194 => Some(code - 170), // KEY_F13 ..= KEY_F24
        _ => None,
    }
}

fn is_function_key(code: u32) -> bool {
    function_key_number(code).is_some()
}

pub fn is_supported(code: u32) -> bool {
    label(code).is_some()
}

pub fn label(code: u32) -> Option<String> {
    if let Some(m) = MODIFIERS.iter().find(|m| m.0 == code) {
        return Some(m.1.to_string());
    }
    function_key_number(code).map(|n| format!("F{n}"))
}

/// Keys are checked against the bound key on every event, so there is
/// nothing to rebuild.
pub fn refresh() {}

type Callback<T> = Arc<dyn Fn(T) + Send + Sync>;

static HELD: AtomicBool = AtomicBool::new(false);
static SPEAK_HELD: AtomicBool = AtomicBool::new(false);
/// When a modifier read-aloud key went down, until another key is pressed
/// (which makes it a shortcut, not a tap).
static SPEAK_TAP: Mutex<Option<Instant>> = Mutex::new(None);

pub fn run(
    on_event: impl Fn(HotkeyEvent) + Send + Sync + 'static,
    on_capture: impl Fn(CaptureResult) + Send + Sync + 'static,
) {
    let on_event: Callback<HotkeyEvent> = Arc::new(on_event);
    let on_capture: Callback<CaptureResult> = Arc::new(on_capture);
    // Paths already handled: keyboards being watched, and devices that are
    // not keyboards. Scanning again picks up keyboards plugged in later.
    let seen: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));
    let mut warned = false;

    log::info!("hotkey: listening (evdev)");
    loop {
        let denied = scan(&seen, &on_event, &on_capture);
        if denied && !warned {
            warned = true;
            log::error!(
                "hotkey: cannot read /dev/input (permission denied). \
                 Run `sudo usermod -aG input $USER`, then log out and back in."
            );
        }
        std::thread::sleep(Duration::from_secs(3));
    }
}

/// Open every new keyboard in /dev/input and start a thread watching it.
/// Returns true if some device could not be opened for lack of permission.
fn scan(
    seen: &Arc<Mutex<HashSet<PathBuf>>>,
    on_event: &Callback<HotkeyEvent>,
    on_capture: &Callback<CaptureResult>,
) -> bool {
    let Ok(dir) = std::fs::read_dir("/dev/input") else {
        return false;
    };
    let mut denied = false;
    for entry in dir.flatten() {
        let path = entry.path();
        let is_event = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("event"));
        if !is_event || seen.lock().unwrap().contains(&path) {
            continue;
        }
        let device = match Device::open(&path) {
            Ok(device) => device,
            Err(e) => {
                if e.kind() == ErrorKind::PermissionDenied {
                    denied = true;
                }
                continue;
            }
        };
        seen.lock().unwrap().insert(path.clone());
        let is_keyboard = device
            .supported_keys()
            .is_some_and(|k| k.contains(KeyCode::KEY_A) && k.contains(KeyCode::KEY_LEFTCTRL));
        if !is_keyboard {
            continue;
        }
        log::info!(
            "hotkey: watching {} ({})",
            path.display(),
            device.name().unwrap_or("unnamed")
        );
        let (on_event, on_capture, seen) = (on_event.clone(), on_capture.clone(), seen.clone());
        std::thread::Builder::new()
            .name("hotkey-device".into())
            .spawn(move || watch(path, device, on_event, on_capture, seen))
            .ok();
    }
    denied
}

/// Read one keyboard until it goes away.
fn watch(
    path: PathBuf,
    mut device: Device,
    on_event: Callback<HotkeyEvent>,
    on_capture: Callback<CaptureResult>,
    seen: Arc<Mutex<HashSet<PathBuf>>>,
) {
    loop {
        match device.fetch_events() {
            Ok(events) => {
                for event in events {
                    // value: 1 = pressed, 0 = released, 2 = auto-repeat
                    if event.event_type() == EventType::KEY && event.value() != 2 {
                        handle(event.code() as u32, event.value() == 1, &on_event, &on_capture);
                    }
                }
            }
            Err(e) => {
                log::info!("hotkey: stopped watching {}: {e}", path.display());
                break;
            }
        }
    }
    // Forget it, so it is picked up again if it comes back.
    seen.lock().unwrap().remove(&path);
}

fn handle(
    key: u32,
    down: bool,
    on_event: &Callback<HotkeyEvent>,
    on_capture: &Callback<CaptureResult>,
) {
    if capturing() {
        if down {
            on_capture(if key == KEY_ESCAPE {
                CaptureResult::Cancelled
            } else if is_supported(key) {
                CaptureResult::Captured(key)
            } else {
                CaptureResult::Unsupported
            });
        }
        return;
    }

    if Some(key) == speak_key() {
        // Two keyboards can report the same key; only act on a change.
        let changed = SPEAK_HELD.swap(down, Ordering::SeqCst) != down;
        if is_function_key(key) {
            // Function key: fires when pressed.
            if changed && down {
                on_event(HotkeyEvent::Speak);
            }
        } else if changed {
            // Modifier key: fires when released after a short press with
            // nothing else pressed in between.
            let mut tap = SPEAK_TAP.lock().unwrap();
            if down {
                *tap = Some(Instant::now());
            } else if tap.take().is_some_and(|at| at.elapsed() < TAP_MAX) {
                on_event(HotkeyEvent::Speak);
            }
        }
    } else if down {
        SPEAK_TAP.lock().unwrap().take();
    }

    if key == bound_key() && HELD.swap(down, Ordering::SeqCst) != down {
        on_event(if down {
            HotkeyEvent::Pressed
        } else {
            HotkeyEvent::Released
        });
    }
}
