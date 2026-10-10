use std::process::Command;

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
