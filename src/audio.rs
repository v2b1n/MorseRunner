// Copyright (C) 2026 Vladimir Vecgailis.
// SPDX-License-Identifier: MPL-2.0
//! Native output adapter. Simulation remains at the original 11025 Hz / 512 samples;
//! only the final device-boundary signal is resampled, preserving all model timings.
use crate::{
    dsp::{BLOCK, RATE},
    simulation::Engine,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
pub struct Audio {
    pub mixer: Arc<Mutex<Engine>>,
    pub error: Arc<Mutex<Option<String>>>,
    pub device: String,
    _stream: cpal::Stream,
}
impl Audio {
    pub fn open() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("No output audio device")?;
        let name = device.name().unwrap_or_default();
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let mixer = Arc::new(Mutex::new(Engine::new(seed)));
        let error = Arc::new(Mutex::new(None));
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(
                &device,
                &config.clone().into(),
                mixer.clone(),
                error.clone(),
            ),
            cpal::SampleFormat::I16 => build::<i16>(
                &device,
                &config.clone().into(),
                mixer.clone(),
                error.clone(),
            ),
            cpal::SampleFormat::U16 => build::<u16>(
                &device,
                &config.clone().into(),
                mixer.clone(),
                error.clone(),
            ),
            f => return Err(format!("Unsupported audio format: {f:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            mixer,
            error,
            device: name,
            _stream: stream,
        })
    }
}
fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mixer: Arc<Mutex<Engine>>,
    error: Arc<Mutex<Option<String>>>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    let channels = config.channels as usize;
    let ratio = RATE as f64 / config.sample_rate.0 as f64;
    let mut buffer = [0.; BLOCK];
    let mut position = BLOCK;
    let mut frac = 0_f64;
    let mut left = 0_f32;
    let mut right = 0_f32;
    device.build_output_stream(
        config,
        move |out: &mut [T], _| {
            if let Ok(mut m) = mixer.lock() {
                for frame in out.chunks_mut(channels) {
                    while frac >= 1. {
                        left = right;
                        if position >= BLOCK {
                            buffer = m.render();
                            position = 0;
                        }
                        right = buffer[position];
                        position += 1;
                        frac -= 1.;
                    }
                    let value = T::from_sample(left + (right - left) * frac as f32);
                    frame.fill(value);
                    frac += ratio;
                }
            } else {
                out.fill(T::from_sample(0.));
            }
        },
        move |e| {
            if let Ok(mut err) = error.lock() {
                *err = Some(e.to_string());
            }
        },
        None,
    )
}
