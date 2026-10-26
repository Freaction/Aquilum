use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::Thread;
use std::time::{Duration, Instant};

const DEFAULT_PERIOD: Duration = Duration::from_micros(8333);

#[cfg(target_os = "windows")]
fn primary_output() -> Option<windows::Win32::Graphics::Dxgi::IDXGIOutput> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;
    let adapter = unsafe { factory.EnumAdapters1(0) }.ok()?;
    unsafe { adapter.EnumOutputs(0) }.ok()
}

#[cfg(target_os = "windows")]
fn refresh_period() -> Option<Duration> {
    use windows::Win32::Graphics::Dwm::{DWM_TIMING_INFO, DwmGetCompositionTimingInfo};
    let mut info = DWM_TIMING_INFO { cbSize: size_of::<DWM_TIMING_INFO>() as u32, ..Default::default() };
    unsafe { DwmGetCompositionTimingInfo(windows::Win32::Foundation::HWND::default(), &mut info) }.ok()?;
    let rate = info.rateRefresh;
    (rate.uiNumerator > 0 && rate.uiDenominator > 0).then(|| Duration::from_secs_f64(f64::from(rate.uiDenominator) / f64::from(rate.uiNumerator)))
}

#[cfg(target_os = "android")]
struct AndroidVsync {
    choreographer: *mut ndk_sys::AChoreographer,
    fired: std::sync::atomic::AtomicBool,
    last_nanos: std::sync::atomic::AtomicI64,
    period_nanos: std::sync::atomic::AtomicI64,
}

#[cfg(target_os = "android")]
unsafe impl Send for AndroidVsync {}
#[cfg(target_os = "android")]
unsafe impl Sync for AndroidVsync {}

#[cfg(target_os = "android")]
impl AndroidVsync {
    fn new() -> Option<Self> {
        unsafe {
            ndk_sys::ALooper_prepare(ndk_sys::ALOOPER_PREPARE_ALLOW_NON_CALLBACKS as i32);
            let choreographer = ndk_sys::AChoreographer_getInstance();
            if choreographer.is_null() {
                return None;
            }
            Some(AndroidVsync {
                choreographer,
                fired: std::sync::atomic::AtomicBool::new(false),
                last_nanos: std::sync::atomic::AtomicI64::new(0),
                period_nanos: std::sync::atomic::AtomicI64::new(0),
            })
        }
    }

    fn wait(&self) {
        unsafe extern "C" fn on_frame_64(frame_time_nanos: i64, data: *mut std::ffi::c_void) {
            let vsync = unsafe { &*(data as *const AndroidVsync) };
            let prev = vsync.last_nanos.swap(frame_time_nanos, Ordering::Relaxed);
            if prev > 0 && frame_time_nanos > prev {
                let delta = frame_time_nanos - prev;
                if (4_000_000..=34_000_000).contains(&delta) {
                    vsync.period_nanos.store(delta, Ordering::Relaxed);
                }
            }
            vsync.fired.store(true, Ordering::Release);
        }

        self.fired.store(false, Ordering::Release);
        unsafe {
            ndk_sys::AChoreographer_postFrameCallback64(
                self.choreographer,
                Some(on_frame_64),
                self as *const Self as *mut std::ffi::c_void,
            );
            while !self.fired.load(Ordering::Acquire) {
                let res = ndk_sys::ALooper_pollOnce(50, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
                if res == ndk_sys::ALOOPER_POLL_TIMEOUT || res == ndk_sys::ALOOPER_POLL_ERROR {
                    break;
                }
            }
        }
    }

    fn period(&self) -> Option<Duration> {
        let nanos = self.period_nanos.load(Ordering::Relaxed);
        (nanos > 0).then(|| Duration::from_nanos(nanos as u64))
    }
}

#[cfg(target_os = "android")]
fn refresh_period() -> Option<Duration> {
    thread_local! {
        static ANDROID_VSYNC: Option<AndroidVsync> = AndroidVsync::new();
    }
    ANDROID_VSYNC.with(|v| v.as_ref()?.period())
}

#[cfg(target_os = "macos")]
struct MacosVsyncInner {
    link: *mut objc2_core_video::CVDisplayLink,
    cv: std::sync::Condvar,
    lock: std::sync::Mutex<bool>,
    waiting: AtomicBool,
}

#[cfg(target_os = "macos")]
unsafe impl Send for MacosVsyncInner {}
#[cfg(target_os = "macos")]
unsafe impl Sync for MacosVsyncInner {}

#[cfg(target_os = "macos")]
struct MacosVsync {
    inner: Arc<MacosVsyncInner>,
}

#[cfg(target_os = "macos")]
impl Drop for MacosVsyncInner {
    fn drop(&mut self) {
        unsafe {
            let _ = (*self.link).stop();
        }
    }
}

#[cfg(target_os = "macos")]
impl MacosVsync {
    fn new() -> Option<Self> {
        let mut link: *mut objc2_core_video::CVDisplayLink = std::ptr::null_mut();
        let ret = unsafe {
            objc2_core_video::CVDisplayLink::create_with_active_cg_displays(
                core::ptr::NonNull::new(&mut link)?
            )
        };
        if ret != 0 || link.is_null() {
            return None;
        }
        let inner = Arc::new(MacosVsyncInner {
            link,
            cv: std::sync::Condvar::new(),
            lock: std::sync::Mutex::new(false),
            waiting: AtomicBool::new(false),
        });
        let raw = Arc::as_ptr(&inner) as *mut std::ffi::c_void;

        unsafe extern "C-unwind" fn on_frame(
            _display_link: core::ptr::NonNull<objc2_core_video::CVDisplayLink>,
            _in_now: core::ptr::NonNull<objc2_core_video::CVTimeStamp>,
            _in_output_time: core::ptr::NonNull<objc2_core_video::CVTimeStamp>,
            _flags_in: objc2_core_video::CVOptionFlags,
            _flags_out: core::ptr::NonNull<objc2_core_video::CVOptionFlags>,
            user_info: *mut std::ffi::c_void,
        ) -> objc2_core_video::CVReturn {
            let inner = unsafe { &*(user_info as *const MacosVsyncInner) };
            if inner.waiting.load(Ordering::Acquire) {
                let mut guard = inner.lock.lock().unwrap();
                *guard = true;
                inner.cv.notify_one();
            }
            0
        }

        let ret = unsafe {
            (*link).set_output_callback(Some(on_frame), raw)
        };
        if ret != 0 || unsafe { (*link).start() } != 0 {
            return None;
        }
        Some(MacosVsync { inner })
    }

