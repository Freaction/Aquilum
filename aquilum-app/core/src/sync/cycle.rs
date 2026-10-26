use super::decide::{settle, Action};
use super::halt::Halt;
use super::progress::Progress;
use super::measure::survey;
use super::records::{Large, Record, Records, Size};
use super::remote::{Remote, RemoteError, RemoteFile};
use super::scan::{absolute, service, LocalFile};
use super::transfer::{execute, fail, Net};
use super::view::observe;
use super::SyncError;
use crate::app_core::Core;
use crate::settings::models::SyncSettings;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub trait Watch: Sync {
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

impl Vault<'_> {
    pub fn net(&self) -> Net<'_> {
        Net { root: self.root, remote: self.remote, staging: self.staging, watch: self.watch }
    }
}

pub struct Facts<'a> {
    pub path: &'a str,
    pub local: Option<LocalFile>,
    pub hash: Option<String>,
    pub remote: Option<&'a RemoteFile>,
    pub record: Option<&'a Record>,
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
    let _ = fs::remove_dir_all(vault.staging);
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
    let paths: BTreeSet<&String> = local.files.keys().chain(view.paths()).chain(records.keys()).filter(|path| !local.kept.contains(*path)).collect();
    let mut plan = Vec::new();
    for path in paths {
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
    plan.sort_by_key(phase);
    outcome.progress = Progress::plan(plan.iter().map(|(_, action)| *action));
    vault.watch.progress(outcome.progress);
    make_folders(vault, &plan);
    for steps in plan.chunk_by(|left, right| phase(left) == phase(right)) {
        execute(vault, steps, &mut outcome)?;
        if outcome.stopped {
            return Ok(outcome);
        }
    }
    if outcome.failed.is_empty() {
        vault.records.commit(&cursor, &vault.scope.fingerprint())?;
    } else if outcome.failed.iter().any(|(_, error)| matches!(error, SyncError::Remote(RemoteError::Stale))) {
        vault.records.forget_cursor()?;
    }
    Ok(outcome)
}

fn phase((facts, action): &(Facts<'_>, Action)) -> u8 {
    match (service(facts.path), action) {
        (false, Action::DeleteLocal) => 2,
        (service, _) => u8::from(service),
    }
}

fn make_folders(vault: &Vault<'_>, plan: &[(Facts<'_>, Action)]) {
    let arriving = plan.iter().filter(|(facts, action)| matches!(action, Action::Pull) && facts.local.is_none() && !service(facts.path));
    let folders: BTreeSet<&str> = arriving.filter_map(|(facts, _)| facts.path.rsplit_once('/')).map(|(folder, _)| folder).collect();
    let mut created = BTreeSet::new();
    for folder in folders {
        let path = absolute(vault.root, folder);
        if !path.is_dir() && fs::create_dir_all(&path).is_ok() {
            created.extend(folder.match_indices('/').map(|(at, _)| &folder[..at]).chain([folder]).map(|part| absolute(vault.root, part)));
        }
    }
    vault.core.synced(created.into_iter().collect());
}

pub fn stopped(vault: &Vault<'_>, outcome: &mut Outcome) -> bool {
    outcome.stopped = vault.watch.stopped();
    outcome.stopped
}
