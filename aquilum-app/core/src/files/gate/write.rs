use super::{announce_history, paths_changed, record};
use crate::app_core::Core;
use crate::files::document::write_file_atomic_impl;
use crate::files::entries::{copy_file_impl, create_binary_file_impl, create_file_impl};
use crate::files::error::FileCommandError;
use crate::files::models::FileWriteResult;
use crate::history::{NoteWrite, Source};
use crate::search::paths::{is_markdown, is_note};
use std::path::Path;
use std::time::SystemTime;

pub fn write(
    core: &Core,
    path: &Path,
    content: &str,
    expected_hash: Option<&str>,
    source: Source,
    version_name: Option<&str>,
) -> Result<FileWriteResult, FileCommandError> {
    let before = core.history.disk_before(path, expected_hash);
    let result = write_file_atomic_impl(path, content, expected_hash)?;
    let write = NoteWrite { disk_before: before, text: content, hash: &result.hash, source, name: version_name };
    record(core, path, write);
    if is_markdown(path) {
        core.search.notify_content_path(path.to_path_buf());
    }
    Ok(result)
}

pub fn create(
    core: &Core,
    path: &Path,
    content: &str,
    source: Source,
    version_name: Option<&str>,
) -> Result<FileWriteResult, FileCommandError> {
    let result = create_file_impl(path, content)?;
    if is_note(path) {
        let write = NoteWrite {
            disk_before: Some((String::new(), SystemTime::now())),
            text: content,
            hash: &result.hash,
            source,
            name: version_name,
        };
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
    let result = create_binary_file_impl(path, bytes)?;
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
