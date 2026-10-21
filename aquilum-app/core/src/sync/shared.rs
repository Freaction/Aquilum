use super::records::Records;
use super::state::State;
use super::worker::Open;
use crate::files::document::hash_bytes;
use crate::search::paths::canonical_path;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard};

pub(super) const ACCOUNT: &str = "dropbox.json";

pub(super) struct Shared {
    pub(super) folder: PathBuf,
    pub(super) open: Open,
    state: Mutex<State>,
    pub(super) signal: Condvar,
}

impl Shared {
    pub(super) fn new(folder: PathBuf, open: Open) -> Self {
        Shared { folder, open, state: Mutex::new(State::default()), signal: Condvar::new() }
    }

    pub(super) fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect("состояние синхронизации")
    }

    pub(super) fn connected(&self) -> bool {
        self.folder.join(ACCOUNT).is_file()
    }

    pub(super) fn records(&self, root: &Path) -> Option<Records> {
        if !self.connected() {
            return None;
        }
        let key = hash_bytes(canonical_path(root).to_string_lossy().as_bytes());
        Records::open(&self.folder.join(format!("{key}.sqlite3"))).ok()
    }

    pub(super) fn change(&self, edit: impl FnOnce(&mut State)) {
        let mut state = self.lock();
        edit(&mut state);
        if let Some(records) = state.vault.as_ref().and_then(|(root, ..)| self.records(root)) {
            let _ = records.remember(&state.memory);
        }
        drop(state);
        self.signal.notify_all();
    }
}
