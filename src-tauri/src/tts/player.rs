//! Plays synthesized speech through the default speakers.
//!
//! Audio is queued chunk by chunk, so the first sentence starts playing
//! while later ones are still being made.

use super::engine::SAMPLE_RATE;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// An open output stream. Dropping it stops playback at once.
pub struct Player {
    _stream: cpal::Stream,
    /// Mono samples at the device's sample rate, waiting to be played.
    queue: Arc<Mutex<VecDeque<f32>>>,
    device_rate: u32,
}

impl Player {
    pub fn open() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("No speakers found")?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("Speakers unavailable: {e}"))?;
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let config = supported.config();
        let q = queue.clone();
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(&device, config, q),
            SampleFormat::I16 => build_stream::<i16>(&device, config, q),
            SampleFormat::U16 => build_stream::<u16>(&device, config, q),
            SampleFormat::I32 => build_stream::<i32>(&device, config, q),
            other => return Err(format!("Unsupported speaker format: {other:?}")),
        }
        .map_err(|e| format!("Could not open speakers: {e}"))?;
        stream
            .play()
            .map_err(|e| format!("Could not start speakers: {e}"))?;
        Ok(Self {
            _stream: stream,
            queue,
            device_rate: supported.sample_rate(),
        })
    }

    /// Queue speech (mono, at the engine's sample rate) for playback.
    pub fn push(&self, samples: &[f32]) {
        let converted = resample(samples, SAMPLE_RATE, self.device_rate);
        self.queue.lock().unwrap().extend(converted);
    }

    /// Seconds of audio queued but not yet played.
    pub fn queued_secs(&self) -> f32 {
        self.queue.lock().unwrap().len() as f32 / self.device_rate as f32
    }

    /// Block until everything queued has been played, or `cancelled` says stop.
    pub fn finish(&self, cancelled: impl Fn() -> bool) {
        while self.queued_secs() > 0.0 && !cancelled() {
            std::thread::sleep(Duration::from_millis(30));
        }
        if !cancelled() {
            // Let the last buffer leave the sound card before the stream closes.
            std::thread::sleep(Duration::from_millis(150));
        }
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    queue: Arc<Mutex<VecDeque<f32>>>,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            let mut queue = queue.lock().unwrap();
            for frame in data.chunks_mut(channels) {
                // Silence when the queue runs dry (the next chunk is still
                // being synthesized).
                let sample = T::from_sample(queue.pop_front().unwrap_or(0.0));
                frame.fill(sample);
            }
        },
        |err| log::error!("speech: stream error: {err}"),
        None,
    )
}

/// Linear-interpolation resampling. Speech from the model has no content
/// near the top of its band, so this is clean enough going up to 44.1/48 kHz.
pub fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || samples.is_empty() {
        return samples.to_vec();
    }
    let step = from as f64 / to as f64;
    let len = (samples.len() as f64 / step) as usize;
    (0..len)
        .map(|i| {
            let pos = i as f64 * step;
            let index = pos as usize;
            let frac = (pos - index as f64) as f32;
            let a = samples[index];
            let b = samples.get(index + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}
