const NAME: &str = "Aquilum";

pub fn available() -> bool {
    cfg!(feature = "installed")
}

fn executable() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    if cfg!(target_os = "linux")
        && let Some(image) = std::env::var_os("APPIMAGE")
    {
        return Some(image.to_string_lossy().into_owned());
    }
    if cfg!(target_os = "macos") {
        let canonical = exe.canonicalize().unwrap_or(exe);
        let text = canonical.to_string_lossy().into_owned();
        if let Some((bundle, _)) = text.split_once(".app/Contents/MacOS/") {
            return Some(format!("{bundle}.app"));
        }
        return Some(text);
    }
    Some(exe.to_string_lossy().into_owned())
}

pub fn repoint() {
    if available() && enabled() {
        let _ = set(true);
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_BINARY, REG_SZ, RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW};
    use windows::core::HSTRING;

    const RUN: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    const APPROVED_ON: [u8; 12] = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    fn open(path: &str, access: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Option<HKEY> {
        let mut key = HKEY::default();
        (unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, access, &mut key) } == ERROR_SUCCESS).then_some(key)
    }

    fn raw(path: &str) -> Option<Vec<u8>> {
        let key = open(path, KEY_READ)?;
        let name = HSTRING::from(super::NAME);
        let mut size = 0u32;
        let mut out = None;
        unsafe {
            if RegQueryValueExW(key, &name, None, None, None, Some(&mut size)) == ERROR_SUCCESS {
                let mut data = vec![0u8; size as usize];
                if RegQueryValueExW(key, &name, None, None, Some(data.as_mut_ptr()), Some(&mut size)) == ERROR_SUCCESS {
                    data.truncate(size as usize);
                    out = Some(data);
                }
            }
            let _ = RegCloseKey(key);
        }
        out
    }

    pub fn enabled() -> bool {
        let approved = raw(APPROVED).filter(|v| v.len() >= 8).is_none_or(|v| v[v.len() - 8..].iter().all(|b| *b == 0));
        raw(RUN).is_some() && approved
    }

    pub fn set(on: bool, exe: &str) -> bool {
        let Some(key) = open(RUN, KEY_SET_VALUE) else { return false };
        let name = HSTRING::from(super::NAME);
        let ok = unsafe {
            if on {
                let value: Vec<u8> = format!("{exe} ").encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
                RegSetValueExW(key, &name, None, REG_SZ, Some(&value)) == ERROR_SUCCESS
            } else {
                let status = RegDeleteValueW(key, &name);
                status == ERROR_SUCCESS || status == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND
            }
        };
        unsafe {
            let _ = RegCloseKey(key);
        }
        if on && let Some(approved) = open(APPROVED, KEY_SET_VALUE) {
            unsafe {
                let _ = RegSetValueExW(approved, &name, None, REG_BINARY, Some(&APPROVED_ON));
                let _ = RegCloseKey(approved);
            }
        }
        ok
    }
}

#[cfg(target_os = "macos")]
mod platform {
    fn file() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from(std::env::var_os("HOME")?).join("Library/LaunchAgents").join(format!("{}.plist", super::NAME)))
    }

    pub fn enabled() -> bool {
        file().is_some_and(|f| f.exists())
    }

    pub fn set(on: bool, exe: &str) -> bool {
        let Some(file) = file() else { return false };
        if !on {
            return std::fs::remove_file(&file).is_ok() || !file.exists();
        }
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n  <dict>\n  <key>Label</key>\n  <string>{}</string>\n  <key>ProgramArguments</key>\n  <array><string>{exe}</string></array>\n  <key>RunAtLoad</key>\n  <true/>\n  </dict>\n</plist>",
            super::NAME
        );
        file.parent().is_some_and(|dir| std::fs::create_dir_all(dir).is_ok()) && std::fs::write(&file, plist).is_ok()
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod platform {
    fn file() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from(std::env::var_os("HOME")?).join(".config/autostart").join(format!("{}.desktop", super::NAME)))
    }

    pub fn enabled() -> bool {
        file().is_some_and(|f| f.exists())
    }

    pub fn set(on: bool, exe: &str) -> bool {
        let Some(file) = file() else { return false };
        if !on {
            return std::fs::remove_file(&file).is_ok() || !file.exists();
        }
        let name = super::NAME;
        let entry = format!("[Desktop Entry]\nType=Application\nVersion=1.0\nName={name}\nComment={name}startup script\nExec={exe} \nStartupNotify=false\nTerminal=false");
        file.parent().is_some_and(|dir| std::fs::create_dir_all(dir).is_ok()) && std::fs::write(&file, entry).is_ok()
    }
}

pub fn enabled() -> bool {
    platform::enabled()
}

pub fn set(on: bool) -> bool {
    executable().is_some_and(|exe| platform::set(on, &exe))
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn round_trip_when_initially_off() {
        assert!(!super::enabled(), "автозапуск уже включён — тест не трогает реальную запись");
        assert!(super::set(true));
        assert!(super::enabled());
        assert!(super::set(false));
        assert!(!super::enabled());
    }
}
