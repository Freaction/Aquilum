mod launch;

use std::path::Path;
#[cfg(not(any(target_os = "windows", target_os = "android")))]
use std::process::Command;

#[cfg(not(any(target_os = "windows", target_os = "android")))]
use launch::launch;
pub use launch::{app_path, appimage, restart};

pub const DESKTOP: bool = cfg!(not(any(target_os = "android", target_os = "ios")));

#[cfg(target_os = "windows")]
pub fn open(target: &str) {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, PCWSTR, w};
    let launched = unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(target), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    if launched.0 as usize <= 32 {
        report!("{target} не открыт: код {}", launched.0 as usize);
    }
}

#[cfg(target_os = "android")]
pub use crate::android::{leave, login_links, open, set_dark_theme, show_sync, vaults_home};

#[cfg(not(target_os = "android"))]
pub fn leave() {}

#[cfg(not(target_os = "android"))]
pub fn set_dark_theme(_: bool) {}

#[cfg(not(target_os = "android"))]
pub fn show_sync(_: &aquilum_core::sync::SyncService) {}

#[cfg(not(target_os = "android"))]
pub fn vaults_home() -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_os = "android"))]
pub fn login_links() -> Option<std::sync::mpsc::Receiver<String>> {
    None
}

#[cfg(not(any(target_os = "windows", target_os = "android")))]
pub fn open(target: &str) {
    let mut command = Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" });
    command.arg(target);
    launch(command, target);
}

pub fn external_url(raw: &str) -> Option<&str> {
    let url = raw.trim().trim_start_matches('<').trim_end_matches('>');
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")).then_some(url)
}

pub fn open_url(raw: &str) {
    if let Some(url) = external_url(raw) {
        open(url);
    }
}

#[cfg(target_os = "windows")]
pub fn reveal(path: &Path) {
    use windows::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems};
    let native = windows::core::HSTRING::from(path.to_string_lossy().replace('/', "\\"));
    let result = unsafe {
        let item = ILCreateFromPathW(&native);
        if item.is_null() {
            Err(windows::core::Error::from_thread())
        } else {
            let shown = SHOpenFolderAndSelectItems(item, None, 0);
            ILFree(Some(item));
            shown
        }
    };
    if let Err(error) = result {
        report!("{} не показан в папке: {error}", path.display());
    }
}

#[cfg(target_os = "android")]
pub fn reveal(_: &Path) {}

#[cfg(not(any(target_os = "windows", target_os = "android")))]
pub fn reveal(path: &Path) {
    let command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg("-R").arg(path);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(path.parent().unwrap_or(path));
        command
    };
    launch(command, &path.display().to_string());
}

pub fn release_memory() {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Memory::{HeapOptimizeResources, HeapSetInformation};
        #[repr(C)]
        struct Optimize {
            version: u32,
            flags: u32,
        }
        let info = Optimize { version: 1, flags: 0 };
        let _ = unsafe { HeapSetInformation(None, HeapOptimizeResources, Some((&raw const info).cast()), size_of::<Optimize>()) };
    }
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        unsafe { malloc_trim(0) };
    }
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
        }
        unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_web_and_mail_links_open_like_tauri() {
        assert_eq!(super::external_url(" <https://example.com/a> "), Some("https://example.com/a"));
        assert_eq!(super::external_url("MAILTO:a@b.c"), Some("MAILTO:a@b.c"));
        for blocked in [r"C:\Windows\System32\cmd.exe", "file:///C:/a.bat", r"\\server\share\x.exe", "ms-settings:", "notes/a.md", "#heading", "www.example.com"] {
            assert_eq!(super::external_url(blocked), None, "{blocked}");
        }
    }
}
