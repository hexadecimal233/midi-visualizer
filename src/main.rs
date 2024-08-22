#![allow(unused_variables)]

extern crate ezing;
extern crate sdl2;

mod audio;

use midly::{num::u7, MetaMessage, MidiMessage, Smf, TrackEventKind};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::FRect;
use std::{collections::HashMap, time::Duration};

const NOTE_BR_PRESSED: u8 = 255;
const NOTE_BR: u8 = 150;
const NOTE_COL: Color = Color::RGB(NOTE_BR, NOTE_BR, NOTE_BR);
const CUR_IND_COL: Color = Color::RGB(255, 0, 0);
const BG_COL: Color = Color::RGB(0, 0, 0);
const NOTE_HEIGHT: f32 = 10.0;
const NOTE_PADDING: f32 = 1.0;
const FPS: u32 = 60;
const TICKSCENE_WIDTH: i32 = 768; // Lower means notes are larger
const CUR_IND_LOC: i32 = 200; // Offsets the current indicator
const SCREEN_SIZE: (u32, u32) = (800, 800); // 5:4
const FILE: &str = r#"csd.mid"#;

fn main() -> Result<(), String> {
    let midi_data = std::fs::read(FILE).unwrap();
    let midi = Smf::parse(&midi_data).unwrap();
    let mut track = Track::from_midi(&midi);

    let frame_interval_nano = 1_000_000_000 / FPS;
    let mut curr_tick = -TICKSCENE_WIDTH; // Let notes appear from the right side of the screen
    // TODO: Figure out how TPQN actually works :thinking:
    // 375000: 375000 microseconds per quarter note, 7500000 * 4 * 60 
    let ticks_per_frame = track.tpqn / (frame_interval_nano / 1000) / 6; // MIDI ticks is based on microseconds, so we convert nanos to micros

    println!(
        "Notes: {}\nTicks per quarter note: {}",
        track.notes.len(),
        track.tpqn
    );

    // track::Track::from_midi(&midi);

    // Initializing SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let sine_waves = audio::init_audio(&sdl_context)?;

    let window = video_subsystem
        .window("midi-visualizer", SCREEN_SIZE.0, SCREEN_SIZE.1)
        .position_centered()
        .resizable()
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
                Event::Window {
                    timestamp,
                    window_id,
                    win_event,
                } => match win_event {
                    sdl2::event::WindowEvent::Resized(w, h) => {
                        // TODO: Resize event also applys to the scene
                        canvas.window_mut().set_size(w as u32, h as u32).unwrap();
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

            canvas.set_draw_color(note.get_color());

            canvas.fill_frect(FRect::new(
                note.get_x(curr_tick),
                note.get_y(),
                note.get_width(),
                NOTE_HEIGHT - NOTE_PADDING * 2.0,
            ))?;
        }

        canvas.set_draw_color(CUR_IND_COL);
        canvas.fill_frect(FRect::new(
            CUR_IND_LOC as f32,
            0.0,
            2.0,
            SCREEN_SIZE.1 as f32,
        ))?;

        // TODO: Tick text
        // println!("tick: {}:", curr_tick);

        canvas.set_draw_color(BG_COL);
        canvas.present();

        curr_tick += ticks_per_frame as i32;
        std::thread::sleep(Duration::new(0, frame_interval_nano));
    }

    sdl2::mixer::Music::halt();
    Ok(())
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
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
    // Basic props
    start_tick: i32, // 0 ~ 2^28-1
    duration: i32,   // 0 ~ 2^28-1
    key: u7,         // 0 ~ 127

    // Render props
    pressed: bool,
    pressed_ticks: i32,
}

impl Note {
    fn new(start_tick: i32, duration: i32, key: u7) -> Note {
        Note {
            start_tick,
            duration,
            key,
            pressed: false,
            pressed_ticks: 0,
        }
    }

    fn tick(&mut self, curr_tick: i32) -> bool {
        let pressed = self.start_tick <= curr_tick && curr_tick < self.start_tick + self.duration;
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
            let color = lerp(
                NOTE_BR_PRESSED as f32,
                NOTE_BR as f32,
                ezing::sine_out(self.easing_t_color()),
            ) as u8;
            Color::RGB(color, color, color)
        } else {
            NOTE_COL
        }
    }

    fn get_y(&self) -> f32 {
        let note_h = (SCREEN_SIZE.1 / 0x7f) as f32; // We have 128 keys
        return SCREEN_SIZE.1 as f32 - (u8::from(self.key) as f32 * note_h)
            + NOTE_PADDING
            + self.easing_offset();
    }

    fn easing_offset(&self) -> f32 {
        if self.pressed {
            return lerp(0.0, NOTE_PADDING * 2.0, ezing::sine_out(self.easing_t()));
        } else {
            return 0.0;
        }
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
                        // FIXME: Some files just dont load
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
