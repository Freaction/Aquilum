use super::records::Records;
use super::state::State;
use super::worker::Open;
use crate::search::paths::{cache_key, canonical_path};
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
        Records::open(&self.folder.join(format!("{}.sqlite3", cache_key(&canonical_path(root))))).ok()
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
