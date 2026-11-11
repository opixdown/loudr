//! Insert text into the focused app: put it on the clipboard, send the paste
//! shortcut, then put the user's previous clipboard text back.

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

    // macOS only allows synthetic key events to be built on the main thread
    // (keyboard layout lookups crash elsewhere), so hop over and wait.
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(send_paste_shortcut());
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Paste timed out".to_string())??;

    // The target app reads the clipboard a moment after the keystroke, so
    // restore on a delay, off the hot path.
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

fn send_paste_shortcut() -> Result<(), String> {
    let modifier = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Keyboard: {e}"))?;
    let result = enigo
        .key(modifier, Direction::Press)
        .and_then(|_| enigo.key(Key::Unicode('v'), Direction::Click));
    // Always release the modifier, even if the click failed.
    let released = enigo.key(modifier, Direction::Release);
    result.and(released).map_err(|e| format!("Keyboard: {e}"))
}
