//! Commands the settings / onboarding window can call.

use crate::pipeline::{self, Command};
use crate::settings::{self, AppState, Settings};
use crate::{audio, hotkey, models, permissions, tray};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    settings::current(&app)
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) {
    apply_settings(&app, settings);
}

/// Store new settings and make every part of the app follow them.
/// Used by the settings window and by the tray model picker.
pub fn apply_settings(app: &AppHandle, new: Settings) {
    let old = {
        let state = app.state::<AppState>();
        let mut current = state.settings.lock().unwrap();
        std::mem::replace(&mut *current, new.clone())
    };
    if old == new {
        return;
    }
    settings::save(app, &new);
    if old.hotkey_code != new.hotkey_code {
        hotkey::set_key(new.hotkey_code);
    }
    if old.theme != new.theme {
        apply_theme(app, &new.theme);
    }

    if old.launch_at_login != new.launch_at_login {
        let autostart = app.autolaunch();
        let result = if new.launch_at_login {
            autostart.enable()
        } else {
            autostart.disable()
        };
        if let Err(e) = result {
            log::error!("could not change launch at login: {e}");
        }
    }

    if old.model != new.model || old.language != new.language {
        tray::sync_model(app, &new.model);
        if models::is_downloaded(app, &new.model, &new.language) {
            pipeline::send(app, Command::ReloadModel);
        } else {
            // Not on disk yet: fetch it, and show the window with the
            // progress bar so the user knows what is happening.
            if let Err(e) = models::download(app, &new.model, &new.language) {
                log::warn!("{e}");
            }
            tray::show_settings(app);
        }
    }
    let _ = app.emit("settings-changed", &new);
}

/// Make the settings window's title bar follow the chosen theme (the page
/// itself is themed in CSS).
pub fn apply_theme(app: &AppHandle, theme: &str) {
    let theme = match theme {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None, // follow the system
    };
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.set_theme(theme);
    }
}

/// Wait for the user to press the key they want as the hold key. The result
/// arrives as a "hotkey-capture" event.
#[tauri::command]
pub fn start_hotkey_capture() {
    hotkey::start_capture();
}

#[tauri::command]
pub fn cancel_hotkey_capture() {
    hotkey::stop_capture();
}

/// Called by the hotkey listener while a capture is running.
pub fn on_hotkey_capture(app: &AppHandle, result: hotkey::CaptureResult) {
    use hotkey::CaptureResult::*;
    let outcome = match result {
        Captured(code) => {
            hotkey::stop_capture();
            let mut s = settings::current(app);
            s.hotkey_code = code;
            apply_settings(app, s);
            "captured"
        }
        Cancelled => {
            hotkey::stop_capture();
            "cancelled"
        }
        Unsupported => "unsupported",
    };
    let _ = app.emit("hotkey-capture", outcome);
}

#[derive(Serialize)]
pub struct SetupState {
    /// Name of the hold-to-talk key, e.g. "Right Option".
    hotkey_label: String,
    platform: &'static str,
    onboarded: bool,
    /// "granted", "denied" or "unknown" (not asked yet)
    microphone: &'static str,
    accessibility: bool,
    model_ready: bool,
    downloading: bool,
}

#[tauri::command]
pub fn get_setup_state(app: AppHandle) -> SetupState {
    setup_state(&app)
}

pub fn setup_state(app: &AppHandle) -> SetupState {
    SetupState {
        hotkey_label: hotkey::label(settings::current(app).hotkey_code),
        platform: std::env::consts::OS,
        onboarded: settings::current(app).onboarded,
        microphone: permissions::microphone_status(),
        accessibility: permissions::accessibility_granted(false),
        model_ready: models::current_ready(app),
        downloading: app
            .state::<AppState>()
            .downloading
            .load(std::sync::atomic::Ordering::SeqCst),
    }
}

impl SetupState {
    /// True when nothing stands between the user and dictating.
    pub fn ready(&self) -> bool {
        self.onboarded && self.microphone == "granted" && self.accessibility && self.model_ready
    }
}

#[tauri::command]
pub fn request_microphone() {
    permissions::request_microphone();
}

#[tauri::command]
pub fn request_accessibility() {
    permissions::accessibility_granted(true);
}

/// Open the matching page of System Settings ("microphone" or "accessibility").
#[tauri::command]
pub fn open_privacy_settings(pane: String) {
    permissions::open_privacy_settings(&pane);
}

#[tauri::command]
pub fn list_input_devices() -> Vec<String> {
    audio::input_devices()
}

#[derive(Serialize)]
pub struct ModelInfo {
    id: &'static str,
    label: &'static str,
    size_mb: u32,
    downloaded: bool,
}

/// The model sizes, and whether each is on disk for the current language.
#[tauri::command]
pub fn list_models(app: AppHandle) -> Vec<ModelInfo> {
    let language = settings::current(&app).language;
    models::SIZES
        .iter()
        .map(|&(id, label, size_mb)| ModelInfo {
            id,
            label,
            size_mb,
            downloaded: models::is_downloaded(&app, id, &language),
        })
        .collect()
}

/// Download the model chosen in settings.
#[tauri::command]
pub fn download_model(app: AppHandle) -> Result<(), String> {
    let s = settings::current(&app);
    models::download(&app, &s.model, &s.language)
}

#[tauri::command]
pub fn finish_onboarding(app: AppHandle) {
    let mut s = settings::current(&app);
    s.onboarded = true;
    apply_settings(&app, s);
}
