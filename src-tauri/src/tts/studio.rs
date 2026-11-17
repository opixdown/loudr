//! Studio voices: optional, online text to speech through the user's own
//! account with ElevenLabs or Google Gemini.
//!
//! This is the one part of Loudr that sends text over the network, so it is
//! off until the user picks a provider and saves an API key, and it is only
//! used for "Script to audio". Reading a selection aloud always stays on the
//! built-in offline voice, so nothing selected in another app is ever sent.
//!
//! Both providers are asked for the same audio the offline voice produces
//! (mono, 24 kHz), so playback and saving work the same way for all three.

use super::engine::SAMPLE_RATE;
use super::player::resample;
use crate::settings;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// The longest piece of script sent in one request. Well under every
/// model's limit, and short enough that progress moves and Stop is quick.
const MAX_REQUEST_CHARS: usize = 2500;
/// The longest answer accepted (about half an hour of audio).
const MAX_RESPONSE_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq)]
pub enum Provider {
    ElevenLabs,
    Gemini,
}

impl Provider {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "elevenlabs" => Some(Self::ElevenLabs),
            "gemini" => Some(Self::Gemini),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::ElevenLabs => "elevenlabs",
            Self::Gemini => "gemini",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::ElevenLabs => "ElevenLabs",
            Self::Gemini => "Google Gemini",
        }
    }

    /// (id, label); the first is the default.
    pub fn models(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::ElevenLabs => &[
                ("eleven_multilingual_v2", "Multilingual v2 · steady"),
                ("eleven_v3", "v3 · expressive"),
                ("eleven_v4", "v4 · newest"),
                ("eleven_flash_v2_5", "Flash v2.5 · fast, cheap"),
            ],
            Self::Gemini => &[
                ("gemini-3.8-flash-tts", "3.8 Flash · best"),
                ("gemini-3.8-flash-lite-tts", "3.8 Flash-Lite · cheaper"),
            ],
        }
    }
}

/// Gemini's built-in voices: (name, character).
#[rustfmt::skip]
const GEMINI_VOICES: &[(&str, &str)] = &[
    ("Kore", "Firm"), ("Puck", "Upbeat"), ("Zephyr", "Bright"), ("Charon", "Informative"),
    ("Fenrir", "Excitable"), ("Leda", "Youthful"), ("Orus", "Firm"), ("Aoede", "Breezy"),
    ("Callirrhoe", "Easy-going"), ("Autonoe", "Bright"), ("Enceladus", "Breathy"),
    ("Iapetus", "Clear"), ("Umbriel", "Easy-going"), ("Algieba", "Smooth"),
    ("Despina", "Smooth"), ("Erinome", "Clear"), ("Algenib", "Gravelly"),
    ("Rasalgethi", "Informative"), ("Laomedeia", "Upbeat"), ("Achernar", "Soft"),
    ("Alnilam", "Firm"), ("Schedar", "Even"), ("Gacrux", "Mature"),
    ("Pulcherrima", "Forward"), ("Achird", "Friendly"), ("Zubenelgenubi", "Casual"),
    ("Vindemiatrix", "Gentle"), ("Sadachbia", "Lively"), ("Sadaltager", "Knowledgeable"),
    ("Sulafat", "Warm"),
];

#[derive(Serialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
}

// ---------------------------------------------------------------------------
// API keys
// ---------------------------------------------------------------------------
//
// Keys are kept in their own file, apart from settings.json, readable only
// by the user. They are never sent to the window: it is only told whether a
// key is saved.

fn keys_file(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .expect("no app config directory")
        .join("keys.json")
}

fn read_keys(app: &AppHandle) -> serde_json::Map<String, Value> {
    std::fs::read_to_string(keys_file(app))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn key(app: &AppHandle, provider: Provider) -> Option<String> {
    read_keys(app)
        .get(provider.id())
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|key| !key.is_empty())
}

/// Save a provider's key; an empty key removes it.
pub fn set_key(app: &AppHandle, provider: Provider, key: &str) -> Result<(), String> {
    let mut keys = read_keys(app);
    let key = key.trim();
    if key.is_empty() {
        keys.remove(provider.id());
    } else {
        keys.insert(provider.id().into(), Value::String(key.into()));
    }
    let path = keys_file(app);
    let text = serde_json::to_string_pretty(&keys).map_err(|e| e.to_string())?;
    let failed = |e: std::io::Error| format!("Could not save the key: {e}");
    std::fs::create_dir_all(path.parent().expect("keys file has a folder")).map_err(failed)?;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600); // this user only
    }
    use std::io::Write;
    options
        .open(&path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(failed)
}

