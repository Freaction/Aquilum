static CRASH_LOG: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

const LOG_LIMIT: u64 = 1024 * 1024;

pub(crate) fn report(message: &str) {
    eprintln!("{message}");
    let Some(file) = CRASH_LOG.get() else { return };
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    append(file, &format!("[{stamp}] {message}\n"));
}

fn append(file: &std::path::Path, entry: &str) {
    let full = std::fs::metadata(file).is_ok_and(|meta| meta.len() > LOG_LIMIT);
    if let Ok(mut log) = std::fs::OpenOptions::new().create(true).write(true).append(!full).truncate(full).open(file) {
        let _ = std::io::Write::write_all(&mut log, entry.as_bytes());
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
        append(&file, &entry);

        let copied = crate::clipboard::copy(entry.clone());
        let prompt = if copied {
            "В приложении произошла непредвиденная ошибка.\nОтчёт об ошибке скопирован в буфер обмена.\n\nПерезапустить Aquilum?"
        } else {
            "В приложении произошла непредвиденная ошибка.\n\nПерезапустить Aquilum?"
        };

        if crate::native_dialog::confirm("Упс, приложение упало!", prompt) {
            crate::system::restart();
        }

        default(info);
    }));
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_full_log_starts_over_instead_of_growing() {
        let directory = crate::test_dir::tempdir();
        let file = directory.path().join("crash.log");
        std::fs::write(&file, vec![b'x'; super::LOG_LIMIT as usize + 1]).unwrap();

        super::append(&file, "новая запись\n");
        super::append(&file, "ещё одна\n");

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "новая запись\nещё одна\n");
    }
}
