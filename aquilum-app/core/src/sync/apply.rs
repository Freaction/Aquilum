use super::conflict::resolve;
use super::cycle::{Facts, Vault};
use super::decide::Action;
use super::records::Record;
use super::remote::{RemoteError, RemoteFile};
use super::scan::{absolute, service, stamp, LocalFile, UNKNOWN};
use super::transfer::Net;
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
use std::path::{Path, PathBuf};

pub enum Step {
    Write(Staged),
    Remember(String, RemoteFile),
    Forget,
    DeleteLocal,
    Resolve,
    Match,
}

pub struct Staged(PathBuf);

impl Staged {
    pub fn open(&self) -> Result<FileContent, SyncError> {
        Ok(FileContent::open(&self.0)?)
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub fn fetch(net: &Net<'_>, facts: &Facts<'_>, action: Action) -> Result<Step, SyncError> {
    Ok(match action {
        Action::Pull => Step::Write(stage(net, present(facts.remote)?)?),
        Action::Push => {
            let (hash, uploaded) = upload(net, facts)?;
            Step::Remember(hash, uploaded)
        }
        Action::DeleteRemote => delete_remote(net, facts)?,
        Action::Forget => Step::Forget,
        Action::DeleteLocal => Step::DeleteLocal,
        Action::Conflict => Step::Resolve,
        Action::Match => Step::Match,
    })
}

pub fn apply(vault: &Vault<'_>, facts: &Facts<'_>, step: Step) -> Result<(), SyncError> {
    match step {
        Step::Write(staged) => pull(vault, facts, &staged),
        Step::Remember(hash, uploaded) => remember(vault, facts.path, hash, present(facts.local)?, uploaded),
        Step::Forget => Ok(vault.records.remove(facts.path)?),
        Step::DeleteLocal => delete_local(vault, facts),
        Step::Resolve => resolve(vault, facts),
        Step::Match => remember(vault, facts.path, local_hash(vault.root, facts)?, present(facts.local)?, present(facts.remote)?.clone()),
    }
}

pub fn present<T>(value: Option<T>) -> Result<T, SyncError> {
    value.ok_or(SyncError::Changed)
}

pub fn remember(vault: &Vault<'_>, path: &str, hash: String, local: LocalFile, mut remote: RemoteFile) -> Result<(), SyncError> {
    remote.path = path.to_owned();
    Ok(vault.records.put(&Record { hash, size: local.size, modified: local.modified, remote })?)
}

pub fn local_hash(root: &Path, facts: &Facts<'_>) -> Result<String, SyncError> {
    match (&facts.hash, &facts.record) {
        (Some(hash), _) => Ok(hash.clone()),
        (None, Some(record)) => Ok(record.hash.clone()),
        (None, None) => Ok(hash_file(&absolute(root, facts.path))?),
    }
}

pub fn upload(net: &Net<'_>, facts: &Facts<'_>) -> Result<(String, RemoteFile), SyncError> {
    let local = present(facts.local)?;
    let hash = local_hash(net.root, facts)?;
    let content = FileContent::open(&absolute(net.root, facts.path))?;
    Ok((hash, net.remote.upload(facts.path, &content, local.modified, facts.remote)?))
}

pub fn stage(net: &Net<'_>, remote: &RemoteFile) -> Result<Staged, SyncError> {
    fs::create_dir_all(net.staging)?;
    let staged = Staged(net.staging.join(hash_bytes(format!("{}\n{}", remote.path, remote.rev).as_bytes())));
    net.remote.download(remote, &mut fs::File::create(&staged.0)?)?;
    whole(remote, fs::metadata(&staged.0)?.len() as usize)?;
    Ok(staged)
}

pub fn whole(remote: &RemoteFile, length: usize) -> Result<(), SyncError> {
    if length as u64 == remote.size {
        return Ok(());
    }
    Err(RemoteError::Failed(format!("{} скачан не полностью", remote.path)).into())
}

fn pull(vault: &Vault<'_>, facts: &Facts<'_>, staged: &Staged) -> Result<(), SyncError> {
    let remote = present(facts.remote)?;
    let path = absolute(vault.root, facts.path);
    let written = write_local(vault, facts, &path, &staged.open()?)?;
    stamp(&path, remote.modified)?;
    remember(vault, facts.path, written.hash, UNKNOWN, remote.clone())
}

fn write_local(vault: &Vault<'_>, facts: &Facts<'_>, path: &Path, content: &FileContent) -> Result<FileWriteResult, SyncError> {
    let source = if vault.scope.history { Source::Synced } else { Source::External };
    if facts.local.is_none() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        return Ok(create(vault, service(facts.path), path, content, source)?);
    }
    let expected = local_hash(vault.root, facts)?;
    if service(facts.path) {
        return Ok(write_content_atomic(path, content, Some(&expected))?);
    }
    let saving = Saving { content, expected_hash: Some(&expected), source, name: None, change: None };
    Ok(gate::save(vault.core, path, saving)?)
}

pub fn create(vault: &Vault<'_>, service: bool, path: &Path, content: &dyn Content, source: Source) -> Result<FileWriteResult, FileCommandError> {
    if service {
        create_content_file(path, content)
    } else {
        gate::create(vault.core, path, content, source, None)
    }
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

fn delete_remote(net: &Net<'_>, facts: &Facts<'_>) -> Result<Step, SyncError> {
    let remote = present(facts.remote)?;
    if absolute(net.root, facts.path).exists() {
        return Err(SyncError::Changed);
    }
    net.remote.delete(remote)?;
    Ok(Step::Forget)
}
