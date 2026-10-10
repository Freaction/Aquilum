use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::Thread;

#[cfg(target_os = "windows")]
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmFlush() -> i32;
}

fn wait_vblank() {
    #[cfg(target_os = "windows")]
    if unsafe { DwmFlush() } == 0 {
        return;
    }
    std::thread::sleep(std::time::Duration::from_millis(8));
}

pub struct Vsync {
    wanted: Arc<AtomicBool>,
    fired: Arc<AtomicBool>,
    thread: Thread,
}

impl Vsync {
    pub fn new(wake: impl Fn() + Send + 'static) -> Self {
        let wanted = Arc::new(AtomicBool::new(false));
        let fired = Arc::new(AtomicBool::new(false));
        let (w, f) = (wanted.clone(), fired.clone());
        let thread = std::thread::Builder::new()
            .name("vsync".into())
            .spawn(move || loop {
                while !w.load(Ordering::Acquire) {
                    std::thread::park();
                }
                wait_vblank();
                aq_trace::mark("такт экрана");
                w.store(false, Ordering::Release);
                f.store(true, Ordering::Release);
                wake();
            })
            .expect("поток vsync")
            .thread()
            .clone();
        Vsync { wanted, fired, thread }
    }

    pub fn request(&self) {
        if !self.wanted.swap(true, Ordering::AcqRel) {
            self.thread.unpark();
        }
    }

    pub fn take(&self) -> bool {
        self.fired.swap(false, Ordering::AcqRel)
    }
}
