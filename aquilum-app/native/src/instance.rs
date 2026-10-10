use std::fs::{File, TryLockError};
use std::path::Path;

pub const WINDOW_CLASS: &str = "AquilumWindow";

pub enum Instance {
    First(Option<File>),
    Running,
}

pub fn acquire(data_dir: &Path) -> Instance {
    let file = std::fs::create_dir_all(data_dir).and_then(|()| File::options().create(true).write(true).truncate(false).open(data_dir.join("instance.lock")));
    let Ok(file) = file else { return Instance::First(None) };
    match file.try_lock() {
        Ok(()) => Instance::First(Some(file)),
        Err(TryLockError::WouldBlock) => Instance::Running,
        Err(TryLockError::Error(_)) => Instance::First(None),
    }
}

#[cfg(target_os = "windows")]
pub fn activate() {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow};
    use windows::core::HSTRING;
    unsafe {
        let Ok(window) = FindWindowW(&HSTRING::from(WINDOW_CLASS), None) else { return };
        if IsIconic(window).as_bool() {
            let _ = ShowWindow(window, SW_RESTORE);
        }
        let _ = SetForegroundWindow(window);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn activate() {}
