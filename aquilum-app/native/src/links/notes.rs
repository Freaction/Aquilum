use std::path::{Path, PathBuf};

use aquilum_core::search::paths::{markdown_files, strip_markdown_extension};
use aquilum_core::search::wiki::target::split;

use crate::file_ops::sanitize_file_name;

pub fn find_note(workspace: &Path, target: &str) -> Option<PathBuf> {
    let note = split(target).note;
    let name = strip_markdown_extension(note.rsplit(['/', '\\']).next()?).to_lowercase();
    if name.is_empty() {
        return None;
    }
    markdown_files(workspace).find(|path| path.file_stem().is_some_and(|stem| stem.to_string_lossy().to_lowercase() == name))
}

pub fn linked_file_path(workspace: &Path, target: &str) -> Option<PathBuf> {
    let note = split(target).note.replace('\\', "/");
    let segments: Vec<&str> = strip_markdown_extension(&note).split('/').collect();
    if segments.iter().any(|s| s.is_empty() || *s != s.trim() || sanitize_file_name(s) != *s) {
        return None;
    }
    let (last, folders) = segments.split_last()?;
    let mut path = workspace.to_path_buf();
    path.extend(folders);
    path.push(format!("{last}.md"));
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_file_path_rules() {
        let root = Path::new("/kb");
        assert_eq!(linked_file_path(root, "Папка/Заметка"), Some(root.join("Папка").join("Заметка.md")));
        assert_eq!(linked_file_path(root, "a.md"), Some(root.join("a.md")));
        assert_eq!(linked_file_path(root, "плохо: имя"), None);
        assert_eq!(linked_file_path(root, "a//b"), None);
        assert_eq!(linked_file_path(root, "v1.2 план"), Some(root.join("v1.2 план.md")));
        assert_eq!(linked_file_path(root, "Отчёт.MD"), Some(root.join("Отчёт.md")));
    }

    #[test]
    fn anchors_and_labels_never_reach_a_file_name() {
        let root = Path::new("/kb");
        assert_eq!(linked_file_path(root, "Заметка#Заголовок"), Some(root.join("Заметка.md")));
        assert_eq!(linked_file_path(root, "Папка/Заметка#^abc|подпись"), Some(root.join("Папка").join("Заметка.md")));
        assert_eq!(linked_file_path(root, " Заметка # Раздел "), Some(root.join("Заметка.md")));
        assert_eq!(linked_file_path(root, "#Заголовок"), None);
        assert_eq!(linked_file_path(root, ""), None);
    }

    #[test]
    fn find_note_ignores_the_anchor_and_extension_case() {
        let vault = crate::test_dir::tempdir();
        std::fs::create_dir_all(vault.path().join("Папка")).unwrap();
        let note = vault.path().join("Папка").join("Книга.MD");
        std::fs::write(&note, "").unwrap();
        assert_eq!(find_note(vault.path(), "книга#Раздел"), Some(note.clone()));
        assert_eq!(find_note(vault.path(), "Папка/Книга.md|подпись"), Some(note));
        assert_eq!(find_note(vault.path(), "#Раздел"), None);
    }
}
