mod audio;
mod commands;
mod hotkey;
mod models;
mod paste;
mod permissions;
mod pill;
mod pipeline;
mod settings;
mod status;
mod transcribe;
mod tray;
mod tts;

use settings::AppState;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Launching Loudr while it is already running shows the window of
        // the running copy. Two copies would both type every dictation.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_settings(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                // whisper.cpp prints dozens of lines per model load.
                .level_for("whisper_rs", log::LevelFilter::Warn)
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::get_setup_state,
            commands::request_microphone,
            commands::request_accessibility,
            commands::open_privacy_settings,
            commands::list_input_devices,
            commands::list_models,
            commands::download_model,
            commands::finish_onboarding,
            commands::start_hotkey_capture,
            commands::cancel_hotkey_capture,
            commands::list_voices,
            commands::download_voice,
            commands::speak_text,
            commands::stop_speaking,
            commands::export_speech,
        ])
        .setup(|app| {
            // Menu bar app: no Dock icon, no app switcher entry.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle();
            let loaded = settings::load(handle);
            hotkey::set_key(loaded.hotkey_code);
            hotkey::set_speak_key(loaded.speak_key);
            commands::apply_theme(handle, &loaded.theme);
            app.manage(AppState {
                settings: Mutex::new(loaded),
                downloading: AtomicBool::new(false),
            });

            tray::create(handle)?;
            pill::setup(handle)?;
            pipeline::spawn(handle);
            tts::spawn(handle);

            // The hotkey listener only forwards events; all the work happens
            // on the pipeline and speech threads.
            let (pipeline_handle, capture_handle) = (handle.clone(), handle.clone());
            hotkey::start(
                move |event| match event {
                    hotkey::HotkeyEvent::Speak => tts::toggle_selection(&pipeline_handle),
                    _ => pipeline::send(
                        &pipeline_handle,
                        pipeline::Command::Hotkey(event, Instant::now()),
                    ),
                },
                move |result| {
                    // Saving settings touches the disk; keep that off the
                    // keyboard listener thread.
                    let app = capture_handle.clone();
                    std::thread::spawn(move || commands::on_hotkey_capture(&app, result));
                },
            );

            // First run, or something is missing (permission revoked, model
            // deleted): open the window so the user can fix it.
            if !commands::setup_state(handle).ready() {
                tray::show_settings(handle);
            }

            log::info!("Loudr started");
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing Settings only hides it; the app lives on in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                    // Don't leave the app waiting for a new hold key forever.
                    commands::on_hotkey_capture(
                        window.app_handle(),
                        hotkey::CaptureResult::Cancelled,
                    );
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Loudr")
        .run(|_app, _event| {
            // macOS: opening the app again (Finder, Spotlight, Launchpad)
            // while it runs in the menu bar brings up the window.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                tray::show_settings(_app);
            }
        });
}
