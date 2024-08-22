#![allow(unused_variables, dead_code)]

extern crate config;
extern crate ezing;
extern crate lazy_static;
extern crate sdl2;

mod audio;
mod myconfig;

use midly::{num::u7, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use myconfig::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::FRect;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

fn main() -> Result<(), String> {
    // Initializing SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let sine_waves = audio::init_audio(&sdl_context)?;

    let window = video_subsystem
        .window("midi-visualizer", screen_size().0, screen_size().1)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    canvas.set_draw_color(*BG_COL);
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;

    let midi_data = std::fs::read(FILE.clone()).unwrap();
    let midi = Smf::parse(&midi_data).unwrap();
    let mut track = Track::from_midi(&midi);

    let frame_interval_nano = 1_000_000_000 / *FPS;
    let mut curr_tick: i64 = (-*TICKSCENE_WIDTH).into(); // Let notes appear from the right side of the screen

    let start_time = Instant::now();

    println!(
        "Notes: {}, Micros per quarter note: {}, BPM: {}",
        track.notes.len(),
        track.mspq,
        60_000_000 / track.mspq
    );

    // Main Loop
    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                Event::Window {
                    timestamp,
                    window_id,
                    win_event,
                } => match win_event {
                    sdl2::event::WindowEvent::Resized(w, h) => {
                        // TODO: Resize event also applys to the scene
                        canvas.window_mut().set_size(w as u32, h as u32).unwrap();
                        screen_size_w(w as u32, h as u32);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        canvas.clear();

        for note in track.notes.iter_mut() {
            if note.tick(curr_tick) && note.pressed_ticks == 1 {
                // Don't generate sine real time or it will bug

                let sine_wave = sine_waves.get(&note.key).unwrap();
                sdl2::mixer::Channel::all().play(&sine_wave, 0)?;
            }

            let note_x = note.get_x(curr_tick);
            let note_w = note.get_width();

            // Skip out-of-screen notes to improve performance
            if note_x + note_w >= 0.0 && note_x < screen_size().0 as f32 {
                canvas.set_draw_color(note.get_color());

                canvas.fill_frect(FRect::new(
                    note_x,
                    note.get_y(),
                    note_w,
                    *NOTE_HEIGHT - *NOTE_PADDING * 2.0,
                ))?;
            }
        }

        canvas.set_draw_color(*CUR_IND_COL);
        canvas.fill_frect(FRect::new(
            *CUR_IND_LOC as f32,
            0.0,
            2.0,
            screen_size().1 as f32,
        ))?;

        // TODO: Tick text
        // println!("tick: {}:", curr_tick);

        canvas.set_draw_color(*BG_COL);
        canvas.present();

        // MSPT = MSPQ / PPQ

        curr_tick = (-*TICKSCENE_WIDTH) as i64
            + (Instant::now() - start_time).as_micros() as i64 * track.ppq as i64
                / track.mspq as i64;

        std::thread::sleep(Duration::new(0, frame_interval_nano));
    }

    sdl2::mixer::Music::halt();
    Ok(())
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::RGB(
        lerp(a.r as f32, b.r as f32, t) as u8,
        lerp(a.g as f32, b.g as f32, t) as u8,
        lerp(a.b as f32, b.b as f32, t) as u8,
    )
}

fn scene_to_screen_w_offset(x: i64) -> f64 {
    let scale = screen_size().0 as f64 / *TICKSCENE_WIDTH as f64;
    x as f64 * scale + *CUR_IND_LOC as f64
}

fn scene_to_screen(x: i64) -> f64 {
    let scale = screen_size().0 as f64 / *TICKSCENE_WIDTH as f64;
    x as f64 * scale
}

// MIDI Processing part

#[derive(Debug, Clone, Copy)]
struct Note {
    // Basic props
    start_tick: i64, // 0 ~ 2^28-1
    duration: i32,   // 0 ~ 2^28-1
    key: u7,         // 0 ~ 127

    // Render props
    pressed: bool,
    pressed_ticks: i32,
}

impl Note {
    fn new(start_tick: i64, duration: i32, key: u7) -> Note {
        Note {
            start_tick,
            duration,
            key,
            pressed: false,
            pressed_ticks: 0,
        }
    }

    fn tick(&mut self, curr_tick: i64) -> bool {
        let pressed =
            self.start_tick <= curr_tick && curr_tick < self.start_tick + self.duration as i64;
        if pressed {
            self.pressed = true;
            self.pressed_ticks += 1;
            return true;
        } else {
            self.pressed = false;
            self.pressed_ticks = 0;
            return false;
        }
    }

    // Gets the easing t between 0 and 1
    fn easing_t_color(&self) -> f32 {
        f32::clamp(self.pressed_ticks as f32 / 20.0, 0.0, 1.0)
    }

    fn easing_t(&self) -> f32 {
        f32::clamp(
            -(self.pressed_ticks as f32 / 10.0 - 1.0).abs() + 1.0,
            0.0,
            1.0,
        )
    }

    fn get_color(&self) -> Color {
        if self.pressed {
            lerp_color(
                *NOTE_COL_PRESSED,
                *NOTE_COL,
                ezing::sine_out(self.easing_t_color()),
            )
        } else {
            *NOTE_COL
        }
    }

    fn get_y(&self) -> f32 {
        let note_h = (screen_size().1 / 0x7f) as f32; // We have 128 keys
        return screen_size().1 as f32 - (u8::from(self.key) as f32 * note_h)
            + *NOTE_PADDING
            + self.easing_offset();
    }

    fn easing_offset(&self) -> f32 {
        if self.pressed {
            return lerp(0.0, *NOTE_PADDING * 2.0, ezing::sine_out(self.easing_t()));
        } else {
            return 0.0;
        }
    }

    fn get_width(&self) -> f32 {
        scene_to_screen(self.duration.into()) as f32
    }

    fn get_x(&self, curr_tick: i64) -> f32 {
        scene_to_screen_w_offset(self.start_tick - curr_tick) as f32
    }
}

struct Track {
    mspq: u32, // Actually a u24
    ppq: u32,  // Actually a u15
    notes: Vec<Note>,
}

impl Track {
    fn from_midi(midi: &midly::Smf) -> Track {
        let mut track = Track {
            notes: Vec::new(),
            mspq: 0xffffffff,
            ppq: match midi.header.timing {
                Timing::Metrical(ppq) => u16::from(ppq) as u32,
                _ => panic!("Unsupported timing"),
            },
        };

        for m_track in &midi.tracks {
            let mut offset: i64 = 0;
            let mut pressed_keys: HashMap<u7, i64> = HashMap::new();
            for event in m_track {
                if event.delta != 0 {
                    offset += u32::from(event.delta) as i64;
                };

                match event.kind {
                    TrackEventKind::Meta(meta) => match meta {
                        MetaMessage::Tempo(tempo) => {
                            if track.mspq != 0xffffffff {
                                // TODO: Dynamic tempo support
                                println!("Tempo already set, ignoring {}.", tempo);
                                break;
                            }

                            println!("Set tempo to {} bpm.", tempo);
                            track.mspq = u32::from(tempo);
                        }
                        _ => {}
                    },
                    TrackEventKind::Midi { channel, message } => match message {
                        MidiMessage::NoteOn { key, vel } => {
                            // Cache pressed keys
                            if vel == 0 && pressed_keys.contains_key(&key) {
                                track.notes.push(Note::new(
                                    *pressed_keys.get(&key).unwrap(),
                                    (offset - *pressed_keys.get(&key).unwrap()) as i32,
                                    key,
                                ));
                                pressed_keys.remove(&key);
                            } else {
                                pressed_keys.insert(key, offset);
                                pressed_keys.insert(key, offset);
                                pressed_keys.insert(key, offset);
                            }
                        }
                        MidiMessage::NoteOff { key, vel } => {
                            if pressed_keys.contains_key(&key) {
                                track.notes.push(Note::new(
                                    *pressed_keys.get(&key).unwrap(),
                                    (offset - *pressed_keys.get(&key).unwrap()) as i32,
                                    key,
                                ));
                                pressed_keys.remove(&key);
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        if track.mspq == 0xffffffff {
            println!("No tempo events found. Defaulting to 120 bpm.");
            track.mspq = 60_000_000 / 120;
        }

        return track;
    }
}
