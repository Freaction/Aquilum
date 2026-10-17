use super::{announce, edit, paths_changed, NoteMove, Relocation};
use crate::app_core::Core;
use crate::files::entries::rename_file_impl;
use crate::files::error::FileCommandError;
use crate::files::models::FileRenameResult;
use crate::files::rename::rename_with_links;
use crate::history::Source;
use crate::search::paths::is_note;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn rename(
    core: &Core,
    from: &Path,
    to: &Path,
) -> Result<FileRenameResult, FileCommandError> {
    core.documents.moving(core, from, || rename_now(core, from, to))
}

fn rename_now(core: &Core, from: &Path, to: &Path) -> Result<FileRenameResult, FileCommandError> {
    let (rewrites, relocation) = relocating(core, from, to, || rename_with_links(&core.search, from, to))?;
    let rewritten: Vec<PathBuf> = rewrites
        .into_iter()
        .filter(|rewrite| {
            edit(core, &rewrite.path, &rewrite.changes, Some(&rewrite.original_hash), Source::Links, None)
                .map_err(|error| eprintln!("[aquilum:files] ссылки в {} не переписаны: {error}", rewrite.path.display()))
                .is_ok()
        })
        .map(|rewrite| rewrite.path)
        .collect();
    let mut changed = vec![from.to_path_buf(), to.to_path_buf()];
    changed.extend(rewritten.iter().cloned());
    announce(core, relocation);
    paths_changed(core, changed);
    core.documents.reconcile_paths(core, &rewritten);
    Ok(FileRenameResult { updated_paths: rewritten })
}

pub fn move_across(core: &Core, from: &Path, to: &Path) -> Result<(), FileCommandError> {
    core.documents.moving(core, from, || move_across_now(core, from, to))
}

fn move_across_now(core: &Core, from: &Path, to: &Path) -> Result<(), FileCommandError> {
    let ((), relocation) = relocating(core, from, to, || rename_file_impl(from, to))?;
    announce(core, relocation);
    paths_changed(core, vec![from.to_path_buf(), to.to_path_buf()]);
    Ok(())
}

fn relocating<T>(
    core: &Core,
    from: &Path,
    to: &Path,
    run: impl FnOnce() -> Result<T, FileCommandError>,
) -> Result<(T, Relocation), FileCommandError> {
    let notes = notes_under(from);
    let done = run()?;
    let relocation = relocated(&notes, from, to);
    for step in &relocation.moves {
        core.history.moved(&|| core.known_roots(), Path::new(&step.from), Path::new(&step.to));
    }
    Ok((done, relocation))
}

pub(super) fn notes_under(path: &Path) -> Vec<PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && is_note(entry.path()))
        .map(|entry| entry.into_path())
        .collect()
}

pub(super) fn relocated(notes: &[PathBuf], from: &Path, to: &Path) -> Relocation {
    let moves = notes
        .iter()
        .filter_map(|note| {
            let target = crate::files::rebase(note, from, to)?;
            Some(NoteMove {
                from: note.to_string_lossy().into_owned(),
                to: target.to_string_lossy().into_owned(),
            })
        })
        .collect();
    Relocation {
        moves,
        removed: Vec::new(),
    }
}
