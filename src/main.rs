extern crate colors;
extern crate find_folder;
extern crate freetype as ft;
extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;
// extern crate piston_window;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate rodio;
#[macro_use]
extern crate serde_json;
extern crate ai_behavior;
extern crate sprite;

mod lyrics;
// use graphics::character::CharacterCache;
// use graphics::*;
// use opengl_graphics::graphics::character::CharacterCache;
// use opengl_graphics::{GlGraphics, OpenGL, GlyphCache};
use std::rc::Rc;
// use graphics::glyph_cache::rusttype::GlyphCache;
use ai_behavior::*;
use chrono::{DateTime, Duration, Local};
use colors::{Color, HslaColorType, RgbaColorType};
use glutin_window::GlutinWindow as Window;
use lyrics::{LyricProgressEvent, LyricsDisplay, LyricsFrame};
use opengl_graphics::*;
use piston::event_loop::*;
use piston::input::*;
use piston::window::WindowSettings;
use serde::de::Deserialize;
use sprite::*;
struct App<G> {
    gl: G,
    rotation: f64,
    frame_count: u64,
    fps: f64,
    color: RgbaColorType,
    lyrics: LyricsDisplay,
    frames: Vec<LyricsFrame>,
    currentFrame: i8,
    audio: Option<rodio::Sink>,
    start: Option<chrono::DateTime<chrono::Local>>,
    scene: sprite::Scene<opengl_graphics::Texture>,
}
impl App<GlGraphics> {
    fn calc_fps(&mut self, dt: f64) {
        self.fps = 1.0 / dt; //((self.fps * 4.0) + (1.0 / dt)) / 5.0;
    }
    fn play(&mut self) {
        self.start = chrono::Local::now()
            .checked_sub_signed(chrono::Duration::milliseconds(1500))
            .unwrap()
            .into();

        let device = rodio::default_output_device().unwrap();
        let sink = rodio::Sink::new(&device);
        let file = std::fs::File::open("thriller.mp3").unwrap();
        sink.append(
            rodio::Decoder::new(std::io::BufReader::with_capacity(10 * 1024 * 1024, file)).unwrap(),
        );
        self.audio = Some(sink);
    }
    fn stop(&mut self) {
        self.start = None;
        if let Some(a) = &self.audio {
            a.stop();
        }
        self.audio = None;
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
        // println!("rendering text (w1={:?}, w2={:?})", w1, w2);
        let l: u32 = (w1 + w2) as u32;
        let x_offset = (args.width - l as f64) / 2.0;
        self.gl.draw(args.viewport(), |c, gl| {
            let transform = c.transform.trans(x_offset, 200.0);
            text(black, 24, fst, cache, transform.scale(1.0, 1.2), gl).unwrap();
            text(red, 24, fst, cache, transform, gl).unwrap();
            text(black, 24, snd, cache, transform.trans(w1, 0.0), gl).unwrap();
        });
    }
    fn render<C>(&mut self, e: &piston::input::Event, args: &RenderArgs, cache: &mut C)
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
        if self.frame_count % 60 * 60 == 0 {
            // println!("fps; {:?}", self.fps);
            println!("dt: {:?}", args.ext_dt);
        }
        let fps = self.fps;
        // self.scene.event(e);
        let scene = &self.scene;
        self.gl.draw(args.viewport(), |c, gl| {
            clear(primary_color, gl);
            let transform = c
                .transform
                .trans(x, y)
                .rot_rad(rotation)
                .trans(-0.5 * SQUARE_EDGE_SIZE, -0.5 * SQUARE_EDGE_SIZE);
            scene.draw(c.transform.trans(x, y), gl);
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
        // {
        //     self.gl.draw(args.viewport(), |c, gl| {
        //         self.scene.draw(c.transform, gl);
        //     });
        // }
        self.render_lyrics(*args, cache);
    }

    fn update(&mut self, args: &UpdateArgs) {
        use chrono::Duration;
        self.rotation += 2.0 * args.dt;
        let offset: Duration = match self.start {
            Some(s) => chrono::Local::now() - s,
            None => Duration::zero(),
        };
        let millis: f64 = offset.num_milliseconds() as f64;

        self.calc_fps(args.dt);
        let active: Vec<&LyricsFrame> = self
            .frames
            .iter()
            .filter(|f| f.offset < millis && !f.text.is_empty())
            .collect();
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
        self.lyrics
            .sing_progress
            .store(progress as usize, std::sync::atomic::Ordering::SeqCst);
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
                self.stop();
                self.play();
            }
            Button::Keyboard(Key::S) => {
                use std::ops::Add;
                self.stop();
            }
            Button::Keyboard(Key::A) => {
                if let Some(s) = &self.start {
                    self.start = s.checked_sub_signed(Duration::seconds(1)).into();
                }
            }
            Button::Keyboard(Key::D) => {
                if let Some(s) = &self.start {
                    self.start = s.checked_add_signed(Duration::seconds(1)).into();
                }
                // if let Some(a) = &self.audio {
                //     a.queue_tx.ptr.
                // }
            }
            _ => println!("pressed {:?}", e),
        }
    }
}

