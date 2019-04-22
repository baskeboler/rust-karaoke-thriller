extern crate colors;
extern crate find_folder;
extern crate freetype as ft;
extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate serde_json;
mod lyrics;
// use graphics::character::CharacterCache;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::*;
// use graphics::*;
// use opengl_graphics::graphics::character::CharacterCache;
// use opengl_graphics::{GlGraphics, OpenGL, GlyphCache};

// use graphics::glyph_cache::rusttype::GlyphCache;
use colors::{Color, HslaColorType, RgbaColorType};
use lyrics::{LyricProgressEvent, LyricsDisplay, LyricsFrame};
use piston::event_loop::*;
use piston::input::*;
use piston::window::WindowSettings;
use serde::de::Deserialize;

struct App<G> {
    gl: G,
    rotation: f64,
    frame_count: u64,
    fps: f64,
    color: RgbaColorType,
    lyrics: LyricsDisplay,
    frames: Vec<LyricsFrame>,
    currentFrame: i8,

    start: chrono::DateTime<chrono::Local>,
}

impl App<GlGraphics> {
    fn calc_fps(&mut self, dt: f64) {
        self.fps = 1.0 / dt; //((self.fps * 4.0) + (1.0 / dt)) / 5.0;
    }
    fn render_lyrics<C>(&mut self, args: RenderArgs, cache: &mut C)
    where
        C: graphics::character::CharacterCache<Texture = opengl_graphics::Texture, Error = String>,
    {
        use graphics::*;
        let (fst, snd) = self.lyrics.consumed_text();
        let black: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
        let red: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        let w1 = cache.width(24, fst).expect("width of processed text");
        let w2 = cache.width(24, snd).expect("width of unprocessed text");
        println!("rendering text (w1={:?}, w2={:?})", w1, w2);

        self.gl.draw(args.viewport(), |c, gl| {
            let transform = c.transform.trans(20.0, 200.0);
            text(red, 24, fst, cache, transform, gl).unwrap();
            text(black, 24, snd, cache, transform.trans(w1, 0.0), gl).unwrap();
        });
    }
    fn render<C>(&mut self, args: &RenderArgs, cache: &mut C)
    where
        C: graphics::character::CharacterCache<Texture = opengl_graphics::Texture, Error = String>,
    {
        use graphics::*;
        let obj = self.color;
        let primary_color: [f32; 4] = obj.into(); //[0.0, 1.0, 0.0, 1.0];
        let secondary_color: [f32; 4] = obj.complement().into();
        let SQUARE_EDGE_SIZE: f64 = 100.0;

        let square = rectangle::square(0.0, 0.0, SQUARE_EDGE_SIZE);
        let rotation = self.rotation;
        let (x, y) = (args.width / 2.0, args.height / 2.0);
        // let ref mut cache = self.glyph_cache;
        self.frame_count += 1;
        self.calc_fps(args.ext_dt);
        if self.frame_count % 50 == 0 {
            println!("fps; {:?}", self.fps);
        }
        let fps = self.fps;
        self.gl.draw(args.viewport(), |c, gl| {
            clear(primary_color, gl);
            let transform = c
                .transform
                .trans(x, y)
                .rot_rad(rotation)
                .trans(-0.5 * SQUARE_EDGE_SIZE, -0.5 * SQUARE_EDGE_SIZE);
            rectangle(secondary_color, square, transform, gl);
            // character::CharacterCache

            text(
                secondary_color,
                24,
                format!("FPS: {:?}", fps).as_str(),
                cache,
                c.transform.trans(100.0, 100.0),
                gl,
            )
            .unwrap();
            text(
                secondary_color,
                24,
                format!("DT: {:?}", args.ext_dt).as_str(),
                cache,
                c.transform.trans(100.0, 130.0),
                gl,
            )
            .unwrap();
        });
        self.render_lyrics(*args, cache);
    }

    fn update(&mut self, args: &UpdateArgs) {
        use chrono::Duration;
        self.rotation += 2.0 * args.dt;
        let offset: Duration = chrono::Local::now() - self.start;
        let millis: f64 = offset.num_milliseconds() as f64;

        let active: Vec<&LyricsFrame> = self.frames.iter().filter(|f| f.offset < millis).collect();
        let t = active
            .last()
            .map(|l| l.text.clone())
            .or(Some(String::new()))
            .unwrap();
        let progress = active
            .last()
            .map(|l| {
                l.event_offsets
                    .iter()
                    .filter(|o| o.offset < millis as f32)
                    .map(|o| o.char_count)
                    .sum()
            })
            .or(Some(0))
            .unwrap();
        self.lyrics.text = t;
        self.lyrics.sing_progress.store(progress as usize, std::sync::atomic::Ordering::SeqCst);
    }
    fn key(&mut self, e: &Button) {
        match e {
            Button::Keyboard(Key::Up) => {
                println!("pressed up!");
                let mut c: HslaColorType = self.color.into();
                c.h += 10.0;
                self.color = c.into();
            }
            Button::Keyboard(Key::Down) => {
                println!("pressed down!");
                let mut c: HslaColorType = self.color.into();
                c.h -= 10.0;
                self.color = c.into();
            }
            Button::Keyboard(Key::Right) => {
                println!("pressed up!");
                let mut c: HslaColorType = self.color.into();
                c.s += 10.0;
                self.color = c.into();
            }
            Button::Keyboard(Key::Left) => {
                println!("pressed down!");
                let mut c: HslaColorType = self.color.into();
                c.s -= 10.0;
                self.color = c.into();
            }
            Button::Keyboard(Key::Space) => {
                println!("advance lyrics!");
                self.lyrics.advance();
            }
            _ => println!("pressed {:?}", e),
        }
    }
}

fn main() {
    println!("Hello, world!");

    let opengl = OpenGL::V4_5;
    let lyrics: Vec<LyricsFrame> =
        std::fs::read_to_string("/home/victor/dev/clj-karaoke/song.json")
            .map(|s| serde_json::de::from_str(s.as_str()).unwrap())
            .unwrap();
    println!("{:?}", lyrics);
    let mut window: Window = WindowSettings::new("spinning square", [400, 400])
        .opengl(opengl)
        .exit_on_esc(true)
        .vsync(true)
        .build()
        .unwrap();
    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .unwrap();
    let font = assets.join("Bangers-Regular.ttf");
    // let factory = window.window.factory.clone();
    let mut app = App {
        gl: GlGraphics::new(opengl),
        rotation: 0.0,
        frame_count: 0,
        fps: 0.0,
        color: RgbaColorType::new(0.2, 0.45, 0.3),
        lyrics: LyricsDisplay::new("hola como te va? uno dos tres cuatro cinco seis siete"),
        frames: lyrics,
        start: chrono::Local::now(),
        currentFrame: -1,
    };
    let mut glyph_cache = GlyphCache::new(font, (), TextureSettings::new()).unwrap();

    // let mut cache =;
    let settings = EventSettings::new().max_fps(60);
    let mut events = Events::new(settings);
    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&r, &mut glyph_cache);
        }

        if let Some(u) = e.update_args() {
            app.update(&u);
        }
        if let Some(e) = e.press_args() {
            app.key(&e);
        }
    }
}
