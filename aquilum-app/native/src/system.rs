use std::path::Path;
use std::process::Command;

pub fn open(target: &str) {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(error) = Command::new(program).arg(target).spawn() {
        report!("{target} не открыт: {error}");
    }
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

#[cfg(not(target_os = "windows"))]
pub fn reveal(path: &Path) {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg("-R").arg(path);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(path.parent().unwrap_or(path));
        command
    };
    if let Err(error) = command.spawn() {
        report!("{} не показан в папке: {error}", path.display());
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
