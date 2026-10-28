#[cfg(target_os = "android")]
#[path = "choreographer.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "display_link.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "dxgi.rs"]
mod platform;
#[cfg(not(any(target_os = "windows", target_os = "android", target_os = "macos")))]
mod platform {
    pub(super) fn wait() -> bool {
        false
    }

    pub(super) fn period() -> Option<std::time::Duration> {
        None
    }
}

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::Thread;
use std::time::{Duration, Instant};

const DEFAULT_PERIOD: Duration = Duration::from_micros(8333);

struct Pacer {
    period: Duration,
    checked: Option<Instant>,
    grid: Option<Instant>,
}

impl Pacer {
    fn tick(&mut self) -> Instant {
        if !platform::wait() {
            std::thread::sleep(DEFAULT_PERIOD);
        }
        let now = Instant::now();
        if self.checked.is_none_or(|t| now - t > Duration::from_secs(1)) {
            self.period = platform::period().unwrap_or(self.period);
            self.checked = Some(now);
        }
        let period = self.period.as_secs_f64();
        let Some(grid) = self.grid else {
            self.grid = Some(now);
            return now;
        };
        let steps = (now.saturating_duration_since(grid).as_secs_f64() / period).round();
        if steps < 1.0 {
            std::thread::sleep((grid + self.period).saturating_duration_since(now));
            let next = grid + self.period;
            self.grid = Some(next);
            return next;
        }
        let expected = grid + Duration::from_secs_f64(steps * period);
        let error = now.max(expected) - now.min(expected);
        let stamp = if error > self.period / 4 { now } else { expected };
        self.grid = Some(stamp);
        stamp
    }
}

pub struct Vsync {
    wanted: Arc<AtomicBool>,
    fired: Arc<AtomicU64>,
    base: Instant,
    thread: Thread,
}

impl Vsync {
    pub fn new(wake: impl Fn() + Send + 'static) -> Self {
        let wanted = Arc::new(AtomicBool::new(false));
        let fired = Arc::new(AtomicU64::new(0));
        let base = Instant::now();
        let (w, f) = (wanted.clone(), fired.clone());
        let thread = std::thread::Builder::new()
            .name("vsync".into())
            .spawn(move || {
                let mut pacer = Pacer { period: DEFAULT_PERIOD, checked: None, grid: None };
                loop {
                    while !w.load(Ordering::Acquire) {
                        std::thread::park();
                    }
                    let stamp = pacer.tick();
                    aq_trace::mark("такт экрана");
                    w.store(false, Ordering::Release);
                    f.store(stamp.saturating_duration_since(base).as_nanos() as u64 + 1, Ordering::Release);
                    wake();
                }
            })
            .expect("поток vsync")
            .thread()
            .clone();
        Vsync { wanted, fired, base, thread }
    }

    pub fn request(&self) {
        if !self.wanted.swap(true, Ordering::AcqRel) {
            self.thread.unpark();
        }
    }

    pub fn take(&self) -> Option<Instant> {
        match self.fired.swap(0, Ordering::AcqRel) {
            0 => None,
            nanos => Some(self.base + Duration::from_nanos(nanos - 1)),
        }
    }
}
