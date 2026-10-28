use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

struct Choreographer {
    handle: *mut ndk_sys::AChoreographer,
    fired: AtomicBool,
    last_nanos: AtomicI64,
    period_nanos: AtomicI64,
}

impl Choreographer {
    fn new() -> Option<Self> {
        unsafe {
            ndk_sys::ALooper_prepare(ndk_sys::ALOOPER_PREPARE_ALLOW_NON_CALLBACKS as i32);
            let handle = ndk_sys::AChoreographer_getInstance();
            (!handle.is_null()).then(|| Choreographer { handle, fired: AtomicBool::new(false), last_nanos: AtomicI64::new(0), period_nanos: AtomicI64::new(i64::MAX) })
        }
    }

    fn wait(&self) {
        unsafe extern "C" fn on_frame(frame_time_nanos: i64, data: *mut std::ffi::c_void) {
            let vsync = unsafe { &*(data as *const Choreographer) };
            let prev = vsync.last_nanos.swap(frame_time_nanos, Ordering::Relaxed);
            if prev > 0 && (4_000_000..=34_000_000).contains(&(frame_time_nanos - prev)) {
                vsync.period_nanos.fetch_min(frame_time_nanos - prev, Ordering::Relaxed);
            }
            vsync.fired.store(true, Ordering::Release);
        }

        self.fired.store(false, Ordering::Release);
        unsafe {
            ndk_sys::AChoreographer_postFrameCallback64(self.handle, Some(on_frame), self as *const Self as *mut std::ffi::c_void);
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
        (nanos < i64::MAX).then(|| Duration::from_nanos(nanos as u64))
    }
}

thread_local! {
    static CHOREOGRAPHER: Option<Choreographer> = Choreographer::new();
}

pub(super) fn wait() -> bool {
    CHOREOGRAPHER.with(|c| c.as_ref().map(Choreographer::wait).is_some())
}

pub(super) fn period() -> Option<Duration> {
    CHOREOGRAPHER.with(|c| c.as_ref()?.period())
}
