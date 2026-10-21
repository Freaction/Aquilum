use super::{announce, paths_changed, Relocation};
use crate::app_core::Core;
use crate::files::deletions::deletion_files;
use crate::files::error::FileCommandError;
use crate::files::trash::{move_to_trash_impl, restore_from_trash_impl};
use crate::search::paths::is_note;
use std::path::{Path, PathBuf};

pub fn trash(core: &Core, workspace: &Path, path: &Path) -> Result<PathBuf, FileCommandError> {
    let moved = move_to_trash_impl(workspace, path)?;
    let notes = note_moves(&moved.files);
    let history = &core.history;
    for (note, trashed_at) in &notes {
        history.trashed(&|| core.known_roots(), note, trashed_at);
    }
    announce(core, removed(notes.iter().map(|(note, _)| note)));
    paths_changed(core, vec![path.to_path_buf(), moved.root.clone()]);
    Ok(moved.root)
}

pub fn restore(
    core: &Core,
    workspace: &Path,
    trashed: &Path,
) -> Result<PathBuf, FileCommandError> {
    let mut roots = restore_all(core, workspace, &[trashed.to_path_buf()])?;
    Ok(roots.remove(0))
}

pub fn restore_deletion(
    core: &Core,
    workspace: &Path,
    id: &str,
) -> Result<(), FileCommandError> {
    restore_all(core, workspace, &deletion_files(workspace, id)).map(|_| ())
}

fn restore_all(
    core: &Core,
    workspace: &Path,
    trashed: &[PathBuf],
) -> Result<Vec<PathBuf>, FileCommandError> {
    let history = &core.history;
    let mut roots = Vec::new();
    let mut failure = None;
    for path in trashed {
        match restore_from_trash_impl(workspace, path) {
            Ok(restored) => {
                for (trashed_note, back) in note_moves(&restored.files) {
                    history.restored(&|| core.known_roots(), &trashed_note, &back);
                }
                roots.push(restored.root);
            }
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    paths_changed(core, trashed.iter().chain(&roots).cloned().collect());
    failure.map_or(Ok(roots), Err)
}

fn note_moves(files: &[(PathBuf, PathBuf)]) -> Vec<(PathBuf, PathBuf)> {
    files
        .iter()
        .filter(|(from, _)| is_note(from))
        .cloned()
        .collect()
}

pub(super) fn removed<'a>(notes: impl Iterator<Item = &'a PathBuf>) -> Relocation {
    Relocation {
        moves: Vec::new(),
        removed: notes.map(|note| note.to_string_lossy().into_owned()).collect(),
    }
}
