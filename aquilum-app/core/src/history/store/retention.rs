use super::names::{names, orphan_names, NAME_EXTENSION};
use super::{history_root, millis, versions, VersionFile};
use crate::history::delta;
use crate::history::rebuild::{is_delta, rebuild};
use crate::files::relocate::{remove_empty_folders, remove_files};
use crate::files::trash::files_in;
use std::fs;
use std::io;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub(super) const DAY: Duration = Duration::from_secs(24 * 60 * 60);

pub struct Usage {
    pub versions: u64,
    pub bytes: u64,
}

pub fn usage(vault: &Path) -> Usage {
    files_in(&history_root(vault)).fold(Usage { versions: 0, bytes: 0 }, |usage, (path, metadata)| Usage {
        versions: usage.versions + u64::from(!path.to_string_lossy().ends_with(NAME_EXTENSION)),
        bytes: usage.bytes + metadata.len(),
    })
}

pub fn clear(vault: &Path) -> io::Result<()> {
    crate::files::missing_ok(fs::remove_dir_all(history_root(vault)))
}

pub fn remove_expired(vault: &Path, retention_days: u32) -> u64 {
    let root = history_root(vault);
    if retention_days == 0 || !root.is_dir() {
        return 0;
    }
    let cutoff = millis(SystemTime::now()).saturating_sub(millis(UNIX_EPOCH + DAY * retention_days));
    let dirs = WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    let mut removed = 0;
    for dir in dirs {
        let expired = expire(&dir, cutoff);
        removed += remove_files(&expired);
        remove_files(&orphan_names(&dir));
    }
    remove_empty_folders(&root);
    removed
}

fn expire(dir: &Path, cutoff: u64) -> Vec<PathBuf> {
    let all = versions(dir);
    if all.is_empty() {
        return Vec::new();
    }
    let named = names(dir);
    let gone: HashSet<&str> = all
        .iter()
        .filter(|version| version.at_ms < cutoff && !named.contains_key(&version.id))
        .map(|version| version.id.as_str())
        .collect();
    for version in all.iter().filter(|version| !gone.contains(version.id.as_str())) {
        if leans_on(dir, version, &gone) {
            settle(dir, version);
        }
    }
    gone.iter().map(|id| dir.join(id)).collect()
}

fn leans_on(dir: &Path, version: &VersionFile, gone: &HashSet<&str>) -> bool {
    is_delta(&version.id)
        && delta::read(&dir.join(&version.id)).is_ok_and(|delta| delta.base.is_some_and(|base| gone.contains(base.as_str())))
}

fn settle(dir: &Path, version: &VersionFile) {
    let path = dir.join(&version.id);
    let settled = delta::read(&path).and_then(|old| {
        let text = rebuild(dir, &version.id)?;
        delta::write(&path, None, &old.hash, &delta::whole(old.len), &text).map_err(std::io::Error::other)
    });
    if let Err(error) = settled {
        eprintln!("[aquilum:history] версия {} не переписана перед удалением старых: {error}", path.display());
    }
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
