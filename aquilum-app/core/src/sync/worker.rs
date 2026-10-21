use super::cycle::Watch;
use super::dropbox::{Dropbox, CLIENT};
use super::records::{Large, Size};
use super::shared::{Shared, ACCOUNT};
use super::{run, Account, Outcome, Progress, Remote, RemoteError, SyncError, Vault};
use crate::app_core::{Core, CoreEvent};
use crate::settings::models::SyncSettings;
use crate::web_agent::{agent, web_agent};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

pub type Open = Box<dyn Fn(&Path, &str) -> Result<Arc<dyn Remote>, SyncError> + Send + Sync>;

const TRANSFER_TIMEOUT: Duration = Duration::from_secs(60);
const SHOW_EVERY: Duration = Duration::from_millis(100);
const RECHECK: Duration = Duration::from_secs(5);

struct Job {
    root: PathBuf,
    scope: SyncSettings,
    offset_seconds: i64,
    allowed: Vec<String>,
    allow_deletions: bool,
    epoch: u64,
}

struct Pass<'a> {
    shared: &'a Shared,
    core: &'a Core,
    epoch: u64,
    shown: Cell<Instant>,
}

impl Watch for Pass<'_> {
    fn measured(&self, large: Vec<Large>, size: Size) {
        self.shared.change(|state| {
            if state.epoch == self.epoch {
                (state.memory.large, state.memory.size) = (large, size);
            }
        });
        self.core.emit(CoreEvent::SyncChanged);
    }

    fn progress(&self, progress: Progress) {
        let mut state = self.shared.lock();
        if state.epoch == self.epoch {
            state.running = Some(progress);
        }
        drop(state);
        if progress.done == progress.total || self.shown.get().elapsed() >= SHOW_EVERY {
            self.shown.set(Instant::now());
            self.core.emit(CoreEvent::SyncChanged);
        }
    }

    fn stopped(&self) -> bool {
        let state = self.shared.lock();
        state.closed || state.epoch != self.epoch
    }
}

pub fn work(shared: &Shared, core: &Weak<Core>) {
    while let Some(job) = wait(shared) {
        let Some(core) = core.upgrade() else { return };
        core.emit(CoreEvent::SyncChanged);
        let watch = Pass { shared, core: &core, epoch: job.epoch, shown: Cell::new(Instant::now()) };
        let passed = pass(shared, &core, &job, &watch);
        shared.change(|state| {
            if state.epoch == job.epoch {
                state.settle(passed);
            }
        });
        core.emit(CoreEvent::SyncChanged);
    }
}

fn wait(shared: &Shared) -> Option<Job> {
    let mut state = shared.lock();
    loop {
        if state.closed {
            return None;
        }
        let now = Instant::now();
        match (state.vault.clone(), state.due) {
            (Some((root, scope, offset_seconds)), Some(due)) if due <= now && shared.connected() => {
                state.due = Some(now + Duration::from_secs(scope.interval));
                state.running = Some(Progress::default());
                let (allowed, allow_deletions) = (state.memory.allowed.clone(), std::mem::take(&mut state.allow_deletions));
                return Some(Job { root, scope, offset_seconds, allowed, allow_deletions, epoch: state.epoch });
            }
            (Some(_), Some(due)) if due > now => state = shared.signal.wait_timeout(state, (due - now).min(RECHECK)).expect("ожидание синхронизации").0,
            _ => state = shared.signal.wait(state).expect("ожидание синхронизации"),
        }
    }
}

fn pass(shared: &Shared, core: &Core, job: &Job, watch: &dyn Watch) -> Result<Outcome, SyncError> {
    let records = shared.records(&job.root).ok_or(RemoteError::Auth)?;
    let name = job.root.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let remote = (shared.open)(&shared.folder, &name)?;
    let staging = shared.folder.join("staging");
    let vault = Vault {
        core,
        root: &job.root,
        remote: &*remote,
        records: &records,
        staging: &staging,
        offset_seconds: job.offset_seconds,
        scope: job.scope,
        watch,
        allow_deletions: job.allow_deletions,
        allowed: &job.allowed,
    };
    run(&vault)
}

pub fn dropbox(folder: &Path, name: &str) -> Result<Arc<dyn Remote>, SyncError> {
    let account = Account::open(CLIENT, web_agent("sync"), folder.join(ACCOUNT))?.ok_or(RemoteError::Auth)?;
    Ok(Arc::new(Dropbox::new(account, agent("sync", TRANSFER_TIMEOUT), name)))
}
