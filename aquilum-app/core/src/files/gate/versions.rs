use super::write::{save, Saving};
use crate::app_core::Core;
use crate::documents::conflict::{ConflictCause, ConflictReport};
use crate::documents::io::preserve_copy;
use crate::files::content::FileContent;
use crate::files::document::hash_file;
use crate::files::error::FileCommandError;
use crate::history::{Change, Source};
use std::path::Path;

const RETRIES: usize = 3;
const BETWEEN: &str = "\n\n⋯\n\n";

pub fn restore_version(core: &Core, note: &Path, id: &str, source: Source, name: Option<&str>) -> Result<bool, FileCommandError> {
    retrying(note, |current| {
        let version = core.history.content(&|| core.known_roots(), note, id).ok_or_else(|| missing(id))?;
        save(core, note, Saving { content: &version, expected_hash: Some(current), source, name, change: None }).map(|_| true)
    })
}

pub fn revert_version(core: &Core, note: &Path, id: &str, source: Source, offset_seconds: i64) -> Result<bool, FileCommandError> {
    let reverted = retrying(note, |hash| {
        let current = Box::new(FileContent::open(note)?);
        let reverted = core.history.revert(&|| core.known_roots(), note, id, current)?;
        if reverted.regions.is_empty() {
            return Ok(None);
        }
        let change = Some(Change { regions: reverted.regions, base: hash });
        save(core, note, Saving { content: &reverted.text, expected_hash: Some(hash), source, name: None, change })?;
        Ok(Some(reverted.displaced))
    })?;
    let Some(displaced) = reverted else { return Ok(false) };
    if !displaced.is_empty() {
        let report = ConflictReport { cause: ConflictCause::Reverted, synced_hash: None };
        preserve_copy(core, note, &displaced.join(BETWEEN), report, offset_seconds);
    }
    Ok(true)
}

pub fn retrying<T>(note: &Path, mut attempt: impl FnMut(&str) -> Result<T, FileCommandError>) -> Result<T, FileCommandError> {
    let mut conflicts = 0;
    loop {
        match attempt(&hash_file(note)?) {
            Err(FileCommandError::Conflict { .. }) if conflicts < RETRIES => conflicts += 1,
            done => return done,
        }
    }
}

fn missing(id: &str) -> FileCommandError {
    FileCommandError::Task { message: format!("нет версии {id}") }
}
