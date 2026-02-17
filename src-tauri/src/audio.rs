use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Device, Host};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("cannot resolve temp directory")]
    MissingTempDir,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("audio device error: {0}")]
    Device(String),
    #[error("audio stream error: {0}")]
    Stream(String),
    #[error("recording session is not active")]
    NotRecording,
    #[error("recording session already active")]
    AlreadyRecording,
}

pub struct AudioRecorder {
    recording: bool,
    current_file: Option<PathBuf>,
    stream: Option<cpal::Stream>,
    buffer: Option<Arc<Mutex<Vec<f32>>>>,
    input_level: Option<Arc<Mutex<f32>>>,
    source_sample_rate: u32,
    source_channels: u16,
    preferred_device_name: Option<String>,
    active_device_name: Option<String>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            recording: false,
            current_file: None,
            stream: None,
            buffer: None,
            input_level: None,
            source_sample_rate: 0,
            source_channels: 1,
            preferred_device_name: None,
            active_device_name: None,
        }
    }

    pub fn start(&mut self) -> Result<(), AudioError> {
        if self.recording {
            return Err(AudioError::AlreadyRecording);
        }

        let mut path = std::env::temp_dir();
        if path.as_os_str().is_empty() {
            return Err(AudioError::MissingTempDir);
        }

        path.push("quan_voice");
        fs::create_dir_all(&path)?;
        path.push("recording.wav");

        let host = cpal::default_host();
        let device = select_input_device(&host, self.preferred_device_name.as_deref())?;
        let supported = device
            .default_input_config()
            .map_err(|err| AudioError::Device(err.to_string()))?;

        let sample_format = supported.sample_format();
        let stream_config: cpal::StreamConfig = supported.into();
        let device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown Input Device".to_string());
        eprintln!(
            "audio input device: {} | {} Hz | {} ch | {:?}",
            device_name, stream_config.sample_rate.0, stream_config.channels, sample_format
        );
        let channels = stream_config.channels;
        let sample_rate = stream_config.sample_rate.0;
        let data = Arc::new(Mutex::new(Vec::<f32>::new()));
        let level = Arc::new(Mutex::new(0.0_f32));
        let data_for_callback = Arc::clone(&data);
        let level_for_callback = Arc::clone(&level);

        let err_fn = |err| {
            eprintln!("audio stream error: {err}");
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |input: &[f32], _| {
                        push_samples_f32(input, &data_for_callback, &level_for_callback);
                    },
                    err_fn,
                    None,
                )
                .map_err(|err| AudioError::Stream(err.to_string()))?,
            cpal::SampleFormat::I16 => device
                .build_input_stream(
                    &stream_config,
                    move |input: &[i16], _| {
                        push_samples_i16(input, &data_for_callback, &level_for_callback);
                    },
                    err_fn,
                    None,
                )
                .map_err(|err| AudioError::Stream(err.to_string()))?,
            cpal::SampleFormat::U16 => device
                .build_input_stream(
                    &stream_config,
                    move |input: &[u16], _| {
                        push_samples_u16(input, &data_for_callback, &level_for_callback);
                    },
                    err_fn,
                    None,
                )
                .map_err(|err| AudioError::Stream(err.to_string()))?,
            other => {
                return Err(AudioError::Stream(format!(
                    "unsupported input sample format: {other:?}"
                )));
            }
        };

        stream
            .play()
            .map_err(|err| AudioError::Stream(err.to_string()))?;

        self.recording = true;
        self.current_file = Some(path);
        self.stream = Some(stream);
        self.buffer = Some(data);
        self.input_level = Some(level);
        self.source_sample_rate = sample_rate;
        self.source_channels = channels;
        self.active_device_name = Some(device_name);

        Ok(())
    }

    pub fn stop(&mut self) -> Result<PathBuf, AudioError> {
        if !self.recording {
            return Err(AudioError::NotRecording);
        }

        self.recording = false;
        self.stream.take();
        if let Some(level) = &self.input_level {
            if let Ok(mut guard) = level.lock() {
                *guard = 0.0;
            }
        }

        let path = self
            .current_file
            .clone()
            .ok_or(AudioError::NotRecording)?;

        let input_samples = self
            .buffer
            .take()
            .ok_or(AudioError::NotRecording)?
            .lock()
            .map_err(|_| AudioError::Stream("audio buffer lock poisoned".to_string()))?
            .clone();
        self.input_level = None;

        let mono = to_mono(&input_samples, self.source_channels);
        let resampled = resample_linear(&mono, self.source_sample_rate, 16_000);
        let normalized = normalize_audio(&resampled);
        write_wav(path.as_path(), &normalized)?;

        Ok(path)
    }

    pub fn current_input_level(&self) -> f32 {
        let Some(level) = &self.input_level else {
            return 0.0;
        };

        match level.lock() {
            Ok(guard) => *guard,
            Err(_) => 0.0,
        }
    }

    pub fn set_preferred_device(&mut self, device_name: Option<String>) {
        self.preferred_device_name = device_name
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty() && name != "Default");
    }

    pub fn active_device_name(&self) -> Option<String> {
        self.active_device_name.clone()
    }

    pub fn list_input_devices() -> Result<Vec<String>, AudioError> {
        let host = cpal::default_host();
        let mut names = host
            .input_devices()
            .map_err(|err| AudioError::Device(err.to_string()))?
            .filter_map(|d| d.name().ok())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        Ok(names)
    }
}

