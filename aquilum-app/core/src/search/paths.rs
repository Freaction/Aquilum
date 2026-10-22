use super::error::SearchError;
use crate::files::trash::TRASH_FOLDER;
use crate::history::store::QUANTUM_FOLDER;
use crate::history::AQUILUM_FOLDER;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use unicode_normalization::UnicodeNormalization;
use walkdir::{DirEntry, WalkDir};

const HIDDEN_NAMES: [&str; 6] = [AQUILUM_FOLDER, TRASH_FOLDER, QUANTUM_FOLDER, ".git", ".obsidian", ".DS_Store"];

pub fn title(path: impl AsRef<Path>) -> String {
    path.as_ref().file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn identity(path: &Path) -> String {
    let normalized: String = slash_path(path).nfc().collect();
    let normalized = normalized
        .strip_prefix("//?/UNC/")
        .map(|path| format!("//{path}"))
        .or_else(|| normalized.strip_prefix("//?/").map(ToOwned::to_owned))
        .unwrap_or(normalized);
    if cfg!(any(windows, target_os = "macos")) {
        normalized.trim_end_matches('/').to_lowercase()
    } else {
        normalized.trim_end_matches('/').to_owned()
    }
}

pub fn canonical_workspace(workspace: &str) -> Result<PathBuf, SearchError> {
    let canonical = fs::canonicalize(workspace).map_err(|error| SearchError::InvalidWorkspace {
        message: error.to_string(),
    })?;
    Ok(normalize_extended_prefix(canonical))
}

pub fn same_workspace(root: &Path, workspace: &str) -> bool {
    canonical_workspace(workspace).is_ok_and(|workspace| same_path(root, &workspace))
}

pub fn same_path(left: &Path, right: &Path) -> bool {
    identity(left) == identity(right)
}

pub fn strip_root<'a>(root: &Path, path: &'a Path) -> Option<&'a Path> {
    let mut rest = path.components();
    let inside = root.components().all(|part| {
        rest.next()
            .is_some_and(|other| same_path(Path::new(&part), Path::new(&other)))
    });
    inside.then_some(rest.as_path())
}

pub fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn relative_slash_path(root: &Path, path: &Path) -> String {
    slash_path(path.strip_prefix(root).unwrap_or(path))
}

pub fn strip_markdown_extension(value: &str) -> &str {
    value
        .get(..value.len().saturating_sub(3))
        .filter(|_| {
            value
                .get(value.len().saturating_sub(3)..)
                .is_some_and(|end| end.eq_ignore_ascii_case(".md"))
        })
        .unwrap_or(value)
}

pub fn inside(root: &Path, path: &str) -> Result<PathBuf, SearchError> {
    let path = canonical_path(Path::new(path));
    match path.starts_with(root) {
        true => Ok(path),
        false => Err(SearchError::invalid_workspace("Document is outside workspace")),
    }
}

pub fn canonical_path(path: &Path) -> PathBuf {
    let resolved = fs::canonicalize(path).or_else(|error| match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => fs::canonicalize(parent).map(|parent| parent.join(name)),
        _ => Err(error),
    });
    normalize_extended_prefix(resolved.unwrap_or_else(|_| path.to_path_buf()))
}

fn normalize_extended_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let value = path.to_string_lossy();
        if let Some(path) = value.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{path}"));
        }
        if let Some(path) = value.strip_prefix(r"\\?\") {
            return PathBuf::from(path);
        }
    }
    path
}

pub fn is_visible_entry(entry: &DirEntry) -> bool {
    entry.depth() == 0 || !is_hidden_name(entry.file_name())
}

pub fn markdown_files(root: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(is_visible_entry)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && is_markdown(entry.path()))
        .map(DirEntry::into_path)
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

pub fn is_note(path: &Path) -> bool {
    is_markdown(path) || is_plain_text(path)
}

pub fn is_plain_text(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
}

pub fn is_hidden_name(name: &OsStr) -> bool {
    HIDDEN_NAMES.iter().any(|hidden| name == *hidden) || name.as_encoded_bytes().starts_with(b"._")
}

pub fn is_hidden(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok_and(|relative| relative.components().any(|component| matches!(component, Component::Normal(name) if is_hidden_name(name))))
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
