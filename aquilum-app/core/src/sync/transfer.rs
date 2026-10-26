use super::apply::{apply, fetch};
use super::cycle::{stopped, Facts, Outcome, Vault, Watch};
use super::decide::Action;
use super::remote::{Remote, RemoteError};
use super::SyncError;
use crate::files::error::FileCommandError;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

pub const TRANSFERS: usize = 4;

pub struct Net<'a> {
    pub root: &'a Path,
    pub remote: &'a dyn Remote,
    pub staging: &'a Path,
    pub watch: &'a dyn Watch,
}

pub fn execute(vault: &Vault<'_>, steps: &[(Facts<'_>, Action)], outcome: &mut Outcome) -> Result<(), SyncError> {
    let (net, next) = (vault.net(), AtomicUsize::new(0));
    thread::scope(|scope| {
        let (sender, results) = mpsc::channel();
        for _ in 0..TRANSFERS.min(steps.len()) {
            let (sender, net, next) = (sender.clone(), &net, &next);
            scope.spawn(move || {
                while let Some((facts, action)) = steps.get(next.fetch_add(1, Ordering::Relaxed)).filter(|_| !net.watch.stopped()) {
                    if sender.send((facts, *action, fetch(net, facts, *action))).is_err() {
                        return;
                    }
                }
            });
        }
        drop(sender);
        for (facts, action, fetched) in results {
            if stopped(vault, outcome) {
                return Ok(());
            }
            if let Err(error) = fetched.and_then(|step| apply(vault, facts, step)) {
                fail(vault, outcome, facts.path, error)?;
            }
            if outcome.progress.step(action) {
                vault.watch.progress(outcome.progress);
            }
        }
        Ok(())
    })
}

pub fn fail(vault: &Vault<'_>, outcome: &mut Outcome, path: &str, error: SyncError) -> Result<(), SyncError> {
    match error {
        SyncError::Remote(RemoteError::Auth) => return Err(error),
        SyncError::Changed | SyncError::File(FileCommandError::Conflict { .. }) => vault.records.forget_stat(path)?,
        _ => {}
    }
    outcome.failed.push((path.to_owned(), error));
    Ok(())
}
