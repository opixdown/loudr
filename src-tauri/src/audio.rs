//! Microphone capture with cpal, plus resampling to the 16 kHz mono that
//! Whisper expects.
//!
//! The input stream is only open while the hotkey is held, so the app uses no
//! CPU (and shows no mic indicator) when idle.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// An in-progress recording. Dropping it closes the microphone.
pub struct Recording {
    stream: cpal::Stream,
    /// Mono samples at the device's native sample rate.
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

/// How often the level callback fires while recording.
const LEVELS_PER_SECOND: u32 = 30;

/// Names of all available microphones.
pub fn input_devices() -> Vec<String> {
    let Ok(devices) = cpal::default_host().input_devices() else {
        return Vec::new();
    };
    devices.filter_map(|d| device_name(&d)).collect()
}

fn device_name(device: &cpal::Device) -> Option<String> {
    device.description().ok().map(|d| d.name().to_string())
}

/// Open a microphone and start recording.
///
/// `device` is a name from `input_devices()`; `None`, or a device that has
/// been unplugged, means the system default. `on_level` receives the loudness
/// (RMS, 0.0 to 1.0) about 30 times a second, for the waveform.
pub fn start(
    device: Option<&str>,
    on_level: impl Fn(f32) + Send + Clone + 'static,
) -> Result<Recording, String> {
    let host = cpal::default_host();
    let chosen = device.and_then(|wanted| {
        host.input_devices()
            .ok()?
            .find(|d| device_name(d).as_deref() == Some(wanted))
    });
    if device.is_some() && chosen.is_none() {
        log::warn!("audio: microphone {device:?} not found, using the default");
    }
    let device = chosen
        .or_else(|| host.default_input_device())
        .ok_or("No microphone found")?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("Microphone unavailable: {e}"))?;

    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let config = supported.config();
    let b = buffer.clone();
    let stream = match supported.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, config, b, on_level),
        SampleFormat::I16 => build_stream::<i16>(&device, config, b, on_level),
        SampleFormat::U16 => build_stream::<u16>(&device, config, b, on_level),
        SampleFormat::I32 => build_stream::<i32>(&device, config, b, on_level),
        other => return Err(format!("Unsupported microphone format: {other:?}")),
    }
    .map_err(|e| format!("Could not open microphone: {e}"))?;
    stream
        .play()
        .map_err(|e| format!("Could not start microphone: {e}"))?;

    Ok(Recording {
        stream,
        buffer,
        sample_rate: supported.sample_rate(),
    })
}

/// Build an input stream for sample type `T` that downmixes every frame to
/// mono f32, appends it to `buffer`, and reports the loudness.
fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
    on_level: impl Fn(f32) + Send + 'static,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let frames_per_level = (config.sample_rate / LEVELS_PER_SECOND).max(1) as usize;
    // Running sum of squares since the last level report.
    let (mut energy, mut frames) = (0.0f32, 0usize);

    device.build_input_stream(
        config,
        move |data: &[T], _| {
            let mut buffer = buffer.lock().unwrap();
            for frame in data.chunks(channels) {
                let sum: f32 = frame.iter().map(|&s| f32::from_sample_(s)).sum();
                let sample = sum / channels as f32;
                buffer.push(sample);

                energy += sample * sample;
                frames += 1;
                if frames >= frames_per_level {
                    on_level((energy / frames as f32).sqrt());
                    (energy, frames) = (0.0, 0);
                }
            }
        },
        |err| log::error!("audio: stream error: {err}"),
        None,
    )
}

impl Recording {
    /// Stop recording. Returns the raw mono samples and their sample rate.
    pub fn stop(self) -> (Vec<f32>, u32) {
        drop(self.stream); // closes the microphone
        let samples = std::mem::take(&mut *self.buffer.lock().unwrap());
        (samples, self.sample_rate)
    }
}

/// Convert mono audio at `from_rate` to 16 kHz.
///
/// Downsampling needs a low-pass filter first, otherwise frequencies above
/// 8 kHz fold back into the speech band as noise. We filter with a small
/// windowed-sinc FIR, then pick output samples by linear interpolation.
pub fn resample_to_16k(input: &[f32], from_rate: u32) -> Vec<f32> {
    if from_rate == WHISPER_SAMPLE_RATE || input.is_empty() {
        return input.to_vec();
    }
    let filtered = if from_rate > WHISPER_SAMPLE_RATE {
        low_pass(input, 7_200.0 / from_rate as f32)
    } else {
        input.to_vec()
    };

    let step = from_rate as f64 / WHISPER_SAMPLE_RATE as f64;
    let out_len = (input.len() as f64 / step) as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * step;
            let j = pos as usize;
            let frac = (pos - j as f64) as f32;
            let a = filtered[j];
            let b = *filtered.get(j + 1).unwrap_or(&a);
            a + (b - a) * frac
        })
        .collect()
}

/// Windowed-sinc low-pass filter. `cutoff` is a fraction of the sample rate.
fn low_pass(input: &[f32], cutoff: f32) -> Vec<f32> {
    const TAPS: usize = 63;
    const HALF: isize = (TAPS / 2) as isize;
    use std::f32::consts::PI;

    let mut kernel = [0.0f32; TAPS];
    for (i, k) in kernel.iter_mut().enumerate() {
        let n = i as f32 - HALF as f32;
        let sinc = if n == 0.0 {
            2.0 * cutoff
        } else {
            (2.0 * PI * cutoff * n).sin() / (PI * n)
        };
        let hamming = 0.54 - 0.46 * (2.0 * PI * i as f32 / (TAPS - 1) as f32).cos();
        *k = sinc * hamming;
    }
    let gain: f32 = kernel.iter().sum();
    kernel.iter_mut().for_each(|k| *k /= gain);

    let len = input.len() as isize;
    (0..len)
        .map(|i| {
            kernel
                .iter()
                .enumerate()
                .map(|(t, k)| {
                    // Clamp at the edges so the start and end don't fade.
                    let j = (i + t as isize - HALF).clamp(0, len - 1);
                    input[j as usize] * k
                })
                .sum()
        })
        .collect()
}

/// Write 16 kHz mono samples to a WAV file (used for the debug dump of the
/// last recording).
pub fn write_wav(path: &Path, samples: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: WHISPER_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        writer
            .write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}
