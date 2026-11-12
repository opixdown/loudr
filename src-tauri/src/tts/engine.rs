//! The Kokoro voice model: text -> phonemes -> audio, all on this computer.

use super::text;
use misaki_rs::fallback::FallbackError;
use misaki_rs::{Fallback, G2P, Language};
use ort::environment::Environment;
use ort::session::Session;
use ort::value::Tensor;
use std::collections::HashMap;
use std::path::Path;

/// Kokoro produces mono audio at this rate.
pub const SAMPLE_RATE: u32 = 24_000;
/// Longest phoneme sequence the model accepts.
const MAX_PHONEMES: usize = 510;
/// Numbers in one voice's style row.
const STYLE_SIZE: usize = 256;

/// Words the pronunciation dictionary doesn't know are sounded out by rule.
struct SoundOut;
impl Fallback for SoundOut {
    fn phonemize(&self, word: &str) -> Result<String, FallbackError> {
        Ok(text::guess_pronunciation(word))
    }
}

pub struct Engine {
    // Declared before the environment so it is dropped first.
    session: Session,
    _environment: Environment,
    /// Name of the model's phoneme input ("tokens" or "input_ids",
    /// depending on who exported the file).
    tokens_input: String,
    /// Voice name -> style table, one row of 256 numbers per phoneme count.
    voices: HashMap<String, Vec<f32>>,
    /// Phoneme character -> model token.
    vocab: HashMap<char, i64>,
    american: G2P,
    /// Only built when a British voice is first used.
    british: Option<G2P>,
}

impl Engine {
    pub fn load(model: &Path, voices: &Path) -> Result<Self, String> {
        let environment = ort::init()
            .with_name("loudr")
            .with_telemetry(false)
            .build()
            .map_err(|e| format!("Speech engine: {e}"))?;
        let session = Session::builder(&environment)
            .and_then(|mut b| b.commit_from_file(model))
            .map_err(|e| format!("Could not load the voice model: {e}"))?;
        let tokens_input = session
            .inputs()
            .first()
            .map(|i| i.name().to_string())
            .ok_or("Voice model has no inputs")?;

        let vocab: HashMap<String, i64> =
            serde_json::from_str(include_str!("vocab.json")).expect("vocab.json is valid");
        let vocab = vocab
            .into_iter()
            .filter_map(|(k, v)| Some((k.chars().next()?, v)))
            .collect();

        Ok(Self {
            session,
            _environment: environment,
            tokens_input,
            voices: read_voices(voices)?,
            vocab,
            american: G2P::with_fallback(Language::EnglishUS, Some(Box::new(SoundOut))),
            british: None,
        })
    }

    /// Speak one chunk of text (a sentence or so). Returns mono samples at
    /// `SAMPLE_RATE`.
    pub fn synthesize(&mut self, text: &str, voice: &str, speed: f32) -> Result<Vec<f32>, String> {
        // Voice names start with the accent: "a" American, "b" British.
        let g2p = if voice.starts_with('b') {
            self.british.get_or_insert_with(|| {
                G2P::with_fallback(Language::EnglishGB, Some(Box::new(SoundOut)))
            })
        } else {
            &self.american
        };
        let (phonemes, _) = g2p.g2p(text).map_err(|e| format!("Could not read text: {e}"))?;

        let mut tokens: Vec<i64> = phonemes
            .chars()
            .filter_map(|c| self.vocab.get(&c).copied())
            .take(MAX_PHONEMES)
            .collect();
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        // The style depends on how long the utterance is.
        let style_table = self
            .voices
            .get(voice)
            .ok_or_else(|| format!("Voice \"{voice}\" not found"))?;
        let row = tokens.len().min(style_table.len() / STYLE_SIZE - 1);
        let style = style_table[row * STYLE_SIZE..(row + 1) * STYLE_SIZE].to_vec();

        // The model expects a padding token on both ends.
        tokens.insert(0, 0);
        tokens.push(0);

        let run = || -> ort::Result<Vec<f32>> {
            let outputs = self.session.run(ort::inputs![
                self.tokens_input.as_str() => Tensor::from_array(([1, tokens.len()], tokens))?,
                "style" => Tensor::from_array(([1, STYLE_SIZE], style))?,
                "speed" => Tensor::from_array(([1usize], vec![speed]))?,
            ])?;
            let (_, audio) = outputs[0].try_extract_tensor::<f32>()?;
            Ok(audio.to_vec())
        };
        let audio = run().map_err(|e| format!("Speech failed: {e}"))?;
        Ok(even_pauses(&audio))
    }
}

/// The model leaves an uneven stretch of silence before and after each
/// chunk, which makes sentences sound disconnected when chunks are played
/// back to back. Cut that silence and end with one short, fixed pause.
fn even_pauses(audio: &[f32]) -> Vec<f32> {
    const QUIET: f32 = 0.005;
    // Kept around the speech so soft first and last sounds aren't clipped.
    const MARGIN: usize = SAMPLE_RATE as usize * 70 / 1000;
    const PAUSE: usize = SAMPLE_RATE as usize * 260 / 1000;

    let Some(first) = audio.iter().position(|s| s.abs() > QUIET) else {
        return Vec::new();
    };
    let last = audio.iter().rposition(|s| s.abs() > QUIET).unwrap_or(first);
    let start = first.saturating_sub(MARGIN);
    let end = (last + MARGIN).min(audio.len());
    let mut out = audio[start..end].to_vec();
    out.resize(out.len() + PAUSE, 0.0);
    out
}

/// Read the voices file: a zip archive (stored, not compressed) with one
/// NumPy array per voice. Both formats are simple enough to read directly.
fn read_voices(path: &Path) -> Result<HashMap<String, Vec<f32>>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read voices: {e}"))?;
    parse_voices(&bytes).ok_or_else(|| "The voices file is damaged. Download it again.".into())
}

fn parse_voices(b: &[u8]) -> Option<HashMap<String, Vec<f32>>> {
    let u16_at = |o: usize| Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?) as usize);
    let u32_at = |o: usize| Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?) as usize);

    // The directory of entries is located by a record at the end of the file.
    let end = (0..=b.len().checked_sub(22)?)
        .rev()
        .find(|&i| &b[i..i + 4] == b"PK\x05\x06")?;
    let count = u16_at(end + 10)?;
    let mut entry = u32_at(end + 16)?;

    let mut voices = HashMap::new();
    for _ in 0..count {
        if b.get(entry..entry + 4)? != b"PK\x01\x02" || u16_at(entry + 10)? != 0 {
            return None; // not a directory entry, or compressed
        }
        let size = u32_at(entry + 20)?;
        let name_len = u16_at(entry + 28)?;
        let local = u32_at(entry + 42)?;
        let name = std::str::from_utf8(b.get(entry + 46..entry + 46 + name_len)?).ok()?;
        entry += 46 + name_len + u16_at(entry + 30)? + u16_at(entry + 32)?;

        let data = local + 30 + u16_at(local + 26)? + u16_at(local + 28)?;
        let npy = b.get(data..data + size)?;
        // NumPy file: 10 bytes of magic and lengths, a text header, then
        // little-endian f32 values.
        if npy.get(..6)? != b"\x93NUMPY" {
            return None;
        }
        let header_len = u16::from_le_bytes(npy.get(8..10)?.try_into().ok()?) as usize;
        let values = npy
            .get(10 + header_len..)?
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect::<Vec<f32>>();
        if values.len() < STYLE_SIZE {
            return None;
        }
        voices.insert(name.trim_end_matches(".npy").to_string(), values);
    }
    (!voices.is_empty()).then_some(voices)
}
