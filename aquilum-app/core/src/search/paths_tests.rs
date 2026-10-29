use super::{
    canonical_workspace, is_hidden, is_markdown, markdown_files, relative_slash_path,
    same_workspace, strip_markdown_extension,
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
    use super::{same_path, strip_root};
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
fn only_service_folders_and_system_files_are_hidden() {
    let root = Path::new("C:/vault");
    for hidden in [".trash/note.md", ".aquilum/history/note.md", ".quantum/history/note.md", ".git/HEAD", ".obsidian/app.json", "Проекты/.git/config", ".DS_Store", "Проекты/._note.md"] {
        assert!(is_hidden(root, &root.join(hidden)), "{hidden}");
    }
    for visible in ["notes/note.md", ".заметки/note.md", "Проекты/.архив/note.md", ".note.md"] {
        assert!(!is_hidden(root, &root.join(visible)), "{visible}");
    }
}

#[test]
fn notes_in_dot_folders_are_listed_and_service_folders_are_not() {
    let directory = tempfile::tempdir().unwrap();
    for path in [".заметки/Идея.md", ".заметки/._Идея.md", ".obsidian/Плагин.md", ".trash/Старое.md", "Проект.md"] {
        let path = directory.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "текст").unwrap();
    }
    let mut found: Vec<String> = markdown_files(directory.path()).map(|path| relative_slash_path(directory.path(), &path)).collect();
    found.sort();
    assert_eq!(found, [".заметки/Идея.md", "Проект.md"]);
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
