pub const UPDATE_DIR: &str = "aquilum-update";

#[cfg(target_os = "windows")]
pub fn apply(payload: &[u8], before_exit: impl FnOnce()) -> Result<(), String> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use windows::core::{HSTRING, w};
    let dir = std::env::temp_dir().join(UPDATE_DIR);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let installer = dir.join("Aquilum-setup.exe");
    std::fs::write(&installer, payload).map_err(|e| e.to_string())?;
    let bat = dir.join("update.bat");
    let script = "@echo off\r\ncd /d \"%~dp0\"\r\n(timeout /t 1 /nobreak >nul 2>&1 || ping 127.0.0.1 -n 2 >nul)\r\n\"Aquilum-setup.exe\" /S /R\r\n";
    std::fs::write(&bat, script).map_err(|e| e.to_string())?;
    let verb = if needs_elevation() { w!("runas") } else { w!("open") };
    before_exit();
    let started = unsafe {
        ShellExecuteW(
            None,
            verb,
            w!("cmd.exe"),
            &HSTRING::from(format!("/c \"{}\"", bat.display())),
            &HSTRING::from(dir.as_os_str()),
            SW_HIDE,
        )
    };
    if started.0 as isize <= 32 {
        return Err(format!("установщик не запустился, код {}", started.0 as isize));
    }
    std::process::exit(0);
}

#[cfg(target_os = "windows")]
fn needs_elevation() -> bool {
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let probe = parent.join(format!(".write_probe_{}", std::process::id()));
        if std::fs::write(&probe, b"").is_err() {
            return true;
        }
        let _ = std::fs::remove_file(&probe);
        return false;
    }
    false
}

#[cfg(target_os = "macos")]
pub fn apply(payload: &[u8], before_exit: impl FnOnce()) -> Result<(), String> {
    let bundle = crate::system::app_path().filter(|path| path.extension().is_some_and(|extension| extension == "app")).ok_or("приложение запущено не из пакета .app")?;
    let parent = bundle.parent().ok_or("нет каталога пакета")?;
    let staging = parent.join(format!(".{UPDATE_DIR}"));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let gz = flate2::read::GzDecoder::new(payload);
    super::tar::unpack(gz, &staging)?;
    let fresh = std::fs::read_dir(&staging).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "app")).ok_or("в обновлении нет пакета .app")?;
    before_exit();
    let backup = parent.join(format!(".aquilum-old-{}", std::process::id()));
    std::fs::rename(&bundle, &backup).map_err(|e| e.to_string())?;
    if let Err(error) = std::fs::rename(&fresh, &bundle) {
        let _ = std::fs::rename(&backup, &bundle);
        return Err(error.to_string());
    }
    let _ = std::fs::remove_dir_all(&backup);
    let _ = std::fs::remove_dir_all(&staging);
    crate::system::restart();
    std::process::exit(0);
}

pub fn supported() -> bool {
    cfg!(any(target_os = "windows", target_os = "macos", target_os = "android")) || crate::system::appimage().is_some()
}

#[cfg(target_os = "android")]
pub fn apply(payload: &[u8], _before_exit: impl FnOnce()) -> Result<(), String> {
    crate::android::install_update(payload)
}

#[cfg(target_os = "linux")]
pub fn apply(payload: &[u8], before_exit: impl FnOnce()) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let image = crate::system::appimage().ok_or("обновление поддерживается только для AppImage")?;
    let fresh = image.with_extension("update");
    std::fs::write(&fresh, payload).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&fresh, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    before_exit();
    std::fs::rename(&fresh, &image).map_err(|e| e.to_string())?;
    crate::system::restart();
    std::process::exit(0);
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux", target_os = "android")))]
pub fn apply(_payload: &[u8], _before_exit: impl FnOnce()) -> Result<(), String> {
    Err("платформа не поддерживается".into())
}
