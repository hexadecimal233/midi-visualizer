#![allow(unused_variables, dead_code)]

extern crate config;
extern crate ezing;
extern crate lazy_static;
extern crate sdl2;

mod audio;
mod midi;
mod myconfig;

use midly::Smf;
use myconfig::*;
use sdl2::keyboard::Keycode;
use sdl2::rect::FRect;
use sdl2::{event::Event, render::TextureQuery};
use std::time::{Duration, Instant};

fn main() -> Result<(), String> {
    // Initializing SDL2
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let ttf_context = sdl2::ttf::init().unwrap();
    let mut font = ttf_context.load_font(
        r#"E:\SteamLibrary\steamapps\common\Teardown\data\ui\font\RobotoMono-Regular.ttf"#,
        32,
    )?;
    font.set_style(sdl2::ttf::FontStyle::NORMAL);

    let sine_waves = audio::init_audio(&sdl_context)?;

    let window = video_subsystem
        .window("midi-visualizer", screen_size().0, screen_size().1)
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    canvas.set_draw_color(*BG_COL);
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;

    let midi_data = std::fs::read(FILE.clone()).unwrap();
    let mut midi = midi::MIDI::from_midi(&Smf::parse(&midi_data).unwrap());

    let frame_interval_nano = 1_000_000_000 / *FPS;
    let mut curr_tick: i64 = (-*TICKSCENE_WIDTH).into(); // Let notes appear from the right side of the screen

    let mut last_mspq_tick: i64 = 0;
    let mut last_mspq_change = Instant::now();

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
                        canvas.window_mut().set_size(w as u32, h as u32).unwrap();
                        screen_size_w(w as u32, h as u32);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        canvas.clear();

        // last_mspq last_mspq_tick = 0, so always trigger on first played
        let (tick_before_curr_mspq, mspq) = midi.get_current_mspq_and_tick(curr_tick);

        if tick_before_curr_mspq != last_mspq_tick {
            last_mspq_tick = tick_before_curr_mspq;
            last_mspq_change = Instant::now();
        }

        for track in midi.tracks.iter_mut() {
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
                    canvas.set_draw_color(note.get_color(track.color));

                    canvas.fill_frect(FRect::new(
                        note_x,
                        note.get_y(),
                        note_w,
                        *NOTE_HEIGHT - *NOTE_PADDING * 2.0,
                    ))?;
                }
            }
        }

        canvas.set_draw_color(*CUR_IND_COL);
        canvas.fill_frect(FRect::new(
            *CUR_IND_LOC as f32,
            0.0,
            2.0,
            screen_size().1 as f32,
        ))?;

        let surface = font
            .render(&format!(
                "Total notes: {}\nCurrent tick: {}\nBPM: {:.3}",
                midi.tracks.len(),
                curr_tick.to_string(),
                get_bpm(mspq)
            ))
            .blended(*TEXT_COL)
            .unwrap();

        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .unwrap();

        let TextureQuery { width, height, .. } = texture.query();
        let target = FRect::new(0.0, 0.0, width as f32, height as f32);
        canvas.copy_f(&texture, None, Some(target))?;

        canvas.set_draw_color(*BG_COL);
        canvas.present();

        // MSPT = MSPQ / PPQ

        let elapsed_time =
            ((Instant::now() - last_mspq_change).as_micros() as f64 * *PLAYBACK_SPD) as i64;
        let tick_after = elapsed_time * midi.ppq as i64 / mspq as i64;

        curr_tick = tick_before_curr_mspq + tick_after;

        // End of track
        if curr_tick >= midi.length {
            break 'main;
        }

        std::thread::sleep(Duration::new(0, frame_interval_nano));
    }

    sdl2::mixer::Music::halt();
    Ok(())
}

fn get_bpm(mspq: u32) -> f64 {
    60_000_000.0 / mspq as f64
}
