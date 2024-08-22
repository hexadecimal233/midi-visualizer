extern crate lazy_static;
use std::sync::RwLock;

use config::Config;
use lazy_static::lazy_static;
use sdl2::pixels::Color;

lazy_static! {
    static ref CONFIG: RwLock<Config> = RwLock::new(
        Config::builder()
            .add_source(config::File::with_name("config"))
            .build()
            .unwrap()
    );
    pub static ref FILE: String = CONFIG
        .read()
        .unwrap()
        .get_string("system.midi_path")
        .unwrap();
    pub static ref NOTE_COL_PRESSED: Color = color_fromi32(
        CONFIG
            .read()
            .unwrap()
            .get::<i32>("visuals.note_color_pressed")
            .unwrap()
    );
    pub static ref NOTE_COL: Color = color_fromi32(
        CONFIG
            .read()
            .unwrap()
            .get::<i32>("visuals.note_color")
            .unwrap()
    );
    pub static ref BG_COL: Color = color_fromi32(
        CONFIG
            .read()
            .unwrap()
            .get::<i32>("visuals.background_color")
            .unwrap()
    );
    pub static ref CUR_IND_COL: Color = color_fromi32(
        CONFIG
            .read()
            .unwrap()
            .get::<i32>("visuals.current_tick_indicator_color")
            .unwrap()
    );
    pub static ref NOTE_HEIGHT: f32 = CONFIG
        .read()
        .unwrap()
        .get::<f32>("visuals.note_height")
        .unwrap();
    pub static ref NOTE_PADDING: f32 = CONFIG
        .read()
        .unwrap()
        .get::<f32>("visuals.note_padding_y")
        .unwrap();
    pub static ref FPS: u32 = CONFIG.read().unwrap().get::<u32>("system.fps").unwrap();
    pub static ref TICKSCENE_WIDTH: i32 = CONFIG
        .read()
        .unwrap()
        .get::<i32>("visuals.tickscene_width")
        .unwrap();
    pub static ref CUR_IND_LOC: i32 = CONFIG
        .read()
        .unwrap()
        .get::<i32>("visuals.current_tick_indicator_x_offset")
        .unwrap();
    pub static ref SCREEN_SIZE: RwLock<(u32, u32)> = {
        let wh = CONFIG
            .read()
            .unwrap()
            .get_table("system.window_size")
            .unwrap();
        RwLock::new((
            wh.get("w").unwrap().clone().into_uint().unwrap() as u32,
            wh.get("h").unwrap().clone().into_uint().unwrap() as u32,
        ))
    };
}

pub fn screen_size() -> (u32, u32) {
    (*SCREEN_SIZE).read().unwrap().to_owned()
}

pub fn screen_size_w(w: u32, h: u32) {
    let mut new_settings = SCREEN_SIZE.write().unwrap();
    *new_settings = (w, h);
}

fn color_fromi32(x: i32) -> Color {
    let color = x as u32;
    Color::RGB(
        (color >> 16 & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
    )
}
