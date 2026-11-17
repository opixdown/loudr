//! Insert text into the focused app: put it on the clipboard, send the paste
//! shortcut, then put the user's previous clipboard text back. Also the
//! reverse, for reading aloud: copy the selection out of the focused app.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::sync::mpsc;
use std::time::Duration;
use tauri::AppHandle;

pub fn paste(app: &AppHandle, text: &str) -> Result<(), String> {
    if !crate::permissions::accessibility_granted(false) {
        return Err("Accessibility permission needed to type text".into());
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard: {e}"))?;
    // Only text can be saved. If the clipboard held an image or files, it is
    // left holding the transcript afterwards.
    let previous = clipboard.get_text().ok();
    clipboard
        .set_text(text)
        .map_err(|e| format!("Clipboard: {e}"))?;

    send_shortcut(app, 'v')?;

    // The target app reads the clipboard a moment after the keystroke, so
    // restore on a delay, off the hot path.
    //
    //
    // On Linux (X11) the text is only served while this `Clipboard` exists:
    // dropping it right away can empty the clipboard before the target app has
    // read it. So here the same object is moved into the thread and kept
    // until the delay is over.
    #[cfg(target_os = "linux")]
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        if let Some(previous) = previous {
            let _ = clipboard.set_text(previous);
        }
    });
    #[cfg(not(target_os = "linux"))]
    if let Some(previous) = previous {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(400));
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(previous);
            }
        });
    }
    Ok(())
}

/// Copy whatever is selected in the focused app and return it as text.
/// The user's clipboard text is put back afterwards.
pub fn copy_selection(app: &AppHandle) -> Result<String, String> {
    if !crate::permissions::accessibility_granted(false) {
        return Err("Accessibility permission needed to read the selection".into());
    }
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard: {e}"))?;
    let previous = clipboard.get_text().ok();
    // A marker makes "the copy put new text on the clipboard" detectable,
    // even when the selection equals what was already there.
    const MARKER: &str = "\u{200b}loudr-waiting-for-copy\u{200b}";
    clipboard
        .set_text(MARKER)
        .map_err(|e| format!("Clipboard: {e}"))?;

    let sent = send_shortcut(app, 'c');
    let mut copied = None;
    if sent.is_ok() {
        // Apps take a moment to answer the copy shortcut.
        for _ in 0..25 {
            std::thread::sleep(Duration::from_millis(30));
            match clipboard.get_text() {
                Ok(text) if text != MARKER => {
                    copied = Some(text);
                    break;
                }
                _ => {}
            }
        }
    }
    let _ = match previous {
        Some(previous) => clipboard.set_text(previous),
        None => clipboard.clear(),
    };
    sent?;
    copied
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| "Select some text first, then press the read-aloud key.".to_string())
}

/// Press Cmd (macOS) or Ctrl (Windows) plus `key` in the focused app.
fn send_shortcut(app: &AppHandle, key: char) -> Result<(), String> {
    // macOS only allows synthetic key events to be built on the main thread
    // (keyboard layout lookups crash elsewhere), so hop over and wait.
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(press_shortcut(key));
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Keyboard shortcut timed out".to_string())?
}

fn press_shortcut(key: char) -> Result<(), String> {
    let modifier = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Keyboard: {e}"))?;
    let result = enigo
        .key(modifier, Direction::Press)
        .and_then(|_| enigo.key(Key::Unicode(key), Direction::Click));
    // Always release the modifier, even if the click failed.
    let released = enigo.key(modifier, Direction::Release);
    result.and(released).map_err(|e| format!("Keyboard: {e}"))
}
