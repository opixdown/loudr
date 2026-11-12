//! Text to speech: read the selected text aloud, speak a script, or save a
//! script as an audio file. Runs fully offline with the Kokoro voice model.
//!
//! Like dictation, the work happens on one worker thread that owns the model
//! and the speaker stream.

mod engine;
mod player;
mod text;

use crate::settings::{self, AppState};
use crate::status::{self, Status};
use crate::{models, paste, tray};
use engine::{Engine, SAMPLE_RATE};
use player::Player;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// (id, label). The first letter of the id is the accent, the second the
/// gender; the ids are the names used inside the voices file.
pub const VOICES: &[(&str, &str)] = &[
    ("am_puck", "Puck · American, male"),
    ("am_michael", "Michael · American, male"),
    ("am_fenrir", "Fenrir · American, male"),
    ("am_onyx", "Onyx · American, male"),
    ("af_heart", "Heart · American, female"),
    ("af_bella", "Bella · American, female"),
    ("af_nicole", "Nicole · American, female"),
    ("bm_george", "George · British, male"),
    ("bf_emma", "Emma · British, female"),
];
pub const DEFAULT_VOICE: &str = "am_puck";

const MODEL_FILE: &str = "kokoro-v1.0.fp16.onnx";
const VOICES_FILE: &str = "voices-v1.0.bin";
const BASE_URL: &str =
    "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0";
/// Approximate size of both files together.
pub const DOWNLOAD_MB: u32 = 205;

/// The model uses a few hundred MB of memory, so it is released after
/// speech hasn't been used for this long.
const UNLOAD_AFTER: Duration = Duration::from_secs(5 * 60);
/// Don't synthesize further ahead of the speakers than this.
const MAX_QUEUED_SECS: f32 = 20.0;

pub fn dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("no app data directory")
        .join("voices")
}

pub fn is_downloaded(app: &AppHandle) -> bool {
    let dir = dir(app);
    dir.join(MODEL_FILE).exists() && dir.join(VOICES_FILE).exists()
}

/// Download the voice model on a background thread. Progress is reported as
/// "voice-download" events.
pub fn download(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.downloading.swap(true, Ordering::SeqCst) {
        return Err("Another download is already running".into());
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let dir = dir(&app);
        // Small file first, so a problem shows up quickly.
        let result = [VOICES_FILE, MODEL_FILE].iter().try_for_each(|file| {
            if dir.join(file).exists() {
                return Ok(());
            }
            models::fetch(&app, &format!("{BASE_URL}/{file}"), &dir, file, "voice-download")
        });
        app.state::<AppState>()
            .downloading
            .store(false, Ordering::SeqCst);
        match &result {
            Ok(()) => log::info!("voice model downloaded"),
            Err(e) => log::error!("voice download failed: {e}"),
        }
        models::emit_finished(&app, "voice-download", MODEL_FILE, result);
    });
    Ok(())
}

enum Command {
    /// Copy the selection in the focused app and read it aloud.
    SpeakSelection,
    Speak(String),
    /// Save the text as a WAV file in the Downloads folder.
    Export(String),
}

/// What the speech worker is doing, sent to the window as "speech" events.
#[derive(Clone, Default, Serialize)]
struct SpeechEvent {
    /// "idle", "preparing", "speaking" or "exporting"
    state: &'static str,
    /// 0.0 to 1.0 while exporting.
    progress: f32,
    error: Option<String>,
    /// Path of the file an export just wrote.
    saved: Option<String>,
}

fn emit(app: &AppHandle, state: &'static str, progress: f32) {
    let event = SpeechEvent { state, progress, ..Default::default() };
    let _ = app.emit("speech", event);
}

/// Handle to the speech worker, managed by Tauri.
pub struct Speech {
    tx: Sender<(u64, Command)>,
    /// Raised to cancel: a command only runs while the epoch still has the
    /// value it had when the command was sent.
    epoch: Arc<AtomicU64>,
    /// Commands sent and not yet finished.
    pending: Arc<AtomicUsize>,
}

fn request(app: &AppHandle, command: Command) {
    let speech = app.state::<Speech>();
    speech.pending.fetch_add(1, Ordering::SeqCst);
    let _ = speech.tx.send((speech.epoch.load(Ordering::SeqCst), command));
}

