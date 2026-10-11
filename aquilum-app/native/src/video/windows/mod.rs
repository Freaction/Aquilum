mod frames;
mod sound;

use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use masonry::peniko::ImageBrush;
use windows::Win32::Media::MediaFoundation::{IMFSourceReader, MF_PD_DURATION, MF_SOURCE_READER_MEDIASOURCE, MF_VERSION, MFSTARTUP_FULL, MFStartup};
use windows::Win32::System::Com::StructuredStorage::{PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::Variant::VT_I8;
use windows::core::{GUID, HSTRING};

use frames::Frames;
use sound::Sound;

const IDLE: Duration = Duration::from_millis(100);
const STEP: Duration = Duration::from_millis(4);

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

pub struct Engine {
    shared: Arc<Mutex<Shared>>,
    commands: Sender<Command>,
    playing: bool,
    muted: bool,
}

fn seek(reader: &IMFSourceReader, seconds: f64) {
    let position = PROPVARIANT {
        Anonymous: PROPVARIANT_0 {
            Anonymous: ManuallyDrop::new(PROPVARIANT_0_0 {
                vt: VT_I8,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: PROPVARIANT_0_0_0 { hVal: (seconds.max(0.0) * 1e7) as i64 },
            }),
        },
    };
    unsafe {
        let _ = reader.SetCurrentPosition(&GUID::zeroed(), &position);
    }
}

fn duration(reader: &IMFSourceReader) -> f64 {
    unsafe {
        reader
            .GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION)
            .map_or(0.0, |value| value.Anonymous.Anonymous.Anonymous.uhVal as f64 / 1e7)
    }
}

impl Engine {
    pub fn open(path: &Path) -> Option<Engine> {
        let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let shared = Arc::new(Mutex::new(Shared::default()));
        let (commands, receiver) = channel();
        let (ready, opened) = channel();
        let worker = Arc::clone(&shared);
        std::thread::Builder::new().name("видео".into()).spawn(move || run(&path, &receiver, &worker, &ready)).ok()?;
        opened.recv().ok()?.then_some(Engine { shared, commands, playing: false, muted: false })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
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
            let mut shared = self.lock();
            (shared.position, shared.duration, std::mem::take(&mut shared.ended))
        };
        if ended {
            self.playing = false;
        }
        (self.playing, position, duration, self.muted)
    }

    pub fn frame(&mut self) -> Option<ImageBrush> {
        let mut shared = self.lock();
        std::mem::take(&mut shared.fresh).then(|| shared.frame.clone()).flatten()
    }
}

struct Player {
    frames: Frames,
    sound: Option<Sound>,
    duration: f64,
    playing: bool,
    base: f64,
    started: Instant,
}

impl Player {
    fn clock(&self) -> f64 {
        let elapsed = match &self.sound {
            Some(sound) => sound.played(),
            None => self.started.elapsed().as_secs_f64(),
        };
        (self.base + if self.playing { elapsed } else { 0.0 }).min(self.duration.max(0.0))
    }

    fn show(&mut self, shared: &Mutex<Shared>, clock: f64) {
        let frame = self.frames.until(clock);
        let mut shared = shared.lock().unwrap_or_else(|e| e.into_inner());
        if frame.is_some() {
            shared.frame = frame;
            shared.fresh = true;
        }
        shared.position = clock;
    }

    fn seek(&mut self, seconds: f64) {
        self.frames.seek(seconds);
        if let Some(sound) = &mut self.sound {
            sound.seek(seconds);
        }
        self.base = seconds;
        self.started = Instant::now();
    }

    fn command(&mut self, shared: &Mutex<Shared>, command: Command) {
        match command {
            Command::Toggle if self.playing => {
                self.base = self.clock();
                self.playing = false;
                if let Some(sound) = &mut self.sound {
                    sound.pause();
                    sound.seek(self.base);
                }
            }
            Command::Toggle => {
                if self.duration > 0.0 && self.base >= self.duration - 0.05 {
                    self.seek(0.0);
                }
                self.started = Instant::now();
                self.playing = true;
            }
            Command::Seek(fraction) => {
                let seconds = fraction * self.duration;
                self.seek(seconds);
                self.show(shared, seconds);
            }
            Command::Mute(muted) => {
                if let Some(sound) = &self.sound {
                    sound.mute(muted);
                }
            }
        }
    }

    fn finished(&self) -> bool {
        self.frames.ended() && self.sound.as_ref().is_none_or(Sound::finished)
    }
}

fn run(path: &PathBuf, commands: &Receiver<Command>, shared: &Mutex<Shared>, ready: &Sender<bool>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let _ = MFStartup(MF_VERSION, MFSTARTUP_FULL);
    }
    let source = HSTRING::from(path.as_os_str());
    let Some(mut frames) = Frames::open(&source) else {
        let _ = ready.send(false);
        return;
    };
    let duration = duration(&frames.reader);
    let first = frames.until(0.0);
    {
        let mut shared = shared.lock().unwrap_or_else(|e| e.into_inner());
        shared.duration = duration;
        shared.fresh = first.is_some();
        shared.frame = first;
    }
    let mut player = Player { frames, sound: Sound::open(&source), duration, playing: false, base: 0.0, started: Instant::now() };
    let _ = ready.send(true);
    loop {
        let next = if player.playing { commands.try_recv().map_err(|e| e == TryRecvError::Disconnected) } else { commands.recv_timeout(IDLE).map_err(|e| e == RecvTimeoutError::Disconnected) };
        match next {
            Ok(command) => player.command(shared, command),
            Err(true) => return,
            Err(false) => {}
        }
        if !player.playing {
            continue;
        }
        if let Some(sound) = &mut player.sound {
            sound.fill();
        }
        let clock = player.clock();
        player.show(shared, clock);
        if player.finished() || (player.duration > 0.0 && clock >= player.duration) {
            player.playing = false;
            player.base = player.duration;
            if let Some(sound) = &mut player.sound {
                sound.pause();
            }
            shared.lock().unwrap_or_else(|e| e.into_inner()).ended = true;
        }
        std::thread::sleep(STEP);
    }
}
