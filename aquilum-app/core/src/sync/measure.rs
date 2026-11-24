use super::conflict::set_aside;
use super::cycle::{Outcome, Vault, LARGE};
use super::records::{Large, Record, Size};
use super::scan::{key, scan, service, LocalFile};
use super::transfer::fail;
use super::SyncError;
use std::collections::{HashMap, HashSet};

pub struct Local {
    pub files: HashMap<String, LocalFile>,
    pub kept: HashSet<String>,
}

pub fn survey(vault: &Vault<'_>, records: &HashMap<String, Record>, outcome: &mut Outcome) -> Result<Local, SyncError> {
    let (mut local, mut size) = (Local { files: HashMap::new(), kept: HashSet::new() }, Size::default());
    let mut large = Vec::new();
    for (path, file) in distinct(vault, records, outcome)? {
        let fits = file.size <= LARGE || vault.allowed.iter().any(|allowed| key(allowed) == path) || records.get(&path).is_some_and(|record| record.remote.size > LARGE);
        if file.size > LARGE && !service(&file.path) {
            large.push(Large { path: file.path.clone(), size: file.size, kept: !fits });
        }
        if fits {
            size.synced += file.size;
            size.files += 1;
            local.files.insert(path, file);
        } else {
            local.kept.insert(path);
        }
    }
    large.sort_by(|left: &Large, right| right.size.cmp(&left.size));
    vault.watch.measured(large, size);
    Ok(local)
}

fn distinct(vault: &Vault<'_>, records: &HashMap<String, Record>, outcome: &mut Outcome) -> Result<HashMap<String, LocalFile>, SyncError> {
    let mut files: HashMap<String, LocalFile> = HashMap::new();
    let mut twins = Vec::new();
    for file in scan(vault.root, vault.scope)? {
        let path = key(&file.path);
        let Some(known) = files.get_mut(&path) else {
            files.insert(path, file);
            continue;
        };
        if known.path != file.path {
            let recorded = records.get(&path).is_some_and(|record| record.remote.path == file.path);
            twins.push(if recorded { std::mem::replace(known, file) } else { file });
        }
    }
    for twin in twins {
        match set_aside(vault, &twin.path) {
            Ok(moved) => {
                files.insert(key(&moved.path), moved);
            }
            Err(error) => fail(vault, outcome, &twin.path, error)?,
        }
    }
    Ok(files)
}
