use std::path::Path;

use aquilum_core::Core;
use aquilum_core::documents::merge::merge_external_change;
use aquilum_core::files::document::read_file_snapshot_impl;
use aquilum_core::files::error::FileCommandError;
use aquilum_core::files::gate;
use aquilum_core::history::Source;

use super::Version;

const WRITE_RETRIES: usize = 3;

pub struct Texts {
    pub text: String,
    pub previous: Option<String>,
}

pub fn texts(core: &Core, note: &Path, id: &str) -> Option<Texts> {
    let texts = core.history.version(&|| core.known_roots(), note, Some(id))?;
    Some(Texts { text: texts.text, previous: texts.previous })
}

fn rewrite(core: &Core, note: &Path, source: Source, next: impl Fn(&str) -> String) -> Result<bool, FileCommandError> {
    let mut conflicts = 0;
    loop {
        let snapshot = read_file_snapshot_impl(note)?;
        let content = next(&snapshot.content);
        if content == snapshot.content {
            return Ok(false);
        }
        match gate::write(core, note, &content, Some(&snapshot.hash), source, None) {
            Ok(_) => return Ok(true),
            Err(FileCommandError::Conflict { .. }) if conflicts < WRITE_RETRIES => conflicts += 1,
            Err(error) => return Err(error),
        }
    }
}

pub fn restore(core: &Core, note: &Path, version: &Version) -> Result<bool, FileCommandError> {
    let Some(texts) = texts(core, note, &version.id) else { return Ok(false) };
    rewrite(core, note, Source::Restore(version.at_ms), |_| texts.text.clone())
}

pub fn revert(core: &Core, note: &Path, version: &Version) -> Result<bool, FileCommandError> {
    let Some(Texts { text, previous: Some(previous) }) = texts(core, note, &version.id) else { return Ok(false) };
    rewrite(core, note, Source::Revert(version.at_ms), |current| {
        let merged = merge_external_change(&text, &previous, current);
        aquilum_core::documents::merge::apply_edits(current, &merged.edits)
    })
}
