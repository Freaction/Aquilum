mod frames;
mod sound;
mod worker;

use std::path::Path;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};

use masonry::peniko::ImageBrush;

enum Command {
    Toggle,
    Seek(f64),
    Mute(bool),
}

#[derive(Default)]
struct Shared {
    frame: Option<ImageBrush>,
    fresh: bool,
    position: f64,
    duration: f64,
    ended: bool,
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct Engine {
    shared: Arc<Mutex<Shared>>,
    commands: Sender<Command>,
    playing: bool,
    muted: bool,
}

impl Engine {
    pub fn open(path: &Path) -> Option<Engine> {
        let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let shared = Arc::new(Mutex::new(Shared::default()));
        let (commands, receiver) = channel();
        let (ready, opened) = channel();
        let worker = Arc::clone(&shared);
        std::thread::Builder::new().name("видео".into()).spawn(move || worker::run(&path, &receiver, &worker, &ready)).ok()?;
        opened.recv().ok()?.then_some(Engine { shared, commands, playing: false, muted: false })
    }

    pub fn toggle(&mut self) {
        self.playing = !self.playing;
        let _ = self.commands.send(Command::Toggle);
    }

    pub fn seek(&mut self, fraction: f64) {
        let _ = self.commands.send(Command::Seek(fraction.clamp(0.0, 1.0)));
    }

    pub fn mute(&mut self) {
        self.muted = !self.muted;
        let _ = self.commands.send(Command::Mute(self.muted));
    }

    pub fn status(&mut self) -> (bool, f64, f64, bool) {
        let (position, duration, ended) = {
            let mut shared = lock(&self.shared);
            (shared.position, shared.duration, std::mem::take(&mut shared.ended))
        };
        if ended {
            self.playing = false;
        }
        (self.playing, position, duration, self.muted)
    }

    pub fn frame(&mut self) -> Option<ImageBrush> {
        let mut shared = lock(&self.shared);
        std::mem::take(&mut shared.fresh).then(|| shared.frame.clone()).flatten()
    }
}
