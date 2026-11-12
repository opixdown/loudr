//! What the app is doing right now, shown in the floating pill.

use crate::pill;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", content = "message", rename_all = "lowercase")]
pub enum Status {
    Idle,
    Recording,
    Transcribing,
    /// Getting ready to read aloud (loading the voice, making the first words).
    Preparing,
    /// Reading aloud.
    Speaking,
    Error(String),
}

pub fn set(app: &AppHandle, status: Status) {
    if let Status::Error(message) = &status {
        log::error!("{message}");
    }
    if status != Status::Idle {
        // Put the pill on whichever screen the user is working on, in front.
        pill::bring_forward(app);
    }
    let _ = app.emit_to("pill", "status", &status);

    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = match &status {
            Status::Error(message) => message.as_str(),
            _ => "Loudr",
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}
