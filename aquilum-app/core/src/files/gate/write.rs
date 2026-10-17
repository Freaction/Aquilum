use super::{announce_history, paths_changed, record};
use crate::app_core::Core;
use crate::files::content::FileContent;
use crate::files::document::{write_content_atomic, write_spliced_atomic};
use crate::files::entries::{copy_file_impl, create_content_file};
use crate::files::error::FileCommandError;
use crate::files::models::FileWriteResult;
use crate::history::{Change, NoteWrite, Source};
use crate::search::paths::{is_markdown, is_note};
use crate::text::content::{Content, Splice};
use std::fs;
use std::ops::Range;
use std::path::Path;

pub struct Saving<'a> {
    pub content: &'a dyn Content,
    pub expected_hash: Option<&'a str>,
    pub source: Source,
    pub name: Option<&'a str>,
    pub change: Option<Change<'a>>,
}

pub fn write(
    core: &Core,
    path: &Path,
    content: &str,
    expected_hash: Option<&str>,
    source: Source,
    version_name: Option<&str>,
) -> Result<FileWriteResult, FileCommandError> {
    save(core, path, Saving { content: &content, expected_hash, source, name: version_name, change: None })
}

pub fn save(core: &Core, path: &Path, saving: Saving<'_>) -> Result<FileWriteResult, FileCommandError> {
    let Saving { content, expected_hash, source, name, change } = saving;
    note_write(core, path, source, name, change, Some(content), || write_content_atomic(path, content, expected_hash))
}

pub fn edit(
    core: &Core,
    path: &Path,
    changes: &[(Range<usize>, String)],
    expected_hash: Option<&str>,
    source: Source,
    name: Option<&str>,
) -> Result<FileWriteResult, FileCommandError> {
    let change = match (changes, expected_hash) {
        ([(range, insert)], Some(base)) => Some(Change { splice: Splice { at: range.start, removed: range.len(), inserted: insert.len() }, base }),
        _ => None,
    };
    note_write(core, path, source, name, change, None, || write_spliced_atomic(path, changes, expected_hash))
}

fn note_write(
    core: &Core,
    path: &Path,
    source: Source,
    name: Option<&str>,
    change: Option<Change<'_>>,
    text: Option<&dyn Content>,
    write: impl FnOnce() -> Result<FileWriteResult, FileCommandError>,
) -> Result<FileWriteResult, FileCommandError> {
    let original = match source.is_quiet() {
        true => None,
        false => core.history.original(&|| core.known_roots(), path),
    };
    let result = write().inspect_err(|_| {
        if let Some(original) = &original {
            let _ = fs::remove_file(original);
        }
    })?;
    let written;
    let text = match text {
        Some(text) => text,
        None => {
            written = FileContent::open(path)?;
            &written
        }
    };
    record(core, path, NoteWrite { text, hash: &result.hash, source, name, change, original });
    if is_markdown(path) {
        core.search.notify_content_path(path.to_path_buf());
    }
    Ok(result)
}

pub fn create(
    core: &Core,
    path: &Path,
    content: &dyn Content,
    source: Source,
    version_name: Option<&str>,
) -> Result<FileWriteResult, FileCommandError> {
    let result = create_content_file(path, content)?;
    if is_note(path) {
        let write = NoteWrite { text: content, hash: &result.hash, source, name: version_name, change: None, original: None };
        record(core, path, write);
    }
    paths_changed(core, vec![path.to_path_buf()]);
    Ok(result)
}

pub fn create_binary(
    core: &Core,
    path: &Path,
    bytes: &[u8],
) -> Result<FileWriteResult, FileCommandError> {
    let result = create_content_file(path, &bytes)?;
    paths_changed(core, vec![path.to_path_buf()]);
    Ok(result)
}

pub fn copy(core: &Core, from: &Path, to: &Path) -> Result<(), FileCommandError> {
    copy_file_impl(from, to)?;
    paths_changed(core, vec![to.to_path_buf()]);
    Ok(())
}

pub fn name_version(
    core: &Core,
    path: &Path,
    version: Option<&str>,
    name: &str,
) -> Option<String> {
    let named = core.history.name(&|| core.known_roots(), path, version, name);
    if named.is_some() {
        announce_history(core, path);
    }
    named
}
