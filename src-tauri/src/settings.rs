//! User settings, stored as JSON in the app config directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Hold-to-talk key, as this platform's key code (see `hotkey`).
    pub hotkey_code: u32,
    /// "system", "light" or "dark".
    pub theme: String,
    /// Model size: tiny, base, small or medium.
    pub model: String,
    /// Whisper language code, or "auto" to detect.
    pub language: String,
    /// Microphone name; `None` means the system default.
    pub input_device: Option<String>,
    pub launch_at_login: bool,
    /// Set once the first-run flow has been completed.
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey_code: crate::hotkey::DEFAULT_KEY,
            theme: "system".into(),
            model: "base".into(),
            language: "en".into(),
            input_device: None,
            launch_at_login: false,
            onboarded: false,
        }
    }
}

/// Shared app state, managed by Tauri.
pub struct AppState {
    pub settings: Mutex<Settings>,
    /// True while a model download is running (only one at a time).
    pub downloading: AtomicBool,
}

fn file(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .expect("no app config directory")
        .join("settings.json")
}

/// Read settings from disk; missing or broken files fall back to defaults.
pub fn load(app: &AppHandle) -> Settings {
    std::fs::read_to_string(file(app))
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) {
    let path = file(app);
    let result = std::fs::create_dir_all(path.parent().unwrap()).and_then(|_| {
        std::fs::write(&path, serde_json::to_string_pretty(settings).unwrap())
    });
    if let Err(e) = result {
        log::error!("could not save settings: {e}");
    }
}

/// A copy of the current settings.
pub fn current(app: &AppHandle) -> Settings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}
