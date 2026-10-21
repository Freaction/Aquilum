use super::{files, free_name, millis};
use crate::files::document::{read_text, write_content_atomic};
use crate::files::error::FileCommandError;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(super) const NAME_EXTENSION: &str = ".name";

pub fn remove_version_and_names(dir: &Path, version_id: &str) {
    let _ = fs::remove_file(dir.join(version_id));
    for (file_name, path) in files(dir) {
        if let Some((version, _, _)) = name_mark(&file_name) {
            if version == version_id {
                let _ = fs::remove_file(path);
            }
        }
    }
}

pub fn write_name(
    dir: &Path,
    device: &str,
    version_id: &str,
    name: &str,
    at: SystemTime,
) -> Result<(), FileCommandError> {
    let path = free_name(dir, millis(at), |at| {
        format!("{version_id}.{at:013}_{device}{NAME_EXTENSION}")
    });
    write_content_atomic(&path, &name.trim(), None).map(|_| ())
}

pub fn names(dir: &Path) -> HashMap<String, String> {
    let mut marks = files(dir)
        .filter_map(|(file_name, path)| {
            let (version, at, device) = name_mark(&file_name)?;
            Some((version.to_owned(), at, device.to_owned(), path))
        })
        .collect::<Vec<_>>();
    marks.sort();
    marks
        .into_iter()
        .map(|(version, _, _, path)| (version, path))
        .collect::<HashMap<_, _>>()
        .into_iter()
        .filter_map(|(version, path)| {
            let name = read_text(&path).ok()?.trim().to_owned();
            (!name.is_empty()).then_some((version, name))
        })
        .collect()
}

pub(super) fn orphan_names(dir: &Path) -> Vec<PathBuf> {
    files(dir)
        .filter_map(|(file_name, path)| {
            let (version, _, _) = name_mark(&file_name)?;
            (!dir.join(version).exists()).then_some(path)
        })
        .collect()
}

fn name_mark(file_name: &str) -> Option<(&str, u64, &str)> {
    let (version, stamp) = file_name.strip_suffix(NAME_EXTENSION)?.rsplit_once('.')?;
    let (at, device) = stamp.split_once('_')?;
    Some((version, at.parse().ok()?, device))
}
