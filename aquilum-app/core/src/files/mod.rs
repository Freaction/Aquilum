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
