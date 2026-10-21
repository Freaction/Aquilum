use super::conflict::resolve;
use super::cycle::{Facts, Vault};
use super::decide::Action;
use super::records::Record;
use super::remote::{RemoteError, RemoteFile};
use super::scan::{absolute, service, stamp, LocalFile, UNKNOWN};
use super::SyncError;
use crate::files::content::FileContent;
use crate::files::document::{hash_bytes, hash_file, write_content_atomic};
use crate::files::entries::create_content_file;
use crate::files::error::FileCommandError;
use crate::files::gate::{self, Saving};
use crate::files::models::FileWriteResult;
use crate::history::Source;
use crate::text::content::Content;
use std::fs;
use std::path::Path;

pub fn apply(vault: &Vault<'_>, facts: &Facts<'_>, action: Action) -> Result<(), SyncError> {
    match action {
        Action::Push => push(vault, facts),
        Action::Pull => pull(vault, facts),
        Action::DeleteLocal => delete_local(vault, facts),
        Action::DeleteRemote => delete_remote(vault, facts),
        Action::Conflict => resolve(vault, facts),
        Action::Match => matched(vault, facts),
        Action::Forget => Ok(vault.records.remove(facts.path)?),
    }
}

pub fn present<T>(value: Option<T>) -> Result<T, SyncError> {
    value.ok_or(SyncError::Changed)
}

pub fn remember(vault: &Vault<'_>, path: &str, hash: String, local: LocalFile, mut remote: RemoteFile) -> Result<(), SyncError> {
    remote.path = path.to_owned();
    Ok(vault.records.put(&Record { hash, size: local.size, modified: local.modified, remote })?)
}

pub fn local_hash(vault: &Vault<'_>, facts: &Facts<'_>) -> Result<String, SyncError> {
    match (&facts.hash, &facts.record) {
        (Some(hash), _) => Ok(hash.clone()),
        (None, Some(record)) => Ok(record.hash.clone()),
        (None, None) => Ok(hash_file(&absolute(vault.root, facts.path))?),
    }
}

pub fn push(vault: &Vault<'_>, facts: &Facts<'_>) -> Result<(), SyncError> {
    let local = present(facts.local)?;
    let hash = local_hash(vault, facts)?;
    let content = FileContent::open(&absolute(vault.root, facts.path))?;
    let uploaded = vault.remote.upload(facts.path, &content, local.modified, facts.remote)?;
    remember(vault, facts.path, hash, local, uploaded)
}

fn pull(vault: &Vault<'_>, facts: &Facts<'_>) -> Result<(), SyncError> {
    let remote = present(facts.remote)?;
    let path = absolute(vault.root, facts.path);
    let written = staged(vault, remote, |content| write_local(vault, facts, &path, content))?;
    stamp(&path, remote.modified)?;
    remember(vault, facts.path, written.hash, UNKNOWN, remote.clone())
}

fn write_local(vault: &Vault<'_>, facts: &Facts<'_>, path: &Path, content: &FileContent) -> Result<FileWriteResult, SyncError> {
    if facts.local.is_none() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        return Ok(create(vault, service(facts.path), path, content)?);
    }
    let expected = local_hash(vault, facts)?;
    if service(facts.path) {
        return Ok(write_content_atomic(path, content, Some(&expected))?);
    }
    let saving = Saving { content, expected_hash: Some(&expected), source: Source::External, name: None, change: None };
    Ok(gate::save(vault.core, path, saving)?)
}

pub fn create(vault: &Vault<'_>, service: bool, path: &Path, content: &dyn Content) -> Result<FileWriteResult, FileCommandError> {
    if service {
        create_content_file(path, content)
    } else {
        gate::create(vault.core, path, content, Source::External, None)
    }
}

pub fn staged<T>(vault: &Vault<'_>, remote: &RemoteFile, use_content: impl FnOnce(&FileContent) -> Result<T, SyncError>) -> Result<T, SyncError> {
    fs::create_dir_all(vault.staging)?;
    let path = vault.staging.join(hash_bytes(format!("{}\n{}", remote.path, remote.rev).as_bytes()));
    let result = download(vault, remote, &path).and_then(|content| use_content(&content));
    let _ = fs::remove_file(&path);
    result
}

fn download(vault: &Vault<'_>, remote: &RemoteFile, path: &Path) -> Result<FileContent, SyncError> {
    vault.remote.download(remote, &mut fs::File::create(path)?)?;
    let content = FileContent::open(path)?;
    whole(remote, content.byte_len())?;
    Ok(content)
}

pub fn whole(remote: &RemoteFile, length: usize) -> Result<(), SyncError> {
    if length as u64 == remote.size {
        return Ok(());
    }
    Err(RemoteError::Failed(format!("{} скачан не полностью", remote.path)).into())
}

fn delete_local(vault: &Vault<'_>, facts: &Facts<'_>) -> Result<(), SyncError> {
    let record = present(facts.record)?;
    let path = absolute(vault.root, facts.path);
    if hash_file(&path)? != record.hash {
        return Err(SyncError::Changed);
    }
    if service(facts.path) {
        fs::remove_file(&path)?;
    } else {
        gate::trash(vault.core, vault.root, &path)?;
    }
    Ok(vault.records.remove(facts.path)?)
}

fn delete_remote(vault: &Vault<'_>, facts: &Facts<'_>) -> Result<(), SyncError> {
    let remote = present(facts.remote)?;
    if absolute(vault.root, facts.path).exists() {
        return Err(SyncError::Changed);
    }
    vault.remote.delete(remote)?;
    Ok(vault.records.remove(facts.path)?)
}

fn matched(vault: &Vault<'_>, facts: &Facts<'_>) -> Result<(), SyncError> {
    remember(vault, facts.path, local_hash(vault, facts)?, present(facts.local)?, present(facts.remote)?.clone())
}