// ---------------------------------------------------------------------------
// Speaking
// ---------------------------------------------------------------------------

/// Everything needed to make speech with the provider chosen in settings.
pub struct Studio {
    provider: Provider,
    key: String,
    voice: String,
    model: String,
}

/// The studio voice to use for scripts, if one is switched on and has a key.
pub fn current(app: &AppHandle) -> Option<Studio> {
    let s = settings::current(app);
    let provider = Provider::from_id(&s.studio_provider)?;
    let key = key(app, provider)?;
    let (voice, model) = match provider {
        Provider::ElevenLabs => (s.elevenlabs_voice, s.elevenlabs_model),
        Provider::Gemini => (s.gemini_voice, s.gemini_model),
    };
    let known = provider.models().iter().any(|(id, _)| *id == model);
    Some(Studio {
        provider,
        key,
        voice: match provider {
            Provider::Gemini if voice.is_empty() => GEMINI_VOICES[0].0.into(),
            _ => voice,
        },
        model: if known { model } else { provider.models()[0].0.into() },
    })
}

impl Studio {
    pub fn name(&self) -> &'static str {
        self.provider.name()
    }

    /// Identifies the voice, so audio made with it can be told apart from
    /// audio made with another voice or the offline one.
    pub fn tag(&self) -> String {
        format!("{}/{}/{}", self.provider.id(), self.model, self.voice)
    }

    /// Speak one piece of text. Returns mono samples at `SAMPLE_RATE`.
    pub fn synthesize(&self, text: &str) -> Result<Vec<f32>, String> {
        match self.provider {
            Provider::ElevenLabs => self.elevenlabs(text),
            Provider::Gemini => self.gemini(text),
        }
    }

    fn elevenlabs(&self, text: &str) -> Result<Vec<f32>, String> {
        if self.voice.is_empty() {
            return Err("Pick an ElevenLabs voice in Loudr's settings first.".into());
        }
        // Raw 16-bit samples at the rate the rest of Loudr works in.
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}?output_format=pcm_{SAMPLE_RATE}",
            self.voice
        );
        let body = json!({ "text": text, "model_id": self.model });
        let audio = post(self.provider, &url, "xi-api-key", &self.key, &body)?;
        Ok(pcm16(&audio))
    }

    fn gemini(&self, text: &str) -> Result<Vec<f32>, String> {
        let url = "https://generativelanguage.googleapis.com/v1beta/interactions";
        let body = json!({
            "model": self.model,
            "input": [{ "type": "user_input", "content": [{ "type": "text", "text": text }] }],
            "response_format": { "type": "audio" },
            "generation_config": { "speech_config": [{ "voice": self.voice }] },
        });
        let answer = post(self.provider, url, "x-goog-api-key", &self.key, &body)?;
        let answer: Value = serde_json::from_slice(&answer)
            .map_err(|_| "Google Gemini sent an answer Loudr could not read.".to_string())?;
        let encoded = find_audio(&answer)
            .ok_or("Google Gemini answered without any audio. Try a different model.")?;
        use base64::Engine;
        let audio = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| "Google Gemini sent audio Loudr could not read.".to_string())?;
        decode_audio(&audio)
    }
}

/// The voices a key can use, for the voice picker.
pub fn voices(app: &AppHandle, provider: Provider) -> Result<Vec<Choice>, String> {
    match provider {
        Provider::Gemini => Ok(GEMINI_VOICES
            .iter()
            .map(|(name, character)| Choice {
                id: (*name).into(),
                label: format!("{name} · {character}"),
            })
            .collect()),
        Provider::ElevenLabs => {
            let key = key(app, provider).ok_or("Save your ElevenLabs API key first.")?;
            let url = "https://api.elevenlabs.io/v2/voices?page_size=100&sort=name&sort_direction=asc";
            let answer = request(provider, ureq::get(url).header("xi-api-key", &key).call())?;
            let answer: Value = serde_json::from_slice(&answer)
                .map_err(|_| "ElevenLabs sent a voice list Loudr could not read.".to_string())?;
            let voices: Vec<Choice> = answer["voices"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|voice| {
                    let name = voice["name"].as_str()?;
                    let kind = voice["category"].as_str().unwrap_or("");
                    Some(Choice {
                        id: voice["voice_id"].as_str()?.into(),
                        label: if kind.is_empty() || kind == "premade" {
                            name.into()
                        } else {
                            format!("{name} · {kind}")
                        },
                    })
                })
                .collect();
            if voices.is_empty() {
                return Err("This ElevenLabs account has no voices yet. Add one on elevenlabs.io.".into());
            }
            Ok(voices)
        }
    }
}

