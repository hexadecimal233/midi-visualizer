use std::collections::HashMap;

use midly::num::u7;
use sdl2::{
    mixer::{Chunk, InitFlag, AUDIO_S16LSB, DEFAULT_CHANNELS},
    Sdl,
};

const SAMPLERATE: i32 = 44_100;
const VOLUME: f32 = 0.1; // Between 0.0 and 1.0

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
pub fn init_audio(sdl_context: &Sdl) -> Result<HashMap<u7, Chunk>, String> {
    let format = AUDIO_S16LSB; // signed 16 bit samples, in little-endian byte order
    let channels = DEFAULT_CHANNELS; // Stereo
    let chunk_size = 1_024;

    std::thread::spawn(move || {});
    sdl_context.audio()?;
    sdl2::mixer::open_audio(SAMPLERATE, format, channels, chunk_size)?;
    sdl2::mixer::init(InitFlag::all())?;
    sdl2::mixer::allocate_channels(256);

    let sine_waves = (0..127)
        .map(|key| ((key as u8).into(), get_square_wave(0.5, key))) // This can also be changed to other wave shapes
        .collect();

    Ok(sine_waves)
}

pub fn get_sine_wave(
    duration: f32, // seconds
    key: i32,      // midi key id, 0-127
) -> Chunk {
    let freq = 440.0 * 2.0_f64.powf((key as f64 - 69.0) / 12.0);
    let sample_count = (SAMPLERATE as f32 * duration) as u32;

    println!(
        "playing note {}: freq = {}, sample_count = {}",
        key, freq, sample_count
    );

    let buffer = (0..sample_count)
        .map(|t| {
            let pluck_factor = lerp(1.0, 0.0, t as f32 / sample_count as f32);

            (pluck_factor
                * VOLUME
                * i16::MAX as f32
                * (2.0 * std::f32::consts::PI * freq as f32 * (t as f32 / SAMPLERATE as f32)).sin())
                as i16
        })
        .collect();
    sdl2::mixer::Chunk::from_raw_buffer(buffer)
        .map_err(|e| format!("Cannot get chunk from buffer: {:?}", e))
        .unwrap()
}

pub fn get_square_wave(
    duration: f32, // seconds
    key: i32,      // midi key id, 0-127
) -> Chunk {
    let freq = 440.0 * 2.0_f64.powf((key as f64 - 69.0) / 12.0);
    let sample_count = (SAMPLERATE as f32 * duration) as u32;

    println!(
        "playing note {}: freq = {}, sample_count = {}",
        key, freq, sample_count
    );

    let buffer = (0..sample_count)
        .map(|t| {
            let pluck_factor = lerp(1.0, 0.0, t as f32 / sample_count as f32);

            let cycle_pos = t as f32 * freq as f32 / SAMPLERATE as f32;
            let phase = cycle_pos % 1.0;
            if phase < 0.5 {
                (i16::MAX as f32 * VOLUME * pluck_factor) as i16
            } else {
                (-i16::MAX as f32 * VOLUME * pluck_factor) as i16
            }
        })
        .collect();

    sdl2::mixer::Chunk::from_raw_buffer(buffer)
        .map_err(|e| format!("Cannot get chunk from buffer: {:?}", e))
        .unwrap()
}
