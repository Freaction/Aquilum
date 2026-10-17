use super::write::{save, Saving};
use crate::app_core::Core;
use crate::documents::conflict::{ConflictCause, ConflictReport};
use crate::documents::io::preserve_copy;
use crate::files::content::FileContent;
use crate::files::document::hash_file;
use crate::files::error::FileCommandError;
use crate::history::Source;
use crate::documents::merge::{apply_edits, merge_external_change};
use crate::text::content::{read_string, Content};
use crate::text::pieces::Pieces;
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
        let current = FileContent::open(note)?;
        let plan = core.history.revert_plan(&|| core.known_roots(), note, id, &current).ok_or_else(|| missing(id))?;
        if plan.windows.is_empty() {
            return Ok(None);
        }
        let len = current.byte_len();
        let mut merged = Vec::new();
        let mut displaced = Vec::new();
        for window in &plan.windows {
            let mine = read_string(&current, window.current.clone())?;
            let found = merge_external_change(&read_string(&plan.version, window.version.clone())?, &read_string(&plan.previous, window.previous.clone())?, &mine);
            displaced.extend(found.displaced);
            merged.push(apply_edits(&mine, &found.edits));
        }
        let mut text: Pieces<Box<dyn Content>> = Pieces::new(vec![Box::new(current)]);
        let mut cursor = 0;
        for (window, result) in plan.windows.iter().zip(merged) {
            text.push(0, cursor..window.current.start);
            let size = result.len();
            let source = text.source(Box::new(result));
            text.push(source, 0..size);
            cursor = window.current.end;
        }
        text.push(0, cursor..len);
        save(core, note, Saving { content: &text, expected_hash: Some(hash), source, name: None, change: None })?;
        Ok(Some(displaced))
    })?;
    let Some(displaced) = reverted else { return Ok(false) };
    if !displaced.is_empty() {
        let report = ConflictReport { cause: ConflictCause::Reverted, disk_hash: None, synced_hash: None };
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