    fn wait(&self) {
        self.inner.waiting.store(true, Ordering::Release);
        let mut guard = self.inner.lock.lock().unwrap();
        *guard = false;
        let _ = self.inner.cv.wait_timeout_while(guard, Duration::from_millis(50), |f| !*f);
        self.inner.waiting.store(false, Ordering::Release);
    }

    fn period(&self) -> Option<Duration> {
        let period = unsafe { (*self.inner.link).actual_output_video_refresh_period() };
        (period > 0.0).then(|| Duration::from_secs_f64(period))
    }
}

#[cfg(target_os = "macos")]
fn refresh_period() -> Option<Duration> {
    thread_local! {
        static MACOS_VSYNC: Option<MacosVsync> = MacosVsync::new();
    }
    MACOS_VSYNC.with(|v| v.as_ref()?.period())
}

#[cfg(not(any(target_os = "windows", target_os = "android", target_os = "macos")))]
fn refresh_period() -> Option<Duration> {
    None
}

struct Pacer {
    period: Duration,
    checked: Option<Instant>,
    grid: Option<Instant>,
}

impl Pacer {
    fn tick(&mut self) -> Instant {
        wait_vblank();
        let now = Instant::now();
        if self.checked.is_none_or(|t| now - t > Duration::from_secs(1)) {
            self.period = refresh_period().unwrap_or(self.period);
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

fn wait_vblank() {
    #[cfg(target_os = "windows")]
    {
        thread_local! {
            static OUTPUT: Option<windows::Win32::Graphics::Dxgi::IDXGIOutput> = primary_output();
        }
        if OUTPUT.with(|output| output.as_ref().is_some_and(|o| unsafe { o.WaitForVBlank() }.is_ok())) {
            return;
        }
        if unsafe { windows::Win32::Graphics::Dwm::DwmFlush() }.is_ok() {
            return;
        }
    }
    #[cfg(target_os = "android")]
    {
        thread_local! {
            static ANDROID_VSYNC: Option<AndroidVsync> = AndroidVsync::new();
        }
        if ANDROID_VSYNC.with(|v| {
            if let Some(vsync) = v.as_ref() {
                vsync.wait();
                true
            } else {
                false
            }
        }) {
            return;
        }
    }
    #[cfg(target_os = "macos")]
    {
        thread_local! {
            static MACOS_VSYNC: Option<MacosVsync> = MacosVsync::new();
        }
        if MACOS_VSYNC.with(|v| {
            if let Some(vsync) = v.as_ref() {
                vsync.wait();
                true
            } else {
                false
            }
        }) {
            return;
        }
    }
    std::thread::sleep(DEFAULT_PERIOD);
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
