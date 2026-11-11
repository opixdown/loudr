//! The worker thread that turns hotkey events into typed text:
//! press -> record, release -> resample -> transcribe -> paste.
//!
//! Everything runs on one thread so the microphone stream and the Whisper
//! state never need to cross threads, and the hotkey listener never blocks.

use crate::audio::{self, Recording, WHISPER_SAMPLE_RATE};
use crate::hotkey::HotkeyEvent;
use crate::status::{self, Status};
use crate::transcribe::Transcriber;
use crate::{models, paste, permissions, settings};
use std::sync::mpsc::{self, Sender};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

pub enum Command {
    /// A hotkey event and the moment it happened (for latency timing).
    Hotkey(HotkeyEvent, Instant),
    /// The model or language in settings changed, or a download finished.
    ReloadModel,
}

/// Handle for sending commands to the pipeline thread, managed by Tauri.
pub struct Pipeline(Sender<Command>);

pub fn send(app: &AppHandle, command: Command) {
    let _ = app.state::<Pipeline>().0.send(command);
}

/// Clips shorter than this are treated as an accidental tap and ignored.
const MIN_CLIP_SECS: f32 = 0.3;

pub fn spawn(app: &AppHandle) {
    let (tx, rx) = mpsc::channel();
    app.manage(Pipeline(tx));

    let app = app.clone();
    std::thread::Builder::new()
        .name("pipeline".into())
        .spawn(move || {
            // Loaded once and kept in memory so transcription starts instantly.
            let mut transcriber = load_model(&app);
            let mut recording: Option<Recording> = None;

            for command in rx {
                match command {
                    Command::Hotkey(HotkeyEvent::Pressed, _) => {
                        match start_recording(&app, transcriber.is_some()) {
                            Ok(r) => {
                                recording = Some(r);
                                status::set(&app, Status::Recording);
                            }
                            Err(e) => status::set(&app, Status::Error(e)),
                        }
                    }
                    Command::Hotkey(HotkeyEvent::Released, released_at) => {
                        let Some(recording) = recording.take() else {
                            continue;
                        };
                        status::set(&app, Status::Transcribing);
                        match dictate(&app, recording, transcriber.as_mut(), released_at) {
                            Ok(()) => status::set(&app, Status::Idle),
                            Err(e) => status::set(&app, Status::Error(e)),
                        }
                    }
                    Command::ReloadModel => {
                        drop(transcriber.take()); // free the old model first
                        transcriber = load_model(&app);
                    }
                }
            }
        })
        .expect("failed to spawn pipeline thread");
}

fn load_model(app: &AppHandle) -> Option<Transcriber> {
    let s = settings::current(app);
    let path = models::path(app, &s.model, &s.language);
    if !path.exists() {
        // Normal on first run; the first-run window offers the download.
        log::warn!("model not downloaded yet: {}", path.display());
        return None;
    }
    let started = Instant::now();
    match Transcriber::load(&path) {
        Ok(t) => {
            log::info!(
                "model {} loaded and warmed up in {:?}",
                models::file_name(&s.model, &s.language),
                started.elapsed()
            );
            Some(t)
        }
        Err(e) => {
            status::set(app, Status::Error(e));
            None
        }
    }
}

fn start_recording(app: &AppHandle, model_loaded: bool) -> Result<Recording, String> {
    // Fail before recording, not after the user has spoken a whole sentence.
    if !model_loaded {
        return Err("No model yet. Open Loudr settings to download one.".into());
    }
    if permissions::microphone_status() == "denied" {
        return Err("Microphone permission denied. Allow Loudr in system settings.".into());
    }
    let device = settings::current(app).input_device;
    let pill = app.clone();
    audio::start(device.as_deref(), move |level| {
        let _ = pill.emit_to("pill", "level", level);
    })
}

/// Everything that happens after the hotkey is released.
fn dictate(
    app: &AppHandle,
    recording: Recording,
    transcriber: Option<&mut Transcriber>,
    released_at: Instant,
) -> Result<(), String> {
    let ms = |from: Instant| from.elapsed().as_secs_f32() * 1000.0;

    let t = Instant::now();
    let (raw, sample_rate) = recording.stop();
    let capture_stop_ms = ms(t);

    let clip_secs = raw.len() as f32 / sample_rate as f32;
    if clip_secs < MIN_CLIP_SECS {
        log::info!("clip too short ({clip_secs:.2}s), ignored");
        return Ok(());
    }
    // A stream of exact zeros means the OS is feeding us silence because
    // microphone access is blocked.
    if raw.iter().all(|&s| s == 0.0) {
        return Err("Microphone is silent. Check microphone permission.".into());
    }

    let t = Instant::now();
    let audio = audio::resample_to_16k(&raw, sample_rate);
    let resample_ms = ms(t);

    let transcriber = transcriber.ok_or("No model loaded")?;
    let language = settings::current(app).language;
    let t = Instant::now();
    let text = transcriber.transcribe(&audio, &language)?;
    let inference_ms = ms(t);

    let t = Instant::now();
    if !text.is_empty() {
        // Trailing space so back-to-back dictations don't run together.
        paste::paste(app, &format!("{text} "))?;
    }
    let paste_ms = ms(t);

    log::info!(
        "timing: audio={:.1}s capture_stop={capture_stop_ms:.0}ms resample={resample_ms:.0}ms \
         inference={inference_ms:.0}ms paste={paste_ms:.0}ms total={:.0}ms chars={}",
        audio.len() as f32 / WHISPER_SAMPLE_RATE as f32,
        ms(released_at),
        text.len(),
    );

    // Debug aid: keep the last clip on disk, after the text is delivered.
    let wav = models::dir(app).with_file_name("last-recording.wav");
    if let Err(e) = audio::write_wav(&wav, &audio) {
        log::warn!("could not write {}: {e}", wav.display());
    }
    Ok(())
}
