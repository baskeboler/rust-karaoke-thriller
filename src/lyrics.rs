use colors::*;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::sync::Arc;
use std::thread::*;

#[derive(Debug, Clone)]
pub struct LyricsDisplay {
    pub text: String,
    pub sing_progress: Arc<AtomicUsize>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LyricProgressEvent {
    pub char_count: u8,
    pub offset: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LyricsFrame {
    pub text: String,
    pub ticks: u32,
    pub offset: f64,
    pub event_offsets: Vec<LyricProgressEvent>,
}

pub trait TextContainer {
    fn set_text(&mut self, t: &str);
    fn get_text(&self) -> String;
}

impl TextContainer for LyricsDisplay {
    fn set_text(&mut self, t: &str) {
        self.text = String::from_str(t).unwrap();
    }

    fn get_text(&self) -> String {
        self.text.clone()
    }
}

impl LyricsDisplay {
    pub fn new(text: &str) -> LyricsDisplay {
        LyricsDisplay {
            text: String::from(text),
            sing_progress: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn advance(&mut self) {
        // let mut l = self.sing_progress.lock();
        let old_val = self.sing_progress.fetch_add(1, Ordering::SeqCst);
        if self.text.len() <= old_val {
            self.sing_progress.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub fn consumed_text(&self) -> (&str, &str) {
        self.text
            .split_at(self.sing_progress.load(Ordering::SeqCst))
    }

    pub fn reset(&mut self) {
        self.sing_progress.store(0, Ordering::SeqCst);
    }

    pub fn play(&mut self, timeouts: Vec<f32>) {
        for t in timeouts {
            let milis = (t * 1000.0).round() as u64;
            let dur = std::time::Duration::from_millis(milis);
            std::thread::park_timeout(dur);
            self.advance();
        }
    }
}