/// Join sentence-sized chunks into pieces of up to `MAX_REQUEST_CHARS`, so a
/// long script takes a few requests instead of one per sentence (which
/// would cost more and make the voice restart its tone each time).
pub fn pieces(chunks: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for chunk in chunks {
        match out.last_mut() {
            Some(last) if last.chars().count() + chunk.chars().count() < MAX_REQUEST_CHARS => {
                last.push(' ');
                last.push_str(chunk);
            }
            _ => out.push(chunk.clone()),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Network and audio plumbing
// ---------------------------------------------------------------------------

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        // Error answers carry the provider's explanation; read them too.
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(180)))
        .build()
        .into()
}

fn post(provider: Provider, url: &str, key_header: &str, key: &str, body: &Value) -> Result<Vec<u8>, String> {
    let body = serde_json::to_vec(body).map_err(|e| e.to_string())?;
    let sent = agent()
        .post(url)
        .header(key_header, key)
        .header("content-type", "application/json")
        .send(&body[..]);
    request(provider, sent)
}

/// Read an answer, turning failures into a sentence the user can act on.
fn request(
    provider: Provider,
    sent: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<Vec<u8>, String> {
    let name = provider.name();
    let mut response = sent.map_err(|e| format!("Could not reach {name}: {e}"))?;
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES)
        .read_to_vec()
        .map_err(|e| format!("The answer from {name} was cut off: {e}"))?;
    if (200..300).contains(&status) {
        return Ok(body);
    }
    let said = error_message(&body);
    log::warn!("studio: {name} answered {status}: {said}");
    let about_key = said.to_lowercase().contains("api key");
    Err(match status {
        401 | 403 => format!("{name} did not accept the API key. {said}"),
        400 if about_key => format!("{name} did not accept the API key. {said}"),
        429 => format!("{name} says you are over your limit or out of credit. {said}"),
        _ => format!("{name} could not do that (error {status}). {said}"),
    }
    .trim()
    .to_string())
}

/// The human-readable part of a provider's error answer.
fn error_message(body: &[u8]) -> String {
    let Ok(json) = serde_json::from_slice::<Value>(body) else {
        return String::from_utf8_lossy(body).chars().take(200).collect();
    };
    // Google sometimes wraps its answer in a list.
    let json = json.get(0).unwrap_or(&json);
    // ElevenLabs: {"detail": {"message": ..}} or {"detail": ".."};
    // Google: {"error": {"message": ..}}.
    [&json["detail"]["message"], &json["detail"], &json["error"]["message"], &json["message"]]
        .into_iter()
        .find_map(Value::as_str)
        .unwrap_or("")
        .chars()
        .take(300)
        .collect()
}

/// Find base64 audio anywhere in an answer: an object marked as audio that
/// has a "data" string. Searching, rather than following one fixed path,
/// keeps this working across the provider's API versions.
fn find_audio(value: &Value) -> Option<&str> {
    match value {
        Value::Object(map) => {
            let mime = ["mime_type", "mimeType"].iter().find_map(|k| map.get(*k)?.as_str());
            let is_audio = map.get("type").and_then(Value::as_str) == Some("audio")
                || mime.is_some_and(|m| m.starts_with("audio/"));
            match map.get("data").and_then(Value::as_str) {
                Some(data) if is_audio => Some(data),
                _ => map.values().rev().find_map(find_audio),
            }
        }
        Value::Array(items) => items.iter().rev().find_map(find_audio),
        _ => None,
    }
}

/// Audio as a WAV file, or bare 16-bit samples at `SAMPLE_RATE`.
fn decode_audio(bytes: &[u8]) -> Result<Vec<f32>, String> {
    if !bytes.starts_with(b"RIFF") {
        return Ok(pcm16(bytes));
    }
    let unreadable = |_| "The audio that came back could not be read.".to_string();
    let mut reader = hound::WavReader::new(std::io::Cursor::new(bytes)).map_err(unreadable)?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>(),
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|s| s as f32 / scale))
                .collect::<Result<_, _>>()
        }
    }
    .map_err(unreadable)?;
    // Mix to mono, then bring to Loudr's rate.
    let channels = spec.channels.max(1) as usize;
    let mono: Vec<f32> = samples
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect();
    Ok(resample(&mono, spec.sample_rate, SAMPLE_RATE))
}

