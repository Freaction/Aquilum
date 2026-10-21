use super::apply::apply;
use super::decide::{settle, Action};
use super::halt::Halt;
use super::measure::survey;
use super::records::{Large, Record, Records, Size};
use super::remote::{Remote, RemoteError, RemoteFile};
use super::scan::{service, LocalFile};
use super::view::observe;
use super::SyncError;
use crate::app_core::Core;
use crate::files::error::FileCommandError;
use crate::settings::models::SyncSettings;
use std::collections::BTreeSet;
use std::path::Path;

pub trait Watch {
    fn measured(&self, large: Vec<Large>, size: Size);
    fn progress(&self, progress: Progress);
    fn stopped(&self) -> bool;
}

pub struct Vault<'a> {
    pub core: &'a Core,
    pub root: &'a Path,
    pub remote: &'a dyn Remote,
    pub records: &'a Records,
    pub staging: &'a Path,
    pub offset_seconds: i64,
    pub scope: SyncSettings,
    pub watch: &'a dyn Watch,
    pub allow_deletions: bool,
    pub allowed: &'a [String],
}

pub struct Facts<'a> {
    pub path: &'a str,
    pub local: Option<LocalFile>,
    pub hash: Option<String>,
    pub remote: Option<&'a RemoteFile>,
    pub record: Option<&'a Record>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

pub const LARGE: u64 = 10 * 1024 * 1024;
const GUARDED_FROM: usize = 10;

#[derive(Default)]
pub struct Outcome {
    pub progress: Progress,
    pub failed: Vec<(String, SyncError)>,
    pub stopped: bool,
}

pub fn run(vault: &Vault<'_>) -> Result<Outcome, SyncError> {
    let remote = Halt { inner: vault.remote, watch: vault.watch };
    let vault = &Vault { remote: &remote, ..*vault };
    let records = vault.records.all()?;
    let local = survey(vault, &records)?;
    let mut outcome = Outcome::default();
    if stopped(vault, &mut outcome) {
        return Ok(outcome);
    }
    let (view, cursor) = observe(vault)?;
    if view.emptied() && !records.is_empty() {
        return Err(RemoteError::Failed("папка в облаке пуста или пропала, а файлы уже синхронизировались — проход остановлен".into()).into());
    }
    let paths = local.files.keys().chain(view.paths()).chain(records.keys()).filter(|path| !local.kept.contains(*path));
    let paths: BTreeSet<(bool, &String)> = paths.map(|path| (!service(path), path)).collect();
    let mut plan = Vec::new();
    for (_, path) in paths {
        if stopped(vault, &mut outcome) {
            return Ok(outcome);
        }
        let record = records.get(path);
        let facts = Facts { path, local: local.files.get(path).copied(), hash: None, remote: view.get(path, record), record };
        match settle(vault, facts) {
            Ok(step) => plan.extend(step),
            Err(error) => fail(vault, &mut outcome, path, error)?,
        }
    }
    let deletions = plan.iter().filter(|(_, action)| matches!(action, Action::DeleteLocal | Action::DeleteRemote)).count();
    if !vault.allow_deletions && records.len() >= GUARDED_FROM && deletions * 2 > records.len() {
        return Err(SyncError::Deletions { planned: deletions, synced: records.len() });
    }
    outcome.progress.total = plan.iter().filter(|(_, action)| counted(*action)).count();
    vault.watch.progress(outcome.progress);
    for (facts, action) in plan {
        if stopped(vault, &mut outcome) {
            return Ok(outcome);
        }
        if let Err(error) = apply(vault, &facts, action) {
            fail(vault, &mut outcome, facts.path, error)?;
        }
        if counted(action) {
            outcome.progress.done += 1;
            vault.watch.progress(outcome.progress);
        }
    }
    if outcome.failed.is_empty() {
        vault.records.commit(&cursor, &vault.scope.fingerprint())?;
    } else if outcome.failed.iter().any(|(_, error)| matches!(error, SyncError::Remote(RemoteError::Stale))) {
        vault.records.forget_cursor()?;
    }
    Ok(outcome)
}

fn stopped(vault: &Vault<'_>, outcome: &mut Outcome) -> bool {
    outcome.stopped = vault.watch.stopped();
    outcome.stopped
}

fn fail(vault: &Vault<'_>, outcome: &mut Outcome, path: &str, error: SyncError) -> Result<(), SyncError> {
    match error {
        SyncError::Remote(RemoteError::Auth) => return Err(error),
        SyncError::Changed | SyncError::File(FileCommandError::Conflict { .. }) => vault.records.forget_stat(path)?,
        _ => {}
    }
    outcome.failed.push((path.to_owned(), error));
    Ok(())
}

fn counted(action: Action) -> bool {
    !matches!(action, Action::Match | Action::Forget)
}