fn get_animation() -> Behavior<Animation> {
    let seq = Sequence(vec![
        Action(Ease(
            EaseFunction::CubicOut,
            Box::new(ScaleTo(2.0, 0.5, 0.5)),
        )),
        Action(Ease(
            EaseFunction::BounceOut,
            Box::new(MoveBy(1.0, 0.0, 100.0)),
        )),
        Action(Ease(
            EaseFunction::ElasticOut,
            Box::new(MoveBy(2.0, 0.0, -100.0)),
        )),
        Action(Ease(
            EaseFunction::BackInOut,
            Box::new(MoveBy(1.0, 0.0, -100.0)),
        )),
        Wait(0.5),
        Action(Ease(
            EaseFunction::ExponentialInOut,
            Box::new(MoveBy(1.0, 0.0, 100.0)),
        )),
        Action(Blink(1.0, 5)),
        While(
            Box::new(Wait(6.0)),
            vec![
                Action(Ease(EaseFunction::QuadraticIn, Box::new(FadeOut(1.0)))),
                Action(Ease(EaseFunction::QuadraticOut, Box::new(FadeIn(1.0)))),
            ],
        ),
    ]);
    While(
        Box::new(WaitForever),
        vec![
            seq,
            Action(Ease(
                EaseFunction::CubicOut,
                Box::new(ScaleTo(5.0, 1.0, 1.0)),
            )),
        ],
    )
}
fn main() {
    let opengl = OpenGL::V4_5;
    let lyrics: Vec<LyricsFrame> = std::fs::read_to_string("song.json")
        .map(|s| serde_json::de::from_str(s.as_str()).unwrap())
        .unwrap();
    println!("lyrics loaded");

    let mut window: Window = WindowSettings::new("thriller karaoke", [800, 600])
        .opengl(opengl)
        .exit_on_esc(true)
        .vsync(true)
        .build()
        .unwrap();
    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .unwrap();

    let mut scene = Scene::new();
    let tex =
        Rc::new(Texture::from_path("infuy.svg.png", &TextureSettings::new()).unwrap());
    let mut sprite = Sprite::from_texture(tex);

    let bg_tex = Rc::new(Texture::from_path("bg.jpeg", &TextureSettings::new()).unwrap());
    let mut bg_sprite = Sprite::from_texture(bg_tex);
    bg_sprite.set_anchor(0.5, 0.5);
    bg_sprite.set_position(0.0, 0.0);
    sprite.set_position(0.0, 0.0);
    sprite.set_anchor(0.5, 0.5);
    let id = bg_sprite.add_child(sprite);
    scene.add_child(bg_sprite);
    
    scene.run(id, &get_animation());
    // This animation and the one above can run in parallel.
    let rotate = Action(Ease(
        EaseFunction::ExponentialInOut,
        Box::new(RotateTo(2.0, 360.0)),
    ));
    scene.run(id, &rotate);

    // scene.ru
    let font = assets.join("Bangers-Regular.ttf");
    // let factory = window.window.factory.clone();
    let mut app = App {
        gl: GlGraphics::new(opengl),
        rotation: 0.0,
        frame_count: 0,
        fps: 0.0,
        color: RgbaColorType{r:0.2, g:0.45, b:0.3, a:0.3},
        lyrics: LyricsDisplay::new("hola como te va? uno dos tres cuatro cinco seis siete"),
        frames: lyrics,
        start: chrono::Local::now()
            .checked_sub_signed(chrono::Duration::seconds(2))
            .unwrap()
            .into(),
        currentFrame: -1,
        audio: None,
        scene: scene,
    };
    app.play();
    let mut glyph_cache = GlyphCache::new(font, (), TextureSettings::new()).unwrap();

    // let mut cache =;
    let settings = EventSettings::new().max_fps(20);
    let mut events = Events::new(settings).max_fps(30);
    while let Some(e) = events.next(&mut window) {
        app.scene.event(&e);
        if let Some(r) = e.render_args() {
            app.render(&e, &r, &mut glyph_cache);
        }

        if let Some(u) = e.update_args() {
            app.update(&u);
        }
        if let Some(e) = e.press_args() {
            app.key(&e);
        }
    }
}
