use super::cycle::Vault;
use super::records::Record;
use super::remote::RemoteFile;
use super::SyncError;
use std::collections::{HashMap, HashSet};

pub enum View {
    Full(HashMap<String, RemoteFile>),
    Delta { changed: HashMap<String, RemoteFile>, removed: HashSet<String> },
}

impl View {
    pub fn paths(&self) -> impl Iterator<Item = &String> {
        match self {
            Self::Full(files) | Self::Delta { changed: files, .. } => files.keys(),
        }
    }

    pub fn get<'a>(&'a self, path: &str, record: Option<&'a Record>) -> Option<&'a RemoteFile> {
        match self {
            Self::Full(files) => files.get(path),
            Self::Delta { changed, removed } => {
                changed.get(path).or_else(|| record.filter(|_| !removed_with_folder(removed, path)).map(|record| &record.remote))
            }
        }
    }

    pub fn emptied(&self) -> bool {
        matches!(self, Self::Full(files) if files.is_empty())
    }
}

fn removed_with_folder(removed: &HashSet<String>, path: &str) -> bool {
    !removed.is_empty() && path.match_indices('/').map(|(end, _)| &path[..end]).chain([path]).any(|prefix| removed.contains(prefix))
}

pub fn observe(vault: &Vault<'_>) -> Result<(View, String), SyncError> {
    let same_scope = vault.records.state("scope")?.as_deref() == Some(vault.scope.fingerprint().as_str());
    let delta = match vault.records.state("cursor")? {
        Some(cursor) if same_scope => vault.remote.delta(&cursor)?,
        _ => None,
    };
    let included = |files: Vec<RemoteFile>| -> HashMap<String, RemoteFile> {
        files.into_iter().filter(|file| vault.scope.included(vault.root, &file.path)).map(|file| (file.path.clone(), file)).collect()
    };
    Ok(match delta {
        Some(delta) => (View::Delta { changed: included(delta.changed), removed: delta.removed.into_iter().collect() }, delta.cursor),
        None => {
            let cursor = vault.remote.cursor()?;
            (View::Full(included(vault.remote.list()?)), cursor)
        }
    })
}
