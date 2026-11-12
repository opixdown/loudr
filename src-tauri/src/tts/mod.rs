//! Text to speech: read the selected text aloud, speak a script, or save
//! either as an MP3 or WAV file. Runs fully offline with the Kokoro voice model.
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
    /// Ask where to save, then write the text as an audio file.
    Export(String),
    /// Save whatever was read aloud last.
    SaveLast,
}

/// The most recent complete reading, kept so it can be saved without
/// making the audio again.
struct Spoken {
    text: String,
    voice: String,
    speed: f32,
    samples: Vec<f32>,
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
    /// The start of the text that was read aloud last and can be saved.
    last: Option<String>,
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
    /// The start of the text read aloud last, if it can still be saved.
    last_preview: Arc<std::sync::Mutex<Option<String>>>,
}

pub fn last_preview(app: &AppHandle) -> Option<String> {
    app.state::<Speech>().last_preview.lock().unwrap().clone()
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

pub fn save_last(app: &AppHandle) {
    stop(app);
    request(app, Command::SaveLast);
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
    let last_preview = Arc::new(std::sync::Mutex::new(None));
    app.manage(Speech {
        tx,
        epoch: epoch.clone(),
        pending: pending.clone(),
        last_preview: last_preview.clone(),
    });

    let app = app.clone();
    std::thread::Builder::new()
        .name("speech".into())
        .spawn(move || {
            let mut engine: Option<Engine> = None;
            let mut last: Option<Spoken> = None;
            loop {
                let (sent_at, command) = match rx.recv_timeout(UNLOAD_AFTER) {
                    Ok(message) => message,
                    Err(RecvTimeoutError::Timeout) => {
                        if engine.take().is_some() {
                            log::info!("speech: voice model unloaded after being idle");
                        }
                        // Don't hold on to minutes of audio forever either.
                        if last.take().is_some() {
                            *last_preview.lock().unwrap() = None;
                            let _ = app.emit("speech", SpeechEvent { state: "idle", ..Default::default() });
                        }
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                let cancelled = || epoch.load(Ordering::SeqCst) != sent_at;

                let mut event = SpeechEvent { state: "idle", ..Default::default() };
                if !cancelled() {
                    let result = match command {
                        Command::SpeakSelection => paste::copy_selection(&app).and_then(|text| {
                            speak_now(&app, &mut engine, &mut last, &text, &cancelled)
                        }),
                        Command::Speak(text) => {
                            speak_now(&app, &mut engine, &mut last, &text, &cancelled)
                        }
                        Command::Export(text) => {
                            export_now(&app, &mut engine, &mut last, &text, &cancelled)
                                .map(|path| event.saved = path.map(|p| p.display().to_string()))
                        }
                        Command::SaveLast => match last.as_ref().map(|l| l.text.clone()) {
                            Some(text) => {
                                export_now(&app, &mut engine, &mut last, &text, &cancelled)
                                    .map(|path| event.saved = path.map(|p| p.display().to_string()))
                            }
                            None => Err("Nothing has been read aloud yet.".into()),
                        },
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
                event.last = last.as_ref().map(|l| l.text.chars().take(80).collect());
                *last_preview.lock().unwrap() = event.last.clone();
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
    last: &mut Option<Spoken>,
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
    let mut samples: Vec<f32> = Vec::new();
    for (index, chunk) in chunks.iter().enumerate() {
        if cancelled() {
            return Ok(());
        }
        let audio = engine.synthesize(chunk, &s.voice, s.voice_speed)?;
        if cancelled() {
            return Ok(());
        }
        player.push(&audio);
        samples.extend(audio);
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
    // Everything has been made, so it can be saved even if playback is
    // stopped early.
    *last = Some(Spoken {
        text: text.trim().to_string(),
        voice: s.voice,
        speed: s.voice_speed,
        samples,
    });
    player.finish(cancelled);
    Ok(())
}

/// Ask where to save, make the audio (unless it was just read aloud) and
/// write it. Returns the saved file, or `None` if the user cancelled.
fn export_now(
    app: &AppHandle,
    engine: &mut Option<Engine>,
    last: &mut Option<Spoken>,
    text: &str,
    cancelled: &dyn Fn() -> bool,
) -> Result<Option<PathBuf>, String> {
    let text = text.trim();
    let chunks = text::chunks(text);
    if chunks.is_empty() {
        return Err("Nothing to save. Type or paste some text first.".into());
    }
    let Some(path) = ask_where_to_save(app, text) else {
        return Ok(None);
    };
    emit(app, "exporting", 0.0);

    let s = settings::current(app);
    let reusable = last
        .as_ref()
        .is_some_and(|l| l.text == text && l.voice == s.voice && l.speed == s.voice_speed);
    if !reusable {
        let engine = loaded(app, engine)?;
        let mut samples: Vec<f32> = Vec::new();
        for (index, chunk) in chunks.iter().enumerate() {
            if cancelled() {
                return Ok(None);
            }
            samples.extend(engine.synthesize(chunk, &s.voice, s.voice_speed)?);
            emit(app, "exporting", (index + 1) as f32 / chunks.len() as f32);
        }
        *last = Some(Spoken {
            text: text.to_string(),
            voice: s.voice,
            speed: s.voice_speed,
            samples,
        });
    }
    let samples = &last.as_ref().unwrap().samples;

    let is_wav = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"));
    if is_wav {
        write_wav(&path, samples).map_err(|e| format!("Could not save the file: {e}"))?;
    } else {
        let mp3 = encode_mp3(samples)?;
        std::fs::write(&path, mp3).map_err(|e| format!("Could not save the file: {e}"))?;
    }
    log::info!(
        "speech: saved {:.1}s to {}",
        samples.len() as f32 / SAMPLE_RATE as f32,
        path.display()
    );
    reveal(&path);
    Ok(Some(path))
}

/// Show the system "save as" dialog, starting in Downloads with a name made
/// from the text. The file type follows the extension: .mp3 (default) or .wav.
fn ask_where_to_save(app: &AppHandle, text: &str) -> Option<PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    let mut dialog = app
        .dialog()
        .file()
        .set_title("Save audio")
        .set_file_name(format!("{}.mp3", file_stem(text)))
        .add_filter("MP3 audio", &["mp3"])
        .add_filter("WAV audio", &["wav"]);
    if let Ok(downloads) = app.path().download_dir() {
        dialog = dialog.set_directory(downloads);
    }
    if let Some(window) = app.get_webview_window("settings") {
        if window.is_visible().unwrap_or(false) {
            dialog = dialog.set_parent(&window);
        }
    }
    let mut path = dialog.blocking_save_file()?.into_path().ok()?;
    let known = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp3") || e.eq_ignore_ascii_case("wav"));
    if !known {
        // Keep whatever the user typed and make it an MP3.
        let mut name = path.file_name()?.to_os_string();
        name.push(".mp3");
        path.set_file_name(name);
    }
    Some(path)
}

/// Encode mono speech as MP3 (96 kbps, plenty for a voice at 24 kHz).
fn encode_mp3(samples: &[f32]) -> Result<Vec<u8>, String> {
    use mp3lame_encoder::{Bitrate, Builder, FlushNoGap, MonoPcm, Quality};
    let failed = |what: &str| format!("MP3 encoding failed ({what})");

    let mut builder = Builder::new().ok_or_else(|| failed("start"))?;
    builder.set_num_channels(1).map_err(|_| failed("channels"))?;
    builder.set_sample_rate(SAMPLE_RATE).map_err(|_| failed("sample rate"))?;
    builder.set_brate(Bitrate::Kbps96).map_err(|_| failed("bitrate"))?;
    builder.set_quality(Quality::Best).map_err(|_| failed("quality"))?;
    let mut encoder = builder.build().map_err(|_| failed("setup"))?;

    let pcm: Vec<i16> = samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();
    let mut out = Vec::with_capacity(mp3lame_encoder::max_required_buffer_size(pcm.len()));
    encoder
        .encode_to_vec(MonoPcm(&pcm), &mut out)
        .map_err(|_| failed("encode"))?;
    out.reserve(7200);
    encoder
        .flush_to_vec::<FlushNoGap>(&mut out)
        .map_err(|_| failed("finish"))?;
    Ok(out)
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

        let speed: f32 = std::env::var("LOUDR_SPEED").map_or(1.0, |s| s.parse().unwrap());

        let started = Instant::now();
        let mut engine = Engine::load(&dir.join(MODEL_FILE), &dir.join(VOICES_FILE)).unwrap();
        println!("loaded in {:?}", started.elapsed());

        let mut samples = Vec::new();
        for chunk in text::chunks(&text) {
            let started = Instant::now();
            let audio = engine.synthesize(&chunk, &voice, speed).unwrap();
            println!(
                "{:?} for {:.1}s of audio: {chunk}",
                started.elapsed(),
                audio.len() as f32 / SAMPLE_RATE as f32
            );
            samples.extend(audio);
        }
        assert!(samples.len() > SAMPLE_RATE as usize);
        write_wav(Path::new(&out), &samples).unwrap();
        let mp3 = encode_mp3(&samples).unwrap();
        println!("mp3: {} bytes for {:.1}s", mp3.len(), samples.len() as f32 / SAMPLE_RATE as f32);
        std::fs::write(Path::new(&out).with_extension("mp3"), mp3).unwrap();
    }
}
