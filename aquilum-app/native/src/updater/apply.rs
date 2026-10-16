#[cfg(target_os = "windows")]
pub fn apply(payload: &[u8], before_exit: impl FnOnce()) -> Result<(), String> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
    use windows::core::{HSTRING, w};
    let dir = std::env::temp_dir().join(format!("aquilum-update-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let installer = dir.join("Aquilum-setup.exe");
    std::fs::write(&installer, payload).map_err(|e| e.to_string())?;
    before_exit();
    let started = unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(installer.as_os_str()), &HSTRING::from("/S /R /UPDATE /ARGS"), None, SW_SHOW) };
    if started.0 as isize <= 32 {
        return Err(format!("установщик не запустился, код {}", started.0 as isize));
    }
    std::process::exit(0);
}

#[cfg(target_os = "macos")]
pub fn apply(payload: &[u8], before_exit: impl FnOnce()) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let text = exe.to_string_lossy().into_owned();
    let (bundle, _) = text.split_once(".app/Contents/MacOS/").ok_or("приложение запущено не из пакета .app")?;
    let bundle = std::path::PathBuf::from(format!("{bundle}.app"));
    let parent = bundle.parent().ok_or("нет каталога пакета")?;
    let staging = parent.join(format!(".aquilum-update-{}", std::process::id()));
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
    let _ = std::process::Command::new("open").arg("-n").arg(&bundle).spawn();
    std::process::exit(0);
}

fn appimage() -> Option<std::path::PathBuf> {
    std::env::var_os("APPIMAGE").map(std::path::PathBuf::from)
}

pub fn supported() -> bool {
    cfg!(any(target_os = "windows", target_os = "macos")) || cfg!(target_os = "linux") && appimage().is_some()
}

#[cfg(target_os = "linux")]
pub fn apply(payload: &[u8], before_exit: impl FnOnce()) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let image = appimage().ok_or("обновление поддерживается только для AppImage")?;
    let fresh = image.with_extension("update");
    std::fs::write(&fresh, payload).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&fresh, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    before_exit();
    std::fs::rename(&fresh, &image).map_err(|e| e.to_string())?;
    let _ = std::process::Command::new(&image).spawn();
    std::process::exit(0);
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn apply(_payload: &[u8], _before_exit: impl FnOnce()) -> Result<(), String> {
    Err("платформа не поддерживается".into())
}
