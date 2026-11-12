//! Whisper model files: where they live and how they are downloaded.
//!
//! Downloading a model (this one, or the optional voice for text to speech)
//! is the only time Loudr touches the network.

use crate::pipeline::{self, Command};
use crate::settings::{self, AppState};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// (id, label, approximate download size in MB)
pub const SIZES: &[(&str, &str, u32)] = &[
    ("tiny", "Tiny", 75),
    ("base", "Base", 142),
    ("small", "Small", 466),
    ("medium", "Medium", 1500),
];

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// The "language" for Hindi spoken and written in English letters ("kal
/// meeting hai"). Standard Whisper can't write that, so it has its own
/// model: Whisper-Hindi2Hinglish-Swift by Oriserve (Apache 2.0), a fine-tune
/// of Whisper Base, converted to the ggml format and hosted with Loudr's
/// releases. It comes in one size.
pub const HINGLISH: &str = "hinglish";
pub const HINGLISH_MB: u32 = 148;
const HINGLISH_FILE: &str = "ggml-hinglish-base.bin";
const HINGLISH_URL: &str = "https://github.com/opixdown/loudr/releases/download/models-v1";

/// English has dedicated, more accurate ".en" models. Every other language
/// (and auto-detect) uses the multilingual model of the same size.
pub fn file_name(size: &str, language: &str) -> String {
    if language == HINGLISH {
        return HINGLISH_FILE.into();
    }
    let suffix = if language == "en" { ".en" } else { "" };
    format!("ggml-{size}{suffix}.bin")
}

pub fn dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("no app data directory")
        .join("models")
}

pub fn path(app: &AppHandle, size: &str, language: &str) -> PathBuf {
    dir(app).join(file_name(size, language))
}

pub fn is_downloaded(app: &AppHandle, size: &str, language: &str) -> bool {
    path(app, size, language).exists()
}

/// Progress event sent to the settings window as "model-download".
#[derive(Clone, Serialize)]
struct Progress {
    file: String,
    downloaded: u64,
    total: u64,
    done: bool,
    error: Option<String>,
}

/// Download a model on a background thread, reporting progress as events.
/// When it finishes, the pipeline is told to (re)load the current model.
pub fn download(app: &AppHandle, size: &str, language: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.downloading.swap(true, Ordering::SeqCst) {
        return Err("A model download is already running".into());
    }

    let app = app.clone();
    let file = file_name(size, language);
    std::thread::spawn(move || {
        let base = if file == HINGLISH_FILE { HINGLISH_URL } else { BASE_URL };
        let url = format!("{base}/{file}");
        let result = fetch(&app, &url, &dir(&app), &file, "model-download");
        app.state::<AppState>()
            .downloading
            .store(false, Ordering::SeqCst);

        if let Err(e) = &result {
            log::error!("model download failed: {e}");
        } else {
            log::info!("model downloaded: {file}");
            pipeline::send(&app, Command::ReloadModel);
        }
        emit_finished(&app, "model-download", &file, result);
    });
    Ok(())
}

/// Tell the window a download ended, successfully or not.
pub fn emit_finished(app: &AppHandle, event: &str, file: &str, result: Result<(), String>) {
    let _ = app.emit(
        event,
        Progress {
            file: file.to_string(),
            downloaded: 0,
            total: 0,
            done: result.is_ok(),
            error: result.err(),
        },
    );
}

/// Download `url` to `dir/file`, reporting progress as `event` events.
pub fn fetch(app: &AppHandle, url: &str, dir: &Path, file: &str, event: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    // Download to a temporary name so a half-finished file is never loaded.
    let partial = dir.join(format!("{file}.part"));

    let mut response = ureq::get(url)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    let total = response.body().content_length().unwrap_or(0);
    let mut reader = response.body_mut().as_reader();
    let mut out = std::fs::File::create(&partial).map_err(|e| e.to_string())?;

    let mut chunk = vec![0u8; 256 * 1024];
    let mut downloaded = 0u64;
    let mut last_event = Instant::now();
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| format!("Download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&chunk[..n]).map_err(|e| e.to_string())?;
        downloaded += n as u64;

        if last_event.elapsed() > Duration::from_millis(100) {
            last_event = Instant::now();
            let _ = app.emit(
                event,
                Progress {
                    file: file.to_string(),
                    downloaded,
                    total,
                    done: false,
                    error: None,
                },
            );
        }
    }
    out.flush().map_err(|e| e.to_string())?;
    drop(out);

    if total > 0 && downloaded != total {
        let _ = std::fs::remove_file(&partial);
        return Err("Download incomplete, please try again".into());
    }
    std::fs::rename(&partial, dir.join(file)).map_err(|e| e.to_string())
}

/// True when the model chosen in settings is on disk.
pub fn current_ready(app: &AppHandle) -> bool {
    let s = settings::current(app);
    is_downloaded(app, &s.model, &s.language)
}
