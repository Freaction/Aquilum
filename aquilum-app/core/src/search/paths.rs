use super::error::SearchError;
use std::fs;
use std::path::{Component, Path, PathBuf};
use unicode_normalization::UnicodeNormalization;
use walkdir::{DirEntry, WalkDir};

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
    entry.depth() == 0 || !entry.file_name().to_string_lossy().starts_with('.')
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

pub fn is_hidden(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok_and(|relative| {
        relative.components().any(|component| match component {
            Component::Normal(value) => value.to_string_lossy().starts_with('.'),
            _ => false,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_workspace, is_hidden, is_markdown, relative_slash_path, same_path,
        same_workspace, strip_markdown_extension, strip_root,
    };
    use std::path::Path;

    #[test]
    fn accepts_markdown_case_insensitively() {
        assert!(is_markdown(Path::new("Note.MD")));
        assert!(!is_markdown(Path::new("Note.txt")));
    }

    #[cfg(windows)]
    #[test]
    fn paths_differing_only_by_cyrillic_case_are_the_same() {
        assert!(same_path(Path::new("C:/База/Идея.md"), Path::new(r"c:\БАЗА\идея.md")));
        assert!(!same_path(Path::new("C:/База/Идея.md"), Path::new("C:/База/Идеи.md")));
        assert_eq!(
            strip_root(Path::new("C:/БАЗА"), Path::new("c:/база/Проекты/Идея.md")),
            Some(Path::new("Проекты/Идея.md"))
        );
        assert_eq!(strip_root(Path::new("C:/База"), Path::new("C:/Другая/Идея.md")), None);
    }

    #[test]
    fn relative_paths_use_forward_slashes_and_lose_only_the_markdown_extension() {
        let root = Path::new("vault");
        assert_eq!(relative_slash_path(root, &root.join("Проекты").join("Идея.md")), "Проекты/Идея.md");
        assert_eq!(strip_markdown_extension("Проекты/Идея.MD"), "Проекты/Идея");
        assert_eq!(strip_markdown_extension("рисунок.png"), "рисунок.png");
    }

    #[test]
    fn excludes_hidden_workspace_segments() {
        let root = Path::new("C:/vault");
        assert!(is_hidden(root, Path::new("C:/vault/.trash/note.md")));
        assert!(!is_hidden(root, Path::new("C:/vault/notes/note.md")));
    }

    #[test]
    fn note_history_versions_never_reach_search() {
        let root = Path::new("C:/vault");
        let version = "1790000000000_aaaaaaaa_agent.md";
        assert!(is_hidden(root, &Path::new("C:/vault/.aquilum/history/Идея.md").join(version)));
        assert!(is_hidden(root, &Path::new("C:/vault/.trash/.aquilum/history/Идея.md").join(version)));
    }

    #[test]
    fn a_path_that_does_not_exist_yet_resolves_through_its_folder() {
        let directory = tempfile::tempdir().unwrap();
        let folder = super::canonical_path(directory.path());
        assert_eq!(super::canonical_path(&directory.path().join("Новая.md")), folder.join("Новая.md"));
    }

    #[test]
    fn canonical_path_is_safe_for_frontend() {
        let directory = tempfile::tempdir().unwrap();
        let path = canonical_workspace(directory.path().to_str().unwrap()).unwrap();
        assert!(!path.to_string_lossy().starts_with(r"\\?\"));
        assert!(same_workspace(&path, path.to_str().unwrap()));
    }

    #[cfg(windows)]
    #[test]
    fn accepts_extended_workspace_paths() {
        let directory = tempfile::tempdir().unwrap();
        let root = canonical_workspace(directory.path().to_str().unwrap()).unwrap();
        let extended = format!(r"\\?\{}", root.display());
        assert!(same_workspace(&root, &extended));
    }

    #[test]
    fn decomposed_and_composed_names_share_one_identity() {
        let composed = std::path::Path::new("Base/Мой план.md");
        let decomposed = std::path::Path::new("Base/Мои\u{306} план.md");
        assert_ne!(composed.to_str(), decomposed.to_str());
        assert_eq!(super::identity(composed), super::identity(decomposed));
    }
}
