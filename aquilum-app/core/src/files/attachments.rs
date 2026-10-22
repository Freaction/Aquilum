use crate::search::paths::{is_hidden_name, is_visible_entry, slash_path};
use notify::event::ModifyKind;
use notify::{Event, EventKind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use unicode_normalization::UnicodeNormalization;
use walkdir::WalkDir;

pub const MEDIA_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif", "ico", "mp4", "webm", "ogv", "ogg",
    "mov", "m4v", "mkv",
];

static INDEX: Mutex<Option<(PathBuf, HashMap<String, String>)>> = Mutex::new(None);

fn is_media(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            MEDIA_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

fn scan(root: &Path) -> HashMap<String, String> {
    let mut index = HashMap::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(is_visible_entry)
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() || !is_media(entry.path()) {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        let relative = slash_path(relative);
        let name = key(&entry.file_name().to_string_lossy());
        index.entry(key(&relative)).or_insert_with(|| relative.clone());
        index.entry(name).or_insert(relative);
    }
    index
}

fn key(path: &str) -> String {
    path.trim()
        .replace(['\u{a0}', '\u{202f}'], " ")
        .nfc()
        .collect::<String>()
        .to_lowercase()
}

fn index() -> MutexGuard<'static, Option<(PathBuf, HashMap<String, String>)>> {
    INDEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn forget_attachments() {
    *index() = None;
}

pub fn forget_attachments_if_touched(paths: &[PathBuf]) {
    if paths.iter().any(|path| may_hold_attachments(path)) {
        forget_attachments();
    }
}

pub fn forget_attachments_after(result: &notify::Result<Event>) {
    match result {
        Ok(event) if is_structural(&event.kind) => forget_attachments_if_touched(&event.paths),
        Ok(_) => {}
        Err(_) => forget_attachments(),
    }
}

fn is_structural(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
    )
}

fn may_hold_attachments(path: &Path) -> bool {
    !path.file_name().is_some_and(is_hidden_name) && (is_media(path) || path.extension().is_none() || path.is_dir())
}

pub fn resolve_attachments_impl(root: &Path, names: Vec<String>) -> Vec<Option<String>> {
    let mut guard = index();
    if !matches!(guard.as_ref(), Some((cached, _)) if cached == root) {
        *guard = Some((root.to_path_buf(), scan(root)));
    }
    let Some((_, found)) = guard.as_ref() else {
        return Vec::new();
    };
    names
        .iter()
        .map(|name| found.get(&key(name)).cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{forget_attachments_if_touched, resolve_attachments_impl};
    use std::fs;

    #[test]
    fn finds_an_attachment_by_bare_name_and_by_relative_path() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir(root.join("Files")).unwrap();
        fs::write(root.join("Files").join("Pasted image 1.png"), b"x").unwrap();
        fs::write(root.join("Files").join("clip.mp4"), b"x").unwrap();
        fs::write(root.join("Note.md"), b"x").unwrap();

        let resolved = resolve_attachments_impl(
            root,
            vec![
                "Pasted image 1.png".to_owned(),
                "Files/clip.mp4".to_owned(),
                "Note.md".to_owned(),
            ],
        );

        assert_eq!(resolved[0].as_deref(), Some("Files/Pasted image 1.png"));
        assert_eq!(resolved[1].as_deref(), Some("Files/clip.mp4"));
        assert_eq!(resolved[2], None);
    }

    #[test]
    fn a_new_attachment_is_found_after_the_cache_is_told_about_it() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::write(root.join("Note.md"), b"x").unwrap();
        assert_eq!(resolve_attachments_impl(root, vec!["new.png".to_owned()]), vec![None]);

        let added = root.join("new.png");
        fs::write(&added, b"x").unwrap();
        forget_attachments_if_touched(&[root.join("Note.md")]);
        forget_attachments_if_touched(&[added]);

        assert_eq!(
            resolve_attachments_impl(root, vec!["new.png".to_owned()]),
            vec![Some("new.png".to_owned())]
        );
    }

    const LINK: &str = "вложенные файлы/7cf4fd6066217977f20d9bb20c5d494b_MD5.jpg";

    fn vault_with(folder: &str) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join(folder);
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("7cf4fd6066217977f20d9bb20c5d494b_MD5.jpg"), b"x").unwrap();
        directory
    }

    #[test]
    fn a_path_with_a_space_in_the_folder_is_found() {
        let vault = vault_with("вложенные файлы");
        assert_eq!(resolve_attachments_impl(vault.path(), vec![LINK.to_owned()]), vec![Some(LINK.to_owned())]);
    }

    #[test]
    fn a_path_is_found_whatever_the_unicode_form_and_the_kind_of_space() {
        use super::UnicodeNormalization;
        let decomposed: String = "вложенные файлы".nfd().collect();
        let vault = vault_with(&decomposed);
        let found = resolve_attachments_impl(vault.path(), vec![LINK.to_owned(), LINK.replace(' ', "\u{a0}")]);
        assert!(found.iter().all(Option::is_some), "{found:?}");
        assert!(vault.path().join(found[0].as_deref().unwrap()).is_file());
    }
}
