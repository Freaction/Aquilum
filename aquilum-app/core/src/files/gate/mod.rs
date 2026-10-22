mod moves;
mod trash;
mod versions;
mod write;

#[cfg(test)]
mod tests;

pub use moves::{move_across, rename};
pub use trash::{restore, restore_deletion, trash};
pub use versions::{restore_version, retrying, revert_version};
pub use write::{copy, create, create_binary, edit, name_version, save, write, Saving};

use super::attachments::forget_attachments_if_touched;
use crate::app_core::{path_names, Core, CoreEvent};
use crate::history::NoteWrite;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Relocation {
    pub moves: Vec<NoteMove>,
    pub removed: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct NoteHistoryChanged {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NoteMove {
    pub from: String,
    pub to: String,
}

fn record(core: &Core, note: &Path, write: NoteWrite<'_>) {
    if core.history.saved(&|| core.known_roots(), note, write) {
        announce_history(core, note);
    }
}

fn paths_changed(core: &Core, paths: Vec<PathBuf>) {
    forget_attachments_if_touched(&paths);
    core.emit(CoreEvent::WorkspaceChanged(path_names(&paths)));
    core.search.queue(&paths, false);
}

fn announce(core: &Core, relocation: Relocation) {
    if relocation.moves.is_empty() && relocation.removed.is_empty() {
        return;
    }
    core.emit(CoreEvent::NotesRelocated(relocation));
}

fn announce_history(core: &Core, note: &Path) {
    let changed = NoteHistoryChanged {
        path: note.to_string_lossy().into_owned(),
    };
    core.emit(CoreEvent::NoteHistoryChanged(changed));
}
