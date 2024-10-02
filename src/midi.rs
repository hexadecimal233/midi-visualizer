use crate::myconfig;

use midly::{num::u7, MetaMessage, MidiMessage, Timing, TrackEventKind};
use myconfig::*;
use sdl2::pixels::Color;
use std::{
    collections::{BTreeMap, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
};

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

fn get_track_color(seed: usize) -> Color {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let hash = hasher.finish();

    let r = ((hash >> 24) & 0xFF) as u8;
    let g = ((hash >> 16) & 0xFF) as u8;
    let b = ((hash >> 8) & 0xFF) as u8;

    Color::RGB(r, g, b)
}

#[derive(Debug, Clone, Copy)]
pub struct Note {
    // Basic props
    pub start_tick: i64, // 0 ~ 2^28-1
    pub duration: i32,   // 0 ~ 2^28-1
    pub key: u7,         // 0 ~ 127

    // Render props
    pub pressed: bool,
    pub pressed_ticks: i32,
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

    pub fn tick(&mut self, curr_tick: i64) -> bool {
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

    pub fn get_color(&self, track_color: Color) -> Color {
        let note_col = if *CUSTOM_NOTE_COL {
            *NOTE_COL
        } else {
            track_color
        };
        let pressed_col = if *CUSTOM_NOTE_COL {
            *NOTE_COL_PRESSED
        } else {
            track_color.invert()
        };

        if self.pressed {
            lerp_color(
                pressed_col,
                note_col,
                ezing::sine_out(self.easing_t_color()),
            )
        } else {
            note_col
        }
    }

    pub fn get_y(&self) -> f32 {
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

    pub fn get_width(&self) -> f32 {
        scene_to_screen(self.duration.into()) as f32
    }

    pub fn get_x(&self, curr_tick: i64) -> f32 {
        scene_to_screen_w_offset(self.start_tick - curr_tick) as f32
    }
}

#[derive(Debug, Clone)]
pub struct MIDI {
    pub mspq: BTreeMap<i64, u32>, // Actually a u24
    pub ppq: u32,                 // Actually a u15
    pub length: i64,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub notes: Vec<Note>,
    pub color: Color,
}

impl Track {
    fn new() -> Track {
        Track {
            notes: Vec::new(),
            color: *NOTE_COL,
        }
    }
}

impl MIDI {
    fn new(ppq: u32) -> MIDI {
        MIDI {
            tracks: Vec::new(),
            mspq: BTreeMap::from([(i64::MIN, 60_000_000 / 120)]),
            ppq,
            length: i64::MAX,
        }
    }

    pub fn get_current_tick_and_mspq(&self, curr_tick: i64) -> (i64, u32) {
        // Get the tick value of the closest previous MSPQ change
        let mut result = (0, 0);
        if let Some((&closest_tick, &mspq_value)) = self.mspq.range(..=curr_tick).next_back() {
            result = (
                {
                    if closest_tick < (-*TICKSCENE_WIDTH) as i64 {
                        0
                    } else {
                        closest_tick
                    }
                },
                mspq_value,
            );
        }

        result
    }
    pub fn from_midi(midly: &midly::Smf) -> MIDI {
        let mut midi = MIDI::new(match midly.header.timing {
            Timing::Metrical(ppq) => u16::from(ppq) as u32,
            _ => panic!("Unsupported timing"),
        });

        for m_track in &midly.tracks {
            let mut track = Track::new();
            let mut offset: i64 = 0;
            let mut pressed_keys: HashMap<u7, i64> = HashMap::new();

            for event in m_track {
                if event.delta != 0 {
                    offset += u32::from(event.delta) as i64;
                };

                match event.kind {
                    TrackEventKind::Meta(meta) => match meta {
                        MetaMessage::Tempo(tempo) => {
                            midi.mspq.insert(offset, u32::from(tempo));
                        }
                        MetaMessage::EndOfTrack => {
                            midi.length = offset;
                        }
                        _ => {}
                    },
                    TrackEventKind::Midi { channel, message } => match message {
                        MidiMessage::NoteOn { key, vel } => {
                            // Cache pressed keys, notes are marked off when vel = 0 or NoteOff is triggered
                            if vel == 0 && pressed_keys.contains_key(&key) {
                                track.notes.push(Note::new(
                                    *pressed_keys.get(&key).unwrap(),
                                    (offset - *pressed_keys.get(&key).unwrap()) as i32,
                                    key,
                                ));
                                pressed_keys.remove(&key);
                            } else {
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

            track.color = get_track_color(midi.tracks.len());
            midi.tracks.push(track);
        }

        if midi.mspq.len() == 1 {
            println!("WARNING: No tempo events found. Defaulting to 120 bpm.");
        }

        return midi;
    }
}
