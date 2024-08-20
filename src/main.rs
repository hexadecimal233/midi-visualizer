#![allow(unused_variables)]

extern crate sdl2;

use midly::{num::u7, MetaMessage, MidiMessage, Smf, TrackEventKind};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::FRect;
use std::{collections::HashMap, time::Duration};

const NOTE_COL_PRESSED: Color = Color::RGB(255, 255, 255);
const NOTE_COL: Color = Color::RGB(200, 200, 200);
const BG_COL: Color = Color::RGB(0, 0, 0);
const NOTE_HEIGHT: f32 = 5.0;
const NOTE_BORDER: f32 = 1.0;
const FPS: u32 = 60;
const SCREEN_SIZE: (u32, u32) = (800, 640); // 5:4
const TICKSCENE_WIDTH: i32 = 10000;
const CUR_IND_LOC: i32 = SCREEN_SIZE.0 as i32 / 2;
const CUR_IND_COL: Color = Color::RGB(255, 0, 0);

fn main() -> Result<(), String> {
    let midi_data = std::fs::read("C:\\Users\\User\\Downloads\\csd.mid").unwrap();
    let midi = Smf::parse(&midi_data).unwrap();
    let track = Track::from_midi(&midi);

    let frame_interval_nano = 1_000_000_000 / FPS;
    let mut curr_tick = -TICKSCENE_WIDTH;
    // TODO: Figure out how TPQN actually works :thinking:
    let ticks_per_frame = 1 * (frame_interval_nano / 1000) / 1000; // MIDI ticks is based on microseconds, so we convert nanos to micros

    println!(
        "Notes: {}\nTicks per quarter note: {}",
        track.notes.len(),
        track.tpqn
    );

    // track::Track::from_midi(&midi);

    // Initializing SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("midi-visualizer", SCREEN_SIZE.0, SCREEN_SIZE.1)
        .position_centered()
        //.resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    canvas.set_draw_color(BG_COL);
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;

    // Main Loop
    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                _ => {}
            }
        }

        canvas.clear();

        for note in track.notes.iter() {
            if note.is_pressed(curr_tick) {
                canvas.set_draw_color(NOTE_COL_PRESSED);
            } else {
                canvas.set_draw_color(NOTE_COL);
            }

            canvas.fill_frect(FRect::new(
                note.get_x(curr_tick),
                note.get_y(),
                note.get_width(),
                NOTE_HEIGHT - NOTE_BORDER * 2.0,
            ))?;
        }

        canvas.set_draw_color(CUR_IND_COL);
        canvas.fill_frect(FRect::new(
            CUR_IND_LOC as f32,
            0.0,
            2.0,
            SCREEN_SIZE.1 as f32,
        ))?;

        println!("tick: {}:", curr_tick);

        canvas.set_draw_color(BG_COL);
        canvas.present();

        curr_tick += ticks_per_frame as i32;
        std::thread::sleep(Duration::new(0, frame_interval_nano));
    }

    Ok(())
}

fn scene_to_screen_w_offset(x: i32) -> f64 {
    let scale = SCREEN_SIZE.0 as f64 / TICKSCENE_WIDTH as f64;
    x as f64 * scale + CUR_IND_LOC as f64
}

fn scene_to_screen(x: i32) -> f64 {
    let scale = SCREEN_SIZE.0 as f64 / TICKSCENE_WIDTH as f64;
    x as f64 * scale
}

// MIDI Processing part

#[derive(Debug, Clone, Copy)]
struct Note {
    start_tick: i32, // 0 ~ 2^28-1
    duration: i32,   // 0 ~ 2^28-1
    key: u7,         // 0 ~ 127
}

impl Note {
    fn new(start_tick: i32, duration: i32, key: u7) -> Note {
        Note {
            start_tick,
            duration,
            key,
        }
    }

    fn is_pressed(&self, curr_tick: i32) -> bool {
        self.start_tick <= curr_tick && curr_tick < self.start_tick + self.duration
    }

    fn get_y(&self) -> f32 {
        let note_h = (SCREEN_SIZE.1 / 0x7f) as f32; // We have 128 keys
        return SCREEN_SIZE.1 as f32 - (u8::from(self.key) as f32 * note_h) + NOTE_BORDER;
    }

    fn get_width(&self) -> f32 {
        scene_to_screen(self.duration) as f32
    }

    fn get_x(&self, curr_tick: i32) -> f32 {
        scene_to_screen_w_offset(self.start_tick - curr_tick) as f32
    }
}

struct Track {
    tpqn: u32, // Actually a u24
    notes: Vec<Note>,
}

impl Track {
    fn from_midi(midi: &midly::Smf) -> Track {
        let mut track = Track {
            notes: Vec::new(),
            tpqn: 0xffffffff,
        };

        let mut pressed_keys: HashMap<u7, i32> = HashMap::new();
        let mut offset = 0;

        for m_track in &midi.tracks {
            for event in m_track {
                if event.delta != 0 {
                    offset += u32::from(event.delta) as i32;
                };

                match event.kind {
                    TrackEventKind::Meta(meta) => match meta {
                        MetaMessage::Tempo(tempo) => {
                            if track.tpqn != 0xffffffff {
                                // TODO: Dynamic tempo support
                                println!("Tempo already set, ignoring.");
                                break;
                            }

                            println!("Set tempo to {} bpm.", tempo);
                            track.tpqn = u32::from(tempo);
                        }
                        _ => {}
                    },
                    TrackEventKind::Midi { channel, message } => match message {
                        MidiMessage::NoteOn { key, vel } => {
                            // Cache pressed keys
                            pressed_keys.insert(key, offset);
                        }
                        MidiMessage::NoteOff { key, vel } => {
                            if pressed_keys.contains_key(&key) {
                                track.notes.push(Note::new(
                                    *pressed_keys.get(&key).unwrap(),
                                    offset - *pressed_keys.get(&key).unwrap(),
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

        if track.tpqn == 0xffffffff {
            println!("No tempo events found. Defaulting to 120 bpm.");
            track.tpqn = 60_000_000 / 120;
        }

        return track;
    }
}