pub fn is_busy(app: &AppHandle) -> bool {
    app.state::<Speech>().pending.load(Ordering::SeqCst) > 0
}

/// Stop speaking (or exporting) right away.
pub fn stop(app: &AppHandle) {
    if is_busy(app) {
        app.state::<Speech>().epoch.fetch_add(1, Ordering::SeqCst);
        status::set(app, Status::Idle);
    }
}

pub fn speak(app: &AppHandle, text: String) {
    stop(app);
    request(app, Command::Speak(text));
}

pub fn export(app: &AppHandle, text: String) {
    stop(app);
    request(app, Command::Export(text));
}

/// The read-aloud key: start reading the selection, or stop if already
/// speaking.
pub fn toggle_selection(app: &AppHandle) {
    if is_busy(app) {
        stop(app);
    } else if !is_downloaded(app) {
        // Nothing to speak with yet: open the page that offers the download.
        tray::show_settings(app);
        let _ = app.emit("navigate", "speech");
    } else {
        request(app, Command::SpeakSelection);
    }
}

pub fn spawn(app: &AppHandle) {
    let (tx, rx) = mpsc::channel::<(u64, Command)>();
    let epoch = Arc::new(AtomicU64::new(0));
    let pending = Arc::new(AtomicUsize::new(0));
    app.manage(Speech { tx, epoch: epoch.clone(), pending: pending.clone() });

    let app = app.clone();
    std::thread::Builder::new()
        .name("speech".into())
        .spawn(move || {
            let mut engine: Option<Engine> = None;
            loop {
                let (sent_at, command) = match rx.recv_timeout(UNLOAD_AFTER) {
                    Ok(message) => message,
                    Err(RecvTimeoutError::Timeout) => {
                        if engine.take().is_some() {
                            log::info!("speech: voice model unloaded after being idle");
                        }
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                let cancelled = || epoch.load(Ordering::SeqCst) != sent_at;

                let mut event = SpeechEvent { state: "idle", ..Default::default() };
                if !cancelled() {
                    let result = match command {
                        Command::SpeakSelection => paste::copy_selection(&app)
                            .and_then(|text| speak_now(&app, &mut engine, &text, &cancelled)),
                        Command::Speak(text) => speak_now(&app, &mut engine, &text, &cancelled),
                        Command::Export(text) => {
                            export_now(&app, &mut engine, &text, &cancelled).map(|path| {
                                event.saved = path.map(|p| p.display().to_string());
                            })
                        }
                    };
                    // Whoever cancelled has already updated the pill.
                    if !cancelled() {
                        match &result {
                            Ok(()) => status::set(&app, Status::Idle),
                            Err(e) => status::set(&app, Status::Error(e.clone())),
                        }
                    }
                    event.error = result.err();
                }
                // Only report "idle" when nothing else is queued behind this.
                if pending.fetch_sub(1, Ordering::SeqCst) == 1 || event.error.is_some() || event.saved.is_some() {
                    let _ = app.emit("speech", event);
                }
            }
        })
        .expect("failed to spawn speech thread");
}

fn loaded<'a>(app: &AppHandle, engine: &'a mut Option<Engine>) -> Result<&'a mut Engine, String> {
    if engine.is_none() {
        if !is_downloaded(app) {
            return Err("No voice yet. Open Loudr and download it under Text to speech.".into());
        }
        let started = Instant::now();
        let dir = dir(app);
        *engine = Some(Engine::load(&dir.join(MODEL_FILE), &dir.join(VOICES_FILE))?);
        log::info!("speech: voice model loaded in {:?}", started.elapsed());
    }
    Ok(engine.as_mut().unwrap())
}

