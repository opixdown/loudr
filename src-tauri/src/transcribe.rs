//! Whisper transcription. The model is loaded once and kept in memory so a
//! transcription starts the moment the hotkey is released.

use crate::audio::WHISPER_SAMPLE_RATE;
use std::path::Path;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

pub struct Transcriber {
    /// The state keeps the model alive and reuses its buffers between runs.
    state: WhisperState,
}

impl Transcriber {
    pub fn load(model_path: &Path) -> Result<Self, String> {
        if !model_path.exists() {
            return Err(format!("Model missing: {}", model_path.display()));
        }
        // Send whisper.cpp's own (very chatty) output to the log crate
        // instead of stderr. Safe to call more than once.
        whisper_rs::install_logging_hooks();

        let ctx = WhisperContext::new_with_params(
            model_path,
            WhisperContextParameters::default(), // GPU (Metal) on when compiled in
        )
        .map_err(|e| format!("Could not load model: {e}"))?;
        let state = ctx
            .create_state()
            .map_err(|e| format!("Could not start Whisper: {e}"))?;

        let mut transcriber = Self { state };
        // The first run pays one-off costs (GPU shader compilation, buffer
        // allocation). Pay them now, on silence, not on the user's first clip.
        let _ = transcriber.transcribe(&vec![0.0; WHISPER_SAMPLE_RATE as usize], "en");
        Ok(transcriber)
    }

    /// Transcribe 16 kHz mono audio. Returns trimmed text, possibly empty.
    pub fn transcribe(&mut self, audio: &[f32], language: &str) -> Result<String, String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        // "auto" makes Whisper detect the language (multilingual models only).
        params.set_language(Some(language));
        params.set_n_threads(4);
        params.set_no_context(true); // each clip is independent
        params.set_no_timestamps(true);
        params.set_suppress_blank(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Whisper rejects clips shorter than one second, so pad with silence.
        let min_len = (WHISPER_SAMPLE_RATE as f32 * 1.1) as usize;
        let padded;
        let audio = if audio.len() < min_len {
            padded = [audio, &vec![0.0; min_len - audio.len()]].concat();
            &padded[..]
        } else {
            audio
        };

        self.state
            .full(params, audio)
            .map_err(|e| format!("Transcription failed: {e}"))?;

        let mut text = String::new();
        for segment in self.state.as_iter() {
            text.push_str(&segment.to_str_lossy().map_err(|e| e.to_string())?);
        }
        let text = text.trim();

        // On silence Whisper emits markers like "[BLANK_AUDIO]" or "(silence)".
        let is_marker = (text.starts_with('[') && text.ends_with(']'))
            || (text.starts_with('(') && text.ends_with(')'));
        Ok(if is_marker { String::new() } else { text.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::resample_to_16k;
    use std::time::Instant;

    /// End-to-end check of resampling + transcription against a real clip.
    /// Needs a model and a WAV file, so it is opt-in:
    ///   LOUDR_MODEL=/path/ggml-base.en.bin LOUDR_WAV=/path/clip.wav \
    ///     cargo test -- --ignored --nocapture
    #[test]
    #[ignore]
    fn transcribes_wav_file() {
        let model = std::env::var("LOUDR_MODEL").expect("set LOUDR_MODEL");
        let wav = std::env::var("LOUDR_WAV").expect("set LOUDR_WAV");

        let mut reader = hound::WavReader::open(wav).unwrap();
        let rate = reader.spec().sample_rate;
        let raw: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / i16::MAX as f32)
            .collect();

        let t = Instant::now();
        let mut transcriber = Transcriber::load(Path::new(&model)).unwrap();
        println!("load + warm-up: {:?}", t.elapsed());

        let t = Instant::now();
        let audio = resample_to_16k(&raw, rate);
        println!("resample {rate} Hz -> 16 kHz: {:?}", t.elapsed());

        for run in 1..=3 {
            let t = Instant::now();
            let text = transcriber.transcribe(&audio, "en").unwrap();
            println!("run {run}: {:?} -> {text}", t.elapsed());
            assert!(!text.is_empty());
        }
    }
}