fn select_input_device(host: &Host, preferred_name: Option<&str>) -> Result<Device, AudioError> {
    let default = host.default_input_device();

    let candidates: Vec<Device> = host
        .input_devices()
        .map_err(|err| AudioError::Device(err.to_string()))?
        .collect();

    if candidates.is_empty() {
        return default.ok_or_else(|| AudioError::Device("no input devices found".to_string()));
    }

    if let Some(preferred) = preferred_name {
        let preferred_norm = preferred.to_ascii_lowercase();
        if let Some(device) = candidates.iter().find(|device| {
            device
                .name()
                .map(|name| name.to_ascii_lowercase() == preferred_norm)
                .unwrap_or(false)
        }) {
            return Ok(device.clone());
        }
        if let Some(device) = candidates.iter().find(|device| {
            device
                .name()
                .map(|name| name.to_ascii_lowercase().contains(&preferred_norm))
                .unwrap_or(false)
        }) {
            return Ok(device.clone());
        }
    }

    default
        .or_else(|| candidates.into_iter().next())
        .ok_or_else(|| AudioError::Device("no usable input device found".to_string()))
}

fn push_samples_f32(input: &[f32], data: &Arc<Mutex<Vec<f32>>>, level: &Arc<Mutex<f32>>) {
    if input.is_empty() {
        return;
    }

    if let Ok(mut guard) = data.lock() {
        guard.extend_from_slice(input);
    }

    let rms = chunk_rms_f32(input);
    if let Ok(mut level_guard) = level.lock() {
        *level_guard = smooth_level(*level_guard, rms);
    }
}

fn push_samples_i16(input: &[i16], data: &Arc<Mutex<Vec<f32>>>, level: &Arc<Mutex<f32>>) {
    if input.is_empty() {
        return;
    }

    let mut sum_sq = 0.0_f32;
    let mut count = 0usize;
    if let Ok(mut guard) = data.lock() {
        for sample in input {
            let value = *sample as f32 / i16::MAX as f32;
            sum_sq += value * value;
            count += 1;
            guard.push(value);
        }
    }

    let rms = if count == 0 {
        0.0
    } else {
        (sum_sq / count as f32).sqrt()
    };
    if let Ok(mut level_guard) = level.lock() {
        *level_guard = smooth_level(*level_guard, rms);
    }
}

fn push_samples_u16(input: &[u16], data: &Arc<Mutex<Vec<f32>>>, level: &Arc<Mutex<f32>>) {
    if input.is_empty() {
        return;
    }

    let mut sum_sq = 0.0_f32;
    let mut count = 0usize;
    if let Ok(mut guard) = data.lock() {
        for sample in input {
            let value = (*sample as f32 - 32768.0) / 32768.0;
            sum_sq += value * value;
            count += 1;
            guard.push(value);
        }
    }

    let rms = if count == 0 {
        0.0
    } else {
        (sum_sq / count as f32).sqrt()
    };
    if let Ok(mut level_guard) = level.lock() {
        *level_guard = smooth_level(*level_guard, rms);
    }
}

fn chunk_rms_f32(input: &[f32]) -> f32 {
    if input.is_empty() {
        return 0.0;
    }
    let sum_sq = input.iter().fold(0.0_f32, |acc, v| acc + (*v * *v));
    (sum_sq / input.len() as f32).sqrt()
}

fn smooth_level(previous: f32, instant: f32) -> f32 {
    let attack = 0.45_f32;
    let decay = 0.85_f32;
    let blended = if instant > previous {
        previous * (1.0 - attack) + instant * attack
    } else {
        previous * decay + instant * (1.0 - decay)
    };
    blended.clamp(0.0, 1.0)
}

fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }

    let channels = channels as usize;
    let mut out = Vec::with_capacity(samples.len() / channels + 1);
    for frame in samples.chunks_exact(channels) {
        let sum: f32 = frame.iter().copied().sum();
        out.push(sum / channels as f32);
    }

    out
}

fn resample_linear(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate == 0 || source_rate == target_rate {
        return samples.to_vec();
    }

    let ratio = source_rate as f64 / target_rate as f64;
    let out_len = ((samples.len() as f64) / ratio).max(1.0).round() as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let left = src_pos.floor() as usize;
        let right = (left + 1).min(samples.len().saturating_sub(1));
        let frac = (src_pos - left as f64) as f32;
        let value = samples[left] * (1.0 - frac) + samples[right] * frac;
        out.push(value);
    }

    out
}

fn write_wav(path: &std::path::Path, samples: &[f32]) -> Result<(), AudioError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|err| AudioError::Io(std::io::Error::other(err.to_string())))?;

    for sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        writer
            .write_sample(pcm)
            .map_err(|err| AudioError::Io(std::io::Error::other(err.to_string())))?;
    }

    writer
        .finalize()
        .map_err(|err| AudioError::Io(std::io::Error::other(err.to_string())))?;

    Ok(())
}

fn normalize_audio(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let peak = samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0_f32, |acc, value| acc.max(value));

    if peak <= f32::EPSILON {
        return samples.to_vec();
    }

    let gain = (0.9 / peak).clamp(1.0, 8.0);
    samples
        .iter()
        .map(|s| (s * gain).clamp(-1.0, 1.0))
        .collect()
}