/// Little-endian 16-bit samples to floats.
fn pcm16(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]) as f32 / i16::MAX as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Call a real provider. Needs an account, so it is opt-in:
    ///   LOUDR_STUDIO=elevenlabs|gemini LOUDR_KEY=<api key> LOUDR_OUT=out.wav \
    ///     [LOUDR_VOICE=<voice id>] [LOUDR_MODEL=<model id>] [LOUDR_TEXT=..] \
    ///     cargo test studio_speaks -- --ignored --nocapture
    #[test]
    #[ignore = "needs a provider account"]
    fn studio_speaks() {
        let var = |name: &str| std::env::var(name).ok();
        let provider = Provider::from_id(&var("LOUDR_STUDIO").expect("set LOUDR_STUDIO")).unwrap();
        let studio = Studio {
            provider,
            key: var("LOUDR_KEY").expect("set LOUDR_KEY"),
            voice: var("LOUDR_VOICE").unwrap_or(match provider {
                Provider::Gemini => "Kore".into(),
                Provider::ElevenLabs => "21m00Tcm4TlvDq8ikWAM".into(),
            }),
            model: var("LOUDR_MODEL").unwrap_or(provider.models()[0].0.into()),
        };
        let text = var("LOUDR_TEXT").unwrap_or("Hello from Loudr. This voice came over the network.".into());
        match studio.synthesize(&text) {
            Ok(samples) => {
                println!("{:.1} s of audio", samples.len() as f32 / SAMPLE_RATE as f32);
                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate: SAMPLE_RATE,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };
                let mut out = hound::WavWriter::create(var("LOUDR_OUT").expect("set LOUDR_OUT"), spec).unwrap();
                for s in samples {
                    out.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).unwrap();
                }
                out.finalize().unwrap();
            }
            Err(e) => println!("failed: {e}"),
        }
    }

    #[test]
    fn finds_audio_in_both_answer_shapes() {
        let interactions = json!({ "steps": [
            { "type": "thought", "content": [] },
            { "type": "model_output", "content": [{ "type": "audio", "mime_type": "audio/wav", "data": "QUJD" }] },
        ]});
        assert_eq!(find_audio(&interactions), Some("QUJD"));
        let generate = json!({ "candidates": [{ "content": { "parts": [
            { "inlineData": { "mimeType": "audio/L16;rate=24000", "data": "REVG" } },
        ]}}]});
        assert_eq!(find_audio(&generate), Some("REVG"));
        assert_eq!(find_audio(&json!({ "data": "not audio" })), None);
    }

    #[test]
    fn reads_wav_and_bare_samples() {
        let mut wav = Vec::new();
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(std::io::Cursor::new(&mut wav), spec).unwrap();
        for _ in 0..4800 {
            writer.write_sample(16_384i16).unwrap();
            writer.write_sample(16_384i16).unwrap();
        }
        writer.finalize().unwrap();
        let decoded = decode_audio(&wav).unwrap();
        // A tenth of a second of stereo 48 kHz becomes mono 24 kHz.
        assert_eq!(decoded.len(), 2400);
        assert!((decoded[100] - 0.5).abs() < 0.01);

        assert_eq!(decode_audio(&[0, 0x40, 0, 0xC0]).unwrap().len(), 2);
    }

    #[test]
    fn error_answers_become_sentences() {
        assert_eq!(
            error_message(br#"{"detail":{"status":"quota_exceeded","message":"Out of credits"}}"#),
            "Out of credits"
        );
        assert_eq!(
            error_message(br#"[{"error":{"code":400,"message":"API key not valid"}}]"#),
            "API key not valid"
        );
    }

    #[test]
    fn sentences_are_grouped_into_few_requests() {
        let chunks: Vec<String> = (0..60).map(|i| format!("Sentence number {i} is here.")).collect();
        let grouped = pieces(&chunks);
        assert!(grouped.len() < 4 && grouped.iter().all(|p| p.chars().count() <= MAX_REQUEST_CHARS));
        assert!(grouped[0].starts_with("Sentence number 0 is here. Sentence number 1"));
    }
}
