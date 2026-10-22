pub mod attachments;
pub mod deletions;
pub mod content;
pub mod document;
pub mod entries;
pub mod error;
pub mod gate;
pub mod models;
pub mod rename;
pub mod relocate;
pub mod trash;
pub mod watcher;
pub mod workspace;

#[cfg(test)]
mod tests;

pub(crate) fn rebase(path: &std::path::Path, from: &std::path::Path, to: &std::path::Path) -> Option<std::path::PathBuf> {
    let inside = path.strip_prefix(from).ok()?;
    Some(if inside.as_os_str().is_empty() { to.to_path_buf() } else { to.join(inside) })
}

pub fn remove_matching(dir: &std::path::Path, doomed: impl Fn(&str) -> bool) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten().filter(|entry| doomed(&entry.file_name().to_string_lossy())) {
        let path = entry.path();
        let removed = if entry.file_type().is_ok_and(|kind| kind.is_dir()) { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
        match removed {
            Ok(()) => eprintln!("[aquilum:cleanup] удалено {}", path.display()),
            Err(error) => eprintln!("[aquilum:cleanup] не удалено {}: {error}", path.display()),
        }
    }
}

pub(crate) fn missing_ok(result: std::io::Result<()>) -> std::io::Result<()> {
    match result {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
