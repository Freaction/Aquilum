use std::process::Command;

#[cfg(target_os = "macos")]
pub fn prefers_reduced_motion() -> bool {
    objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

#[cfg(not(target_os = "macos"))]
pub fn prefers_reduced_motion() -> bool {
    false
}

pub fn open(target: &str) {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(target);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(target);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(target);
        command
    };
    if let Err(error) = command.spawn() {
        eprintln!("{target} не открыт: {error}");
    }
}
