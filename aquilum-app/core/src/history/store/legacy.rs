use super::{history_root, HISTORY_FOLDER};
use crate::files::relocate::remove_empty_folders;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

const QUANTUM_FOLDER: &str = ".quantum";

pub fn adopt_quantum_history(base: &Path) {
    let legacy = base.join(QUANTUM_FOLDER);
    let legacy_history = legacy.join(HISTORY_FOLDER);
    if !legacy_history.is_dir() {
        return;
    }
    let target = history_root(base);
    let files = WalkDir::new(&legacy_history)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    for file in files {
        let Ok(relative) = file.strip_prefix(&legacy_history) else {
            continue;
        };
        let destination = target.join(relative);
        if destination.exists() {
            continue;
        }
        let moved = destination
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::rename(&file, &destination));
        if let Err(error) = moved {
            eprintln!("[aquilum:history] версия не перенесена из {}: {error}", file.display());
        }
    }
    remove_empty_folders(&legacy);
    let _ = fs::remove_dir(&legacy);
}
