use super::conflict::{copy_content, copy_stem, ConflictReport, LocalMinute};
use crate::app_core::Core;
use crate::files::entries::first_free;
use crate::files::error::FileCommandError;
use crate::files::gate;
use crate::history::Source;
use crate::text::content::Content;
use std::path::{Path, PathBuf};

fn copy_path(note: &Path, label: &str, attempt: usize, extension: &str) -> PathBuf {
    let name = if attempt == 0 { format!("{label}{extension}") } else { format!("{label} {}{extension}", attempt + 1) };
    note.with_file_name(name)
}

pub fn free_copy<T>(note: &Path, label: &str, extension: &str, create: impl FnMut(&Path) -> Result<T, FileCommandError>) -> Result<PathBuf, FileCommandError> {
    first_free(|attempt| copy_path(note, label, attempt, extension), create)
}

pub fn preserve_copy(core: &Core, path: &Path, body: &dyn Content, report: ConflictReport, offset_seconds: i64) -> Option<PathBuf> {
    let stem = crate::search::paths::title(path);
    let moment = LocalMinute::now(offset_seconds);
    let note = path.to_string_lossy();
    let content = copy_content(&note, &stem, &report, &moment, body)?;
    let label = copy_stem(&stem, &moment, core.history.device());
    free_copy(path, &label, ".md", |target| gate::create(core, target, &content, Source::Me, None))
        .map_err(|error| eprintln!("[aquilum:documents] копия-конфликт {label} не записана: {error}"))
        .ok()
}
