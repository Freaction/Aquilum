use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

struct Inner {
    link: *mut objc2_core_video::CVDisplayLink,
    cv: std::sync::Condvar,
    lock: std::sync::Mutex<bool>,
    waiting: AtomicBool,
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe {
            let _ = (*self.link).stop();
        }
    }
}

struct DisplayLink {
    inner: Arc<Inner>,
}

impl DisplayLink {
    fn new() -> Option<Self> {
        let mut link: *mut objc2_core_video::CVDisplayLink = std::ptr::null_mut();
        let ret = unsafe { objc2_core_video::CVDisplayLink::create_with_active_cg_displays(core::ptr::NonNull::new(&mut link)?) };
        if ret != 0 || link.is_null() {
            return None;
        }
        let inner = Arc::new(Inner { link, cv: std::sync::Condvar::new(), lock: std::sync::Mutex::new(false), waiting: AtomicBool::new(false) });
        let raw = Arc::as_ptr(&inner) as *mut std::ffi::c_void;

        unsafe extern "C-unwind" fn on_frame(
            _display_link: core::ptr::NonNull<objc2_core_video::CVDisplayLink>,
            _in_now: core::ptr::NonNull<objc2_core_video::CVTimeStamp>,
            _in_output_time: core::ptr::NonNull<objc2_core_video::CVTimeStamp>,
            _flags_in: objc2_core_video::CVOptionFlags,
            _flags_out: core::ptr::NonNull<objc2_core_video::CVOptionFlags>,
            user_info: *mut std::ffi::c_void,
        ) -> objc2_core_video::CVReturn {
            let inner = unsafe { &*(user_info as *const Inner) };
            if inner.waiting.load(Ordering::Acquire) {
                let mut guard = inner.lock.lock().unwrap();
                *guard = true;
                inner.cv.notify_one();
            }
            0
        }

        let ret = unsafe { (*link).set_output_callback(Some(on_frame), raw) };
        if ret != 0 || unsafe { (*link).start() } != 0 {
            return None;
        }
        Some(DisplayLink { inner })
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

thread_local! {
    static DISPLAY_LINK: Option<DisplayLink> = DisplayLink::new();
}

pub(super) fn wait() -> bool {
    DISPLAY_LINK.with(|d| d.as_ref().map(DisplayLink::wait).is_some())
}

pub(super) fn period() -> Option<Duration> {
    DISPLAY_LINK.with(|d| d.as_ref()?.period())
}
