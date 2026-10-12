static CRASH_LOG: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

pub(crate) fn report(message: &str) {
    eprintln!("{message}");
    let Some(file) = CRASH_LOG.get() else { return };
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    if let Ok(mut log) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
        let _ = std::io::Write::write_all(&mut log, format!("[{stamp}] {message}\n").as_bytes());
    }
}

pub(super) fn install_crash_log(file: std::path::PathBuf) {
    let _ = CRASH_LOG.set(file.clone());
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let thread = std::thread::current().name().unwrap_or("?").to_owned();
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[{stamp}] поток {thread}: {info}\n{backtrace}\n\n");
        if let Ok(mut log) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
            let _ = std::io::Write::write_all(&mut log, entry.as_bytes());
        }

        let copied = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(entry.clone())).is_ok();
        let prompt = if copied {
            "В приложении произошла непредвиденная ошибка.\nОтчёт об ошибке скопирован в буфер обмена.\n\nПерезапустить Aquilum?"
        } else {
            "В приложении произошла непредвиденная ошибка.\n\nПерезапустить Aquilum?"
        };

        let restart = rfd::MessageDialog::new()
            .set_title("Упс, приложение упало!")
            .set_description(prompt)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();

        if restart == rfd::MessageDialogResult::Yes
            && let Ok(exe) = std::env::current_exe()
        {
            let _ = std::process::Command::new(exe).spawn();
        }

        default(info);
    }));
}