fn speak_now(
    app: &AppHandle,
    engine: &mut Option<Engine>,
    text: &str,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    let started = Instant::now();
    let chunks = text::chunks(text);
    if chunks.is_empty() {
        return Err("Nothing to read".into());
    }
    status::set(app, Status::Preparing);
    emit(app, "preparing", 0.0);

    let engine = loaded(app, engine)?;
    let s = settings::current(app);
    let player = Player::open()?;
    for (index, chunk) in chunks.iter().enumerate() {
        if cancelled() {
            return Ok(());
        }
        let audio = engine.synthesize(chunk, &s.voice, s.voice_speed)?;
        if cancelled() {
            return Ok(());
        }
        player.push(&audio);
        if index == 0 {
            log::info!(
                "speech: first audio after {:?} ({} chunks, voice {})",
                started.elapsed(),
                chunks.len(),
                s.voice
            );
            status::set(app, Status::Speaking);
            emit(app, "speaking", 0.0);
        }
        // For a long text, stay only a little ahead of the speakers.
        while player.queued_secs() > MAX_QUEUED_SECS && !cancelled() {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    player.finish(cancelled);
    Ok(())
}

/// Returns the saved file, or `None` if the export was cancelled.
fn export_now(
    app: &AppHandle,
    engine: &mut Option<Engine>,
    text: &str,
    cancelled: &dyn Fn() -> bool,
) -> Result<Option<PathBuf>, String> {
    let chunks = text::chunks(text);
    if chunks.is_empty() {
        return Err("Nothing to save. Type or paste some text first.".into());
    }
    emit(app, "exporting", 0.0);
    let engine = loaded(app, engine)?;
    let s = settings::current(app);

    let mut samples: Vec<f32> = Vec::new();
    for (index, chunk) in chunks.iter().enumerate() {
        if cancelled() {
            return Ok(None);
        }
        samples.extend(engine.synthesize(chunk, &s.voice, s.voice_speed)?);
        emit(app, "exporting", (index + 1) as f32 / chunks.len() as f32);
    }

    let folder = app
        .path()
        .download_dir()
        .map_err(|e| format!("No Downloads folder: {e}"))?;
    let path = free_path(&folder, &file_stem(text));
    write_wav(&path, &samples).map_err(|e| format!("Could not save the file: {e}"))?;
    log::info!(
        "speech: saved {:.1}s to {}",
        samples.len() as f32 / SAMPLE_RATE as f32,
        path.display()
    );
    reveal(&path);
    Ok(Some(path))
}

/// "Loudr - first few words", safe to use as a file name.
fn file_stem(text: &str) -> String {
    let words: String = text
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .take(40)
        .collect();
    format!("Loudr - {}", words.trim())
}

/// A path in `folder` that doesn't exist yet: "name.wav", "name 2.wav", ...
fn free_path(folder: &Path, stem: &str) -> PathBuf {
    (1..)
        .map(|n| match n {
            1 => folder.join(format!("{stem}.wav")),
            n => folder.join(format!("{stem} {n}.wav")),
        })
        .find(|p| !p.exists())
        .unwrap()
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in samples {
        writer.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
    }
    writer.finalize()
}

/// Show the saved file in Finder / Explorer.
fn reveal(path: &Path) {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg("-R").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    if let Err(e) = result {
        log::warn!("could not reveal {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Speaks a paragraph into a WAV file with a real voice model:
    ///   LOUDR_VOICE_DIR=<folder with the two voice files> LOUDR_OUT=out.wav \
    ///     cargo test --release speaks_to_wav -- --ignored --nocapture
    #[test]
    #[ignore = "needs the downloaded voice model"]
    fn speaks_to_wav() {
        let dir = PathBuf::from(std::env::var("LOUDR_VOICE_DIR").expect("set LOUDR_VOICE_DIR"));
        let out = std::env::var("LOUDR_OUT").expect("set LOUDR_OUT");
        let voice = std::env::var("LOUDR_VOICE").unwrap_or(DEFAULT_VOICE.into());
        let text = std::env::var("LOUDR_TEXT").unwrap_or(
            "Loudr can now read text aloud. Select anything, press a key, and listen.".into(),
        );

        let started = Instant::now();
        let mut engine = Engine::load(&dir.join(MODEL_FILE), &dir.join(VOICES_FILE)).unwrap();
        println!("loaded in {:?}", started.elapsed());

        let mut samples = Vec::new();
        for chunk in text::chunks(&text) {
            let started = Instant::now();
            let audio = engine.synthesize(&chunk, &voice, 1.0).unwrap();
            println!(
                "{:?} for {:.1}s of audio: {chunk}",
                started.elapsed(),
                audio.len() as f32 / SAMPLE_RATE as f32
            );
            samples.extend(audio);
        }
        assert!(samples.len() > SAMPLE_RATE as usize);
        write_wav(Path::new(&out), &samples).unwrap();
    }
}
