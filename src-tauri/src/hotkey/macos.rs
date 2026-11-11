//! macOS: a CGEventTap on the keyboard.
//!
//! Modifier keys arrive as "flags changed" events, and a listen-only tap is
//! enough for them, so with a modifier hold key (the default) Loudr only ever
//! observes modifier changes. A function-key hold key, or capturing a new
//! key, needs an active tap that also sees (and swallows) key presses. The
//! tap is rebuilt whenever that requirement changes.

use super::{bound_key, capturing, CaptureResult, HotkeyEvent};
use crate::permissions;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    CallbackResult, EventField,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

pub const DEFAULT_KEY: u32 = 61; // Right Option
const KEY_ESCAPE: u32 = 53;

/// Modifier keys: (key code, name, flag bit). The flag is the device-specific
/// bit that is set only while that exact key is down; the generic modifier
/// flags can't tell the left key from the right one.
const MODIFIERS: &[(u32, &str, u64)] = &[
    (61, "Right Option", 0x40),
    (58, "Left Option", 0x20),
    (54, "Right Command", 0x10),
    (55, "Left Command", 0x08),
    (62, "Right Control", 0x2000),
    (59, "Left Control", 0x01),
    (60, "Right Shift", 0x04),
    (56, "Left Shift", 0x02),
    (63, "Fn", 0x80_0000),
];

/// Function keys F1 to F20, by key code.
const FUNCTION_KEYS: [u32; 20] = [
    122, 120, 99, 118, 96, 97, 98, 100, 101, 109, 103, 111, 105, 107, 113, 106, 64, 79, 80, 90,
];

fn modifier_flag(code: u32) -> Option<u64> {
    MODIFIERS.iter().find(|m| m.0 == code).map(|m| m.2)
}

pub fn is_supported(code: u32) -> bool {
    label(code).is_some()
}

pub fn label(code: u32) -> Option<String> {
    if let Some(m) = MODIFIERS.iter().find(|m| m.0 == code) {
        return Some(m.1.to_string());
    }
    let index = FUNCTION_KEYS.iter().position(|&k| k == code)?;
    Some(format!("F{}", index + 1))
}

/// The listener thread's run loop, so other threads can ask for a rebuild.
static RUN_LOOP: Mutex<Option<CFRunLoop>> = Mutex::new(None);

/// Rebuild the tap, because the hold key or capture mode changed.
pub fn refresh() {
    if let Some(run_loop) = RUN_LOOP.lock().unwrap().as_ref() {
        run_loop.stop();
    }
}

pub fn run(
    on_event: impl Fn(HotkeyEvent) + Send + Sync + 'static,
    on_capture: impl Fn(CaptureResult) + Send + Sync + 'static,
) {
    // The tap sees nothing until the user grants Accessibility. The first-run
    // window asks for it; we just wait here, so no restart is needed after
    // they flip the switch.
    if !permissions::accessibility_granted(false) {
        log::warn!("hotkey: waiting for Accessibility permission");
        while !permissions::accessibility_granted(false) {
            std::thread::sleep(Duration::from_secs(1));
        }
        log::info!("hotkey: Accessibility permission granted");
    }

    let on_event = Arc::new(on_event);
    let on_capture = Arc::new(on_capture);
    let held = Arc::new(AtomicBool::new(false));
    *RUN_LOOP.lock().unwrap() = Some(CFRunLoop::get_current());

    // Each pass builds one tap and runs until `refresh()` is called or macOS
    // disables the tap (which it does if a tap ever stalls).
    loop {
        let needs_key_events = capturing() || modifier_flag(bound_key()).is_none();
        let (options, events) = if needs_key_events {
            (
                CGEventTapOptions::Default,
                vec![
                    CGEventType::FlagsChanged,
                    CGEventType::KeyDown,
                    CGEventType::KeyUp,
                ],
            )
        } else {
            (CGEventTapOptions::ListenOnly, vec![CGEventType::FlagsChanged])
        };

        let (event_cb, capture_cb, held_cb) = (on_event.clone(), on_capture.clone(), held.clone());
        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            options,
            events,
            move |_proxy, event_type, event| {
                let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u32;
                // `down` is None for events that are neither a press nor a release.
                let down = match event_type {
                    CGEventType::FlagsChanged => {
                        modifier_flag(code).map(|flag| event.get_flags().bits() & flag != 0)
                    }
                    CGEventType::KeyDown => Some(true),
                    CGEventType::KeyUp => Some(false),
                    CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                        log::warn!("hotkey: event tap was disabled, restarting it");
                        CFRunLoop::get_current().stop();
                        None
                    }
                    _ => None,
                };
                let is_key_event = !matches!(event_type, CGEventType::FlagsChanged);

                if capturing() {
                    if down == Some(true) {
                        capture_cb(if code == KEY_ESCAPE {
                            CaptureResult::Cancelled
                        } else if is_supported(code) {
                            CaptureResult::Captured(code)
                        } else {
                            CaptureResult::Unsupported
                        });
                    }
                    // Keep the keys pressed during capture out of other apps.
                    return if is_key_event { CallbackResult::Drop } else { CallbackResult::Keep };
                }

                if code == bound_key() {
                    if let Some(down) = down {
                        // swap() returns the old value, so we only fire on a
                        // change (holding a function key repeats key-downs).
                        if held_cb.swap(down, Ordering::SeqCst) != down {
                            event_cb(if down {
                                HotkeyEvent::Pressed
                            } else {
                                HotkeyEvent::Released
                            });
                        }
                        // A function-key hold key must not reach other apps.
                        if is_key_event {
                            return CallbackResult::Drop;
                        }
                    }
                }
                CallbackResult::Keep
            },
        );

        let Ok(tap) = tap else {
            log::error!("hotkey: could not create event tap, retrying in 2s");
            std::thread::sleep(Duration::from_secs(2));
            continue;
        };

        let source = tap
            .mach_port()
            .create_runloop_source(0)
            .expect("failed to create run loop source");
        let run_loop = CFRunLoop::get_current();
        run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });
        tap.enable();
        log::info!(
            "hotkey: listening for {} ({})",
            super::label(bound_key()),
            if needs_key_events { "active tap" } else { "listen-only tap" }
        );
        CFRunLoop::run_current(); // blocks until stop()
        run_loop.remove_source(&source, unsafe { kCFRunLoopCommonModes });

        // If the key was down when the tap went away we never saw the release.
        if held.swap(false, Ordering::SeqCst) {
            on_event(HotkeyEvent::Released);
        }
    }
}
