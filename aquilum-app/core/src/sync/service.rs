use super::decide::edited;
use super::dropbox::{self, CLIENT};
use super::records::{Large, Size};
use super::scan::relative;
use super::shared::{Shared, ACCOUNT};
use super::state::{State, SyncStatus};
use super::worker::{dropbox, work, Open};
use super::{Account, SyncError, Tokens};
use crate::app_core::{Core, CoreEvent};
use crate::files::{missing_ok, remove_matching};
use crate::search::paths::canonical_path;
use crate::settings::models::SyncSettings;
use crate::web_agent::web_agent;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Weak};
use std::thread;
use std::time::{Duration, Instant};

pub struct SyncService {
    pub(super) shared: Arc<Shared>,
}

impl SyncService {
    pub fn new(app_data: &Path) -> Self {
        Self::with(app_data.join("sync"), Box::new(dropbox))
    }

    pub(super) fn with(folder: PathBuf, open: Open) -> Self {
        Self { shared: Arc::new(Shared::new(folder, open)) }
    }

    pub fn start(&self, core: Weak<Core>) {
        let shared = Arc::clone(&self.shared);
        thread::Builder::new().name("aquilum-sync".into()).spawn(move || work(&shared, &core)).expect("поток синхронизации");
    }

    pub fn status(&self) -> SyncStatus {
        self.shared.lock().status(self.shared.connected())
    }

    pub fn large(&self) -> Vec<Large> {
        self.shared.lock().memory.large.clone()
    }

    pub fn size(&self) -> Size {
        self.shared.lock().memory.size
    }

    pub fn last(&self) -> Option<u64> {
        self.shared.lock().memory.done
    }

    pub fn due(&self) -> Option<Instant> {
        let state = self.shared.lock();
        let waiting = state.vault.is_some() && matches!(state.status(self.shared.connected()), SyncStatus::Ready | SyncStatus::Offline | SyncStatus::Unsynced { .. } | SyncStatus::Failed(_));
        state.due.filter(|_| waiting)
    }

    pub fn set_vault(&self, vault: Option<(PathBuf, SyncSettings)>, offset_seconds: i64) {
        let vault = vault.filter(|(_, sync)| sync.enabled);
        if vault.as_ref().is_some_and(|(root, sync)| self.reschedule(root, *sync)) {
            return;
        }
        let memory = vault.as_ref().and_then(|(root, _)| self.shared.records(root)).map(|records| records.recall()).unwrap_or_default();
        self.shared.change(|state| {
            state.restart();
            state.memory = memory;
            state.vault = vault.map(|(root, sync)| (root, sync, offset_seconds));
        });
    }

    fn reschedule(&self, root: &Path, sync: SyncSettings) -> bool {
        let mut state = self.shared.lock();
        let Some((current, scope, _)) = &mut state.vault else { return false };
        let same_scope = SyncSettings { interval: sync.interval, ..*scope };
        if current.as_path() != root || same_scope != sync {
            return false;
        }
        scope.interval = sync.interval;
        drop(state);
        self.request(Duration::from_secs(sync.interval));
        true
    }

    pub fn touched(&self, paths: &[PathBuf], rescan: bool) -> bool {
        let Some((root, scope, _)) = self.shared.lock().vault.clone() else { return false };
        let Some(records) = self.shared.records(&root) else { return false };
        if rescan {
            return true;
        }
        let watched = canonical_path(&root);
        paths.iter().any(|path| path.starts_with(&watched) && edited(&root, scope, &records, &relative(&watched, path)))
    }

    pub fn request(&self, delay: Duration) {
        self.shared.lock().due = Some(Instant::now() + delay);
        self.shared.signal.notify_all();
    }

    pub fn allow_deletions(&self) {
        self.shared.change(|state| {
            state.allow_deletions = true;
            state.due = Some(Instant::now());
        });
    }

    pub fn allow(&self, paths: Vec<String>) {
        self.shared.change(|state| {
            state.memory.large.iter_mut().filter(|large| paths.contains(&large.path)).for_each(|large| large.kept = false);
            state.memory.allowed.extend(paths);
            state.due = Some(Instant::now());
        });
    }

    pub fn connect(&self, core: &Arc<Core>, links: Option<Receiver<String>>) -> io::Result<String> {
        self.stop_login();
        let login = CLIENT.login(links)?;
        let url = login.url.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let (shared, core, stop) = (Arc::clone(&self.shared), Arc::downgrade(core), Arc::clone(&cancel));
        let mut state = self.shared.lock();
        let waiting = thread::spawn(move || {
            let tokens = login.finish(&web_agent("sync"), &stop).map_err(|error| error.to_string());
            let connected = tokens.and_then(|tokens| start_over(&shared.folder, tokens).map_err(|error| error.to_string()));
            shared.change(|state| {
                state.login = None;
                match connected {
                    Ok(()) => state.restart(),
                    Err(_) if stop.load(Ordering::Relaxed) => {}
                    Err(error) => state.problem = Some(SyncStatus::Failed(error)),
                }
            });
            if let Some(core) = core.upgrade() {
                core.emit(CoreEvent::SyncChanged);
            }
        });
        state.problem = None;
        state.login = Some((cancel, waiting));
        Ok(url)
    }

    pub fn disconnect(&self) -> io::Result<()> {
        self.stop_login();
        let removed = missing_ok(fs::remove_file(self.shared.folder.join(ACCOUNT)));
        self.shared.change(State::restart);
        removed
    }

    fn stop_login(&self) {
        let login = self.shared.lock().login.take();
        if let Some((cancel, waiting)) = login {
            cancel.store(true, Ordering::Relaxed);
            let _ = waiting.join();
        }
    }

    pub fn close(&self) {
        self.stop_login();
        self.shared.lock().closed = true;
        self.shared.signal.notify_all();
    }

    pub fn prune(&self, keep: &HashSet<String>) {
        remove_matching(&self.shared.folder, |name| name.split_once(".sqlite3").is_some_and(|(key, _)| !keep.contains(key)));
    }

    pub fn vaults(&self) -> Result<Vec<String>, SyncError> {
        Ok(dropbox::vaults(super::worker::account(&self.shared.folder)?, web_agent("sync"))?)
    }
}

impl Drop for SyncService {
    fn drop(&mut self) {
        self.close();
    }
}

fn start_over(folder: &Path, tokens: Tokens) -> io::Result<()> {
    missing_ok(fs::remove_dir_all(folder))?;
    fs::create_dir_all(folder)?;
    Account::new(CLIENT, web_agent("sync"), folder.join(ACCOUNT), tokens).map(drop)
}
