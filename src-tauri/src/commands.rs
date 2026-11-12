//! Commands the settings / onboarding window can call.

use crate::pipeline::{self, Command};
use crate::settings::{self, AppState, Settings};
use crate::{audio, hotkey, models, permissions, tray, tts};
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
    if old.speak_key != new.speak_key {
        hotkey::set_speak_key(new.speak_key);
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

/// Wait for the user to press the key they want as the hold key ("hold") or
/// the read-aloud key ("speak"). The result arrives as a "hotkey-capture"
/// event.
#[tauri::command]
pub fn start_hotkey_capture(target: String) {
    hotkey::start_capture(match target.as_str() {
        "speak" => hotkey::CaptureTarget::Speak,
        _ => hotkey::CaptureTarget::Hold,
    });
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
            let mut s = settings::current(app);
            let for_speak = hotkey::capture_target() == hotkey::CaptureTarget::Speak;
            // The two keys must differ; keep waiting for another one.
            let taken = if for_speak { s.hotkey_code } else { s.speak_key };
            if code == taken {
                let _ = app.emit("hotkey-capture", "unsupported");
                return;
            }
            hotkey::stop_capture();
            if for_speak {
                s.speak_key = code;
            } else {
                s.hotkey_code = code;
            }
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
    /// Name of the read-aloud key.
    speak_label: String,
    /// The text-to-speech voice is on disk.
    voice_ready: bool,
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
    let s = settings::current(app);
    SetupState {
        hotkey_label: hotkey::label(s.hotkey_code),
        speak_label: hotkey::label(s.speak_key),
        voice_ready: tts::is_downloaded(app),
        platform: std::env::consts::OS,
        onboarded: s.onboarded,
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

#[derive(Serialize)]
pub struct VoiceInfo {
    id: &'static str,
    label: &'static str,
}

#[derive(Serialize)]
pub struct Voices {
    voices: Vec<VoiceInfo>,
    /// Size of the one-time voice download.
    download_mb: u32,
}

#[tauri::command]
pub fn list_voices() -> Voices {
    Voices {
        voices: tts::VOICES
            .iter()
            .map(|&(id, label)| VoiceInfo { id, label })
            .collect(),
        download_mb: tts::DOWNLOAD_MB,
    }
}

#[tauri::command]
pub fn download_voice(app: AppHandle) -> Result<(), String> {
    tts::download(&app)
}

/// Read text aloud. Progress arrives as "speech" events.
#[tauri::command]
pub fn speak_text(app: AppHandle, text: String) {
    tts::speak(&app, text);
}

#[tauri::command]
pub fn stop_speaking(app: AppHandle) {
    tts::stop(&app);
    let _ = app.emit("speech", serde_json::json!({ "state": "idle", "progress": 0.0 }));
}

/// Ask where to save, then write text as an MP3 or WAV file. The result
/// arrives as a "speech" event carrying the file's path.
#[tauri::command]
pub fn export_speech(app: AppHandle, text: String) {
    tts::export(&app, text);
}

/// The start of the text that was read aloud last, if its audio can be saved.
#[tauri::command]
pub fn last_speech(app: AppHandle) -> Option<String> {
    tts::last_preview(&app)
}

/// Save the audio that was read aloud last (a selection or a script).
#[tauri::command]
pub fn save_last_speech(app: AppHandle) {
    tts::save_last(&app);
}
