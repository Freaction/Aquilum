use super::cycle::{Facts, Vault};
use super::records::{Record, Records};
use super::remote::RemoteFile;
use super::scan::{absolute, stat, LocalFile, UNKNOWN};
use super::SyncError;
use crate::files::content::FileContent;
use crate::files::document::hash_file;
use crate::settings::models::SyncSettings;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Absent,
    Same,
    Changed,
}

#[derive(Clone, Copy)]
pub enum Action {
    Push,
    Pull,
    DeleteLocal,
    DeleteRemote,
    Conflict,
    Match,
    Forget,
}

fn decide(local: Side, remote: Side, equal: bool) -> Option<Action> {
    match (local, remote) {
        (Side::Same, Side::Same) => None,
        (Side::Absent, Side::Absent) => Some(Action::Forget),
        (Side::Changed, Side::Changed) if equal => Some(Action::Match),
        (Side::Changed, Side::Changed) => Some(Action::Conflict),
        (Side::Changed, _) => Some(Action::Push),
        (_, Side::Changed) => Some(Action::Pull),
        (Side::Absent, Side::Same) => Some(Action::DeleteRemote),
        (Side::Same, Side::Absent) => Some(Action::DeleteLocal),
    }
}

pub fn settle<'a>(vault: &Vault<'_>, mut facts: Facts<'a>) -> Result<Option<(Facts<'a>, Action)>, SyncError> {
    let (local, hash) = local_change(vault.root, facts.path, facts.local, facts.record)?;
    facts.hash = hash;
    let remote = remote_side(&facts);
    let equal = local == Side::Changed && remote == Side::Changed && equal(vault, &facts)?;
    Ok(match decide(local, remote, equal) {
        Some(action) => Some((facts, action)),
        None if drifted(&facts) => Some((facts, Action::Match)),
        None => None,
    })
}

fn local_change(root: &Path, path: &str, local: Option<LocalFile>, record: Option<&Record>) -> Result<(Side, Option<String>), SyncError> {
    let (Some(local), Some(record)) = (local, record) else {
        return Ok((if local.is_some() { Side::Changed } else { Side::Absent }, None));
    };
    if local.size == record.size && local.modified == record.modified {
        return Ok((Side::Same, None));
    }
    let hash = hash_file(&absolute(root, path))?;
    Ok((if hash == record.hash { Side::Same } else { Side::Changed }, Some(hash)))
}

pub fn edited(root: &Path, scope: SyncSettings, records: &Records, path: &str) -> bool {
    if !scope.included(root, path) {
        return false;
    }
    let local = fs::metadata(absolute(root, path)).ok().filter(|metadata| metadata.is_file()).and_then(|metadata| stat(&metadata).ok());
    let record = records.get(path).ok().flatten();
    if let (Some(local), Some(record)) = (local, &record) {
        if record.modified != UNKNOWN.modified && local.size != record.size {
            return true;
        }
    }
    match local_change(root, path, local, record.as_ref()) {
        Ok((Side::Same, _)) => false,
        Ok((Side::Absent, _)) => record.is_some() || records.holds_under(path).unwrap_or(true),
        _ => true,
    }
}

fn remote_side(facts: &Facts<'_>) -> Side {
    match (facts.remote, facts.record) {
        (None, _) => Side::Absent,
        (Some(file), Some(record)) if same_version(file, &record.remote) => Side::Same,
        _ => Side::Changed,
    }
}

fn same_version(file: &RemoteFile, recorded: &RemoteFile) -> bool {
    file.rev == recorded.rev || !file.checksum.is_empty() && file.checksum == recorded.checksum
}

fn equal(vault: &Vault<'_>, facts: &Facts<'_>) -> Result<bool, SyncError> {
    let (Some(local), Some(remote)) = (facts.local, facts.remote) else {
        return Ok(false);
    };
    if local.size != remote.size {
        return Ok(false);
    }
    let checksum = vault.remote.checksum(&FileContent::open(&absolute(vault.root, facts.path))?)?;
    Ok(checksum == remote.checksum)
}

fn drifted(facts: &Facts<'_>) -> bool {
    match (facts.record, facts.local, facts.remote) {
        (Some(record), Some(local), Some(remote)) => {
            local.size != record.size || local.modified != record.modified || remote.rev != record.remote.rev
        }
        _ => false,
    }
}
