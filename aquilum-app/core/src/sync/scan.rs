use crate::files::trash::TRASH_FOLDER;
use crate::history::store::{millis, AQUILUM_FOLDER, HISTORY_FOLDER};
use crate::search::paths::{is_hidden, is_visible_entry};
use crate::settings::models::SyncSettings;
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use walkdir::{DirEntry, WalkDir};

#[derive(Clone)]
pub struct LocalFile {
    pub path: String,
    pub size: u64,
    pub modified: u64,
}

pub const UNKNOWN: LocalFile = LocalFile { path: String::new(), size: 0, modified: 0 };

pub(crate) fn key(path: &str) -> String {
    path.to_lowercase()
}

impl SyncSettings {
    pub fn included(self, root: &Path, relative: &str) -> bool {
        let mut parts = relative.split('/').peekable();
        let trash = parts.next_if_eq(&TRASH_FOLDER).is_some();
        let history = parts.next_if_eq(&AQUILUM_FOLDER).is_some() && parts.next_if_eq(&HISTORY_FOLDER).is_some();
        (!trash || self.trash) && (!history || self.history) && (trash || history || !is_hidden(root, &absolute(root, relative)))
    }

    pub fn fingerprint(self) -> String {
        format!("trash={};history={}", self.trash, self.history)
    }

    fn areas(self, root: &Path) -> Vec<PathBuf> {
        let trash = self.trash.then(|| root.join(TRASH_FOLDER));
        let history = self.history.then(|| root.join(AQUILUM_FOLDER).join(HISTORY_FOLDER));
        trash.into_iter().chain(history).collect()
    }
}

pub fn service(relative: &str) -> bool {
    matches!(relative.split('/').next(), Some(TRASH_FOLDER | AQUILUM_FOLDER))
}

pub fn scan(root: &Path, scope: SyncSettings) -> io::Result<Vec<LocalFile>> {
    let mut files = Vec::new();
    walk(root, root, is_visible_entry, |_| true, &mut files)?;
    for area in scope.areas(root).iter().filter(|area| area.is_dir()) {
        walk(root, area, |_| true, |relative| scope.included(root, relative), &mut files)?;
    }
    Ok(files)
}

fn walk(
    root: &Path,
    from: &Path,
    enter: impl FnMut(&DirEntry) -> bool,
    keep: impl Fn(&str) -> bool,
    files: &mut Vec<LocalFile>,
) -> io::Result<()> {
    for entry in WalkDir::new(from).into_iter().filter_entry(enter) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let relative = relative(root, entry.path());
            if keep(&relative) {
                let mut file = stat(&entry.metadata()?)?;
                file.path = relative;
                files.push(file);
            }
        }
    }
    Ok(())
}

pub fn stat(metadata: &Metadata) -> io::Result<LocalFile> {
    Ok(LocalFile { path: String::new(), size: metadata.len(), modified: millis(metadata.modified()?) })
}

pub fn stamp(path: &Path, modified: u64) -> io::Result<()> {
    if modified == 0 {
        return Ok(());
    }
    fs::File::options().write(true).open(path)?.set_modified(UNIX_EPOCH + Duration::from_millis(modified))
}

pub fn relative(root: &Path, path: &Path) -> String {
    let parts: Vec<_> = path.strip_prefix(root).unwrap_or(path).iter().map(|part| part.to_string_lossy()).collect();
    parts.join("/")
}

pub fn absolute(root: &Path, relative: &str) -> PathBuf {
    relative.split('/').fold(root.to_path_buf(), |path, part| path.join(part))
}
