use super::entries::rename_file_impl;
use super::error::FileCommandError;
use crate::search::paths::same_path;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct Relocated {
    pub root: PathBuf,
    pub files: Vec<(PathBuf, PathBuf)>,
}

#[derive(Clone, Copy)]
pub enum Placement {
    Trash,
    Workspace,
}

pub fn relocate(
    source: &Path,
    target: &Path,
    placement: Placement,
) -> Result<Relocated, FileCommandError> {
    let mut files = Vec::new();
    let root = relocate_into(source, target, placement, SystemTime::now(), &mut files)?;
    Ok(Relocated { root, files })
}

pub fn move_folder(
    source: &Path,
    target: &Path,
    placement: Placement,
) -> Result<(), FileCommandError> {
    if !source.is_dir() {
        return Ok(());
    }
    if same_path(source, target) {
        return rename_file_impl(source, target);
    }
    relocate(source, target, placement).map(|_| ())
}

pub fn remove_empty_ancestors(path: &Path, root: &Path) {
    for folder in path.ancestors().skip(1) {
        if folder == root || !folder.starts_with(root) || fs::remove_dir(folder).is_err() {
            break;
        }
    }
}

pub fn remove_files(paths: &[PathBuf]) -> u64 {
    paths
        .iter()
        .filter(|path| match fs::remove_file(path) {
            Ok(()) => true,
            Err(error) => {
                eprintln!("[aquilum:files] файл не удалён {}: {error}", path.display());
                false
            }
        })
        .count() as u64
}

pub fn remove_empty_folders(root: &Path) {
    let folders = WalkDir::new(root)
        .min_depth(1)
        .contents_first(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    for folder in folders {
        let empty = fs::read_dir(&folder).is_ok_and(|mut entries| entries.next().is_none());
        if !empty {
            continue;
        }
        if let Err(error) = fs::remove_dir(&folder) {
            eprintln!("[aquilum:files] пустая папка не удалена {}: {error}", folder.display());
        }
    }
}

fn relocate_into(
    source: &Path,
    target: &Path,
    placement: Placement,
    moved_at: SystemTime,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<PathBuf, FileCommandError> {
    if source.is_dir() && target.is_dir() {
        for child in fs::read_dir(source)? {
            let child = child?;
            relocate_into(&child.path(), &target.join(child.file_name()), placement, moved_at, files)?;
        }
        fs::remove_dir(source)?;
        return Ok(target.to_path_buf());
    }
    let target = match placement {
        Placement::Trash => free_trash_path(target),
        Placement::Workspace => free_workspace_path(target),
    };
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    rename_file_impl(source, &target)?;
    let stamp = matches!(placement, Placement::Trash).then_some(moved_at);
    record_moved(source, &target, stamp, files);
    Ok(target)
}

fn free_trash_path(wanted: &Path) -> PathBuf {
    let (Some(parent), Some(name)) = (wanted.parent(), wanted.file_name()) else {
        return wanted.to_path_buf();
    };
    std::iter::once(wanted.to_path_buf())
        .chain((1..).map(|copy| parent.join(format!(".{copy}")).join(name)))
        .find(|candidate| !candidate.exists())
        .unwrap_or_else(|| wanted.to_path_buf())
}

fn free_workspace_path(wanted: &Path) -> PathBuf {
    if !wanted.exists() {
        return wanted.to_path_buf();
    }
    let parent = wanted.parent().unwrap_or_else(|| Path::new(""));
    let stem = wanted
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let suffix = wanted
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()))
        .unwrap_or_default();
    (1..)
        .map(|copy| parent.join(format!("{stem} ({copy}){suffix}")))
        .find(|candidate| !candidate.exists())
        .unwrap_or_else(|| wanted.to_path_buf())
}

fn record_moved(
    source: &Path,
    moved: &Path,
    stamp: Option<SystemTime>,
    files: &mut Vec<(PathBuf, PathBuf)>,
) {
    for entry in WalkDir::new(moved).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(from) = super::rebase(entry.path(), moved, source) else { continue };
        files.push((from, entry.path().to_path_buf()));
        let Some(deleted_at) = stamp else {
            continue;
        };
        let stamped = fs::File::options()
            .write(true)
            .open(entry.path())
            .and_then(|file| file.set_modified(deleted_at));
        if let Err(error) = stamped {
            eprintln!("[aquilum:trash] дата удаления не проставлена {}: {error}", entry.path().display());
        }
    }
}
