use super::apply::{create, local_hash, present, push, remember, staged, whole};
use super::cycle::{Facts, Vault};
use super::remote::{RemoteError, RemoteFile};
use super::scan::{absolute, service, stat, UNKNOWN};
use super::SyncError;
use crate::documents::conflict::{copy_stem, ConflictCause, ConflictReport, LocalMinute};
use crate::documents::io::{free_copy, preserve_copy};
use crate::documents::merge::{apply_edits, merge_external_change};
use crate::files::content::FileContent;
use crate::files::document::text_from_bytes;
use crate::files::gate;
use crate::history::Source;
use crate::search::paths::{is_markdown, title};
use std::fs;
use std::path::Path;

const MERGE_LIMIT: u64 = 1024 * 1024;

struct Texts {
    base: String,
    theirs: String,
    ours: String,
}

pub fn resolve(vault: &Vault<'_>, facts: &Facts<'_>) -> Result<(), SyncError> {
    let remote = present(facts.remote)?;
    let path = absolute(vault.root, facts.path);
    match texts(vault, facts, &path, remote)? {
        Some(texts) => merge(vault, facts, &path, remote, &texts),
        None => keep_both(vault, facts, &path, remote),
    }
}

fn texts(vault: &Vault<'_>, facts: &Facts<'_>, path: &Path, remote: &RemoteFile) -> Result<Option<Texts>, SyncError> {
    let (Some(record), Some(local)) = (facts.record, facts.local) else {
        return Ok(None);
    };
    let small = local.size <= MERGE_LIMIT && remote.size <= MERGE_LIMIT && record.remote.size <= MERGE_LIMIT;
    if !small || !is_markdown(path) || service(facts.path) {
        return Ok(None);
    }
    let base = match text(vault, &record.remote) {
        Err(SyncError::Remote(RemoteError::Missing)) => return Ok(None),
        base => base?,
    };
    let ours = text_from_bytes(fs::read(path)?).ok();
    Ok(match (base, text(vault, remote)?, ours) {
        (Some(base), Some(theirs), Some(ours)) => Some(Texts { base, theirs, ours }),
        _ => None,
    })
}

fn text(vault: &Vault<'_>, file: &RemoteFile) -> Result<Option<String>, SyncError> {
    let mut bytes = Vec::new();
    vault.remote.download(file, &mut bytes)?;
    whole(file, bytes.len())?;
    Ok(text_from_bytes(bytes).ok())
}

fn merge(vault: &Vault<'_>, facts: &Facts<'_>, path: &Path, remote: &RemoteFile, texts: &Texts) -> Result<(), SyncError> {
    let expected = local_hash(vault, facts)?;
    let merged = merge_external_change(&texts.base, &texts.theirs, &texts.ours);
    let written = gate::write(vault.core, path, &apply_edits(&texts.ours, &merged.edits), Some(&expected), Source::External, None)?;
    if !merged.displaced.is_empty() {
        let report = ConflictReport { cause: ConflictCause::Displaced, synced_hash: None };
        preserve_copy(vault.core, path, &merged.displaced.join("\n"), report, vault.offset_seconds);
    }
    let modified = stat(&fs::metadata(path)?)?.modified;
    let uploaded = vault.remote.upload(facts.path, &FileContent::open(path)?, modified, Some(remote))?;
    remember(vault, facts.path, written.hash, UNKNOWN, uploaded)
}

fn keep_both(vault: &Vault<'_>, facts: &Facts<'_>, path: &Path, remote: &RemoteFile) -> Result<(), SyncError> {
    let label = copy_stem(&title(path), &LocalMinute::now(vault.offset_seconds), vault.core.history.device());
    let extension = path.extension().map(|extension| format!(".{}", extension.to_string_lossy())).unwrap_or_default();
    let service = service(facts.path);
    let copy = staged(vault, remote, |content| Ok(free_copy(path, &label, &extension, |target| create(vault, service, target, content))?))?;
    push(vault, facts).inspect_err(|_| {
        let _ = fs::remove_file(&copy);
    })
}
