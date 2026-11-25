use std::num::NonZero;
use std::sync::OnceLock;
use std::thread::JoinHandle;

#[derive(Clone, Copy)]
pub enum Work {
    Frame,
    Parallel,
    Background,
}

pub fn count(work: Work) -> usize {
    static CORES: OnceLock<usize> = OnceLock::new();
    let cores = *CORES.get_or_init(|| std::thread::available_parallelism().map_or(1, NonZero::get));
    match work {
        Work::Frame if cores <= 2 => 0,
        Work::Frame => (cores - 1).min(4),
        Work::Parallel => cores.min(4),
        Work::Background => (cores - 1).clamp(1, 2),
    }
}

pub fn spawn<T: Send + 'static>(name: &str, job: impl FnOnce() -> T + Send + 'static) -> std::io::Result<JoinHandle<T>> {
    std::thread::Builder::new().name(name.into()).spawn(job)
}

pub fn background<T: Send + 'static>(name: &str, job: impl FnOnce() -> T + Send + 'static) -> std::io::Result<JoinHandle<T>> {
    spawn(name, move || {
        lower_priority();
        job()
    })
}

#[cfg(windows)]
pub fn lower_priority() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> isize;
        fn SetThreadPriority(thread: isize, priority: i32) -> i32;
    }
    unsafe { SetThreadPriority(GetCurrentThread(), -1) };
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn lower_priority() {
    unsafe extern "C" {
        fn setpriority(which: i32, who: u32, priority: i32) -> i32;
    }
    unsafe { setpriority(0, 0, 10) };
}

#[cfg(target_os = "macos")]
pub fn lower_priority() {
    unsafe extern "C" {
        fn pthread_set_qos_class_self_np(class: u32, relative: i32) -> i32;
    }
    unsafe { pthread_set_qos_class_self_np(0x11, 0) };
}

#[cfg(not(any(windows, target_os = "linux", target_os = "android", target_os = "macos")))]
pub fn lower_priority() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_never_exceed_the_cores_and_frames_stay_on_the_caller_when_cores_are_few() {
        let cores = std::thread::available_parallelism().map_or(1, NonZero::get);
        assert!(count(Work::Parallel) <= cores && count(Work::Parallel) >= 1);
        assert!(count(Work::Background) < cores.max(2) && count(Work::Background) >= 1);
        assert!(count(Work::Frame) < cores);
        assert_eq!(count(Work::Frame) == 0, cores <= 2);
    }

    #[test]
    fn background_threads_run_their_job_and_carry_the_name() {
        let handle = background("фон", || std::thread::current().name().map(str::to_owned)).unwrap();
        assert_eq!(handle.join().unwrap().as_deref(), Some("фон"));
    }
}
