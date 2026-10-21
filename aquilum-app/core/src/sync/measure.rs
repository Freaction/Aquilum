use super::cycle::{Vault, LARGE};
use super::records::{Large, Record, Size};
use super::scan::{scan, service, LocalFile};
use std::collections::{HashMap, HashSet};
use std::io;

pub struct Local {
    pub files: HashMap<String, LocalFile>,
    pub kept: HashSet<String>,
}

pub fn survey(vault: &Vault<'_>, records: &HashMap<String, Record>) -> io::Result<Local> {
    let (mut local, mut size) = (Local { files: HashMap::new(), kept: HashSet::new() }, Size::default());
    let mut large = Vec::new();
    for (path, file) in scan(vault.root, vault.scope)? {
        let fits = file.size <= LARGE || vault.allowed.contains(&path) || records.get(&path).is_some_and(|record| record.remote.size > LARGE);
        if file.size > LARGE && !service(&path) {
            large.push(Large { path: path.clone(), size: file.size, kept: !fits });
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
