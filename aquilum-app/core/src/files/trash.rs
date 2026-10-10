use crate::search::paths::slash_path;
use super::error::FileCommandError;
use super::relocate::{
    relocate, remove_empty_ancestors, remove_empty_folders, remove_files, Placement, Relocated,
};
use crate::history::AQUILUM_FOLDER;
use crate::search::paths::canonical_path;
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub const TRASH_FOLDER: &str = ".trash";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashState {
    pub path: String,
    pub count: u64,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashedFile {
    pub path: String,
    pub original: String,
    pub deleted_at_ms: u64,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashPage {
    pub total: usize,
    pub files: Vec<TrashedFile>,
}

pub fn trash_path(workspace: &Path) -> PathBuf {
    workspace.join(TRASH_FOLDER)
}

pub fn trash_state_impl(workspace: &Path) -> Result<TrashState, FileCommandError> {
    let path = trash_path(workspace);
    let (count, bytes) = trashed_notes(&path)
        .fold((0, 0), |(count, bytes), (_, metadata)| (count + 1, bytes + metadata.len()));
    Ok(TrashState {
        path: path.to_string_lossy().into_owned(),
        count,
        bytes,
    })
}

pub fn cleanup_trash_impl(
    workspace: &Path,
    retention_days: u32,
) -> Result<u64, FileCommandError> {
    let trash = trash_path(workspace);
    if retention_days == 0 || !trash.is_dir() {
        return Ok(0);
    }
    let lifetime = Duration::from_secs(u64::from(retention_days) * 24 * 60 * 60);
    let now = SystemTime::now();
    let expired = trashed_files(&trash)
        .filter(|(_, metadata)| {
            metadata
                .modified()
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age > lifetime)
        })
        .map(|(path, _)| path)
        .collect::<Vec<_>>();
    let removed = remove_files(&expired);
    remove_empty_folders(&trash);
    Ok(removed)
}

pub fn empty_trash_impl(workspace: &Path) -> Result<u64, FileCommandError> {
    let trash = trash_path(workspace);
    if !trash.is_dir() {
        return Ok(0);
    }
    let files = trashed_files(&trash).map(|(path, _)| path).collect::<Vec<_>>();
    let removed = remove_files(&files);
    remove_empty_folders(&trash);
    Ok(removed)
}

pub fn list_trash_impl(workspace: &Path) -> Result<Vec<TrashedFile>, FileCommandError> {
    let trash = trash_path(workspace);
    let mut files = trashed_notes(&trash)
        .map(|(path, metadata)| TrashedFile {
            original: slash_path(&original_path(&trash, &path)),
            deleted_at_ms: metadata
                .modified()
                .ok()
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |since| since.as_millis() as u64),
            bytes: metadata.len(),
            path: path.to_string_lossy().into_owned(),
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| {
        right
            .deleted_at_ms
            .cmp(&left.deleted_at_ms)
            .then_with(|| left.original.cmp(&right.original))
    });
    Ok(files)
}

pub fn trash_page_impl(
    workspace: &Path,
    offset: usize,
    limit: usize,
) -> Result<TrashPage, FileCommandError> {
    let files = list_trash_impl(workspace)?;
    Ok(TrashPage {
        total: files.len(),
        files: files.into_iter().skip(offset).take(limit).collect(),
    })
}

pub fn restore_from_trash_impl(
    workspace: &Path,
    trashed: &Path,
) -> Result<Relocated, FileCommandError> {
    let trash = canonical_path(&trash_path(workspace));
    let trashed = canonical_path(trashed);
    let relative = Some(original_path(&trash, &trashed))
        .filter(|relative| trashed.starts_with(&trash) && !relative.as_os_str().is_empty())
        .ok_or_else(|| FileCommandError::Io {
            message: format!("не в корзине: {}", trashed.display()),
        })?;
    if !trashed.exists() {
        return Err(FileCommandError::Io {
            message: format!("в корзине нет {}", relative.display()),
        });
    }
    let restored = relocate(&trashed, &canonical_path(workspace).join(relative), Placement::Workspace)?;
    remove_empty_ancestors(&trashed, &trash);
    Ok(restored)
}

pub fn move_to_trash_impl(
    workspace: &Path,
    entry: &Path,
) -> Result<Relocated, FileCommandError> {
    let target = trash_path(workspace).join(place_in_trash(workspace, entry));
    relocate(entry, &target, Placement::Trash)
}

pub fn ensure_trash_impl(workspace: &Path) -> Result<String, FileCommandError> {
    let path = trash_path(workspace);
    fs::create_dir_all(&path)?;
    Ok(path.to_string_lossy().into_owned())
}

fn place_in_trash(workspace: &Path, entry: &Path) -> PathBuf {
    canonical_path(entry)
        .strip_prefix(canonical_path(workspace))
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(|| entry.file_name().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("Заметка"))
}

pub fn original_path(trash: &Path, trashed: &Path) -> PathBuf {
    trashed
        .strip_prefix(trash)
        .map(|relative| {
            relative
                .components()
                .filter(|component| !is_copy_marker(component))
                .collect()
        })
        .unwrap_or_default()
}

fn is_copy_marker(component: &Component<'_>) -> bool {
    let Component::Normal(name) = component else {
        return false;
    };
    let name = name.to_string_lossy();
    name.strip_prefix('.')
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
}

pub fn trashed_notes(trash: &Path) -> impl Iterator<Item = (PathBuf, fs::Metadata)> + '_ {
    trashed_files(trash).filter(move |(path, _)| {
        let original = original_path(trash, path);
        !original.starts_with(AQUILUM_FOLDER)
    })
}

fn trashed_files(trash: &Path) -> impl Iterator<Item = (PathBuf, fs::Metadata)> {
    WalkDir::new(trash)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok().map(|metadata| (entry.into_path(), metadata)))
}

#[cfg(test)]
mod tests {
    use super::{
        cleanup_trash_impl, empty_trash_impl, ensure_trash_impl, list_trash_impl, move_to_trash_impl,
        restore_from_trash_impl, trash_page_impl, trash_path, trash_state_impl,
    };
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, SystemTime};
    use walkdir::WalkDir;

    fn aged_file(path: &Path, days: u64) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "мусор").unwrap();
        age(path, days);
    }

    fn age(path: &Path, days: u64) {
        let when = SystemTime::now() - Duration::from_secs(days * 24 * 60 * 60);
        let file = fs::File::options().write(true).open(path).unwrap();
        file.set_modified(when).unwrap();
    }

    fn workspace() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn removes_only_files_older_than_retention() {
        let directory = workspace();
        ensure_trash_impl(directory.path()).unwrap();
        let trash = trash_path(directory.path());
        aged_file(&trash.join("Старая.md"), 40);
        aged_file(&trash.join("Свежая.md"), 2);
        aged_file(&trash.join("Проекты").join("Вложенная старая.md"), 40);

        assert_eq!(cleanup_trash_impl(directory.path(), 30).unwrap(), 2);
        assert!(!trash.join("Старая.md").exists());
        assert!(trash.join("Свежая.md").exists());
        assert!(!trash.join("Проекты").exists(), "опустевшая папка убрана");
    }

    #[test]
    fn counts_retention_from_the_moment_of_deletion() {
        let directory = workspace();
        let note = directory.path().join("Древняя.md");
        aged_file(&note, 900);

        let moved = move_to_trash_impl(directory.path(), &note).unwrap().root;

        assert!(moved.exists());
        assert_eq!(cleanup_trash_impl(directory.path(), 30).unwrap(), 0);
    }

    #[test]
    fn keeps_everything_when_retention_is_disabled() {
        let directory = workspace();
        ensure_trash_impl(directory.path()).unwrap();
        aged_file(&trash_path(directory.path()).join("Древняя.md"), 900);

        assert_eq!(cleanup_trash_impl(directory.path(), 0).unwrap(), 0);
        assert_eq!(trash_state_impl(directory.path()).unwrap().count, 1);
    }

    #[test]
    fn reports_empty_state_without_a_trash_folder() {
        let directory = workspace();
        let state = trash_state_impl(directory.path()).unwrap();
        assert_eq!(state.count, 0);
        assert_eq!(state.bytes, 0);
    }

    #[test]
    fn a_note_keeps_its_folder_in_the_trash() {
        let directory = workspace();
        let note = directory.path().join("Проекты").join("Идея.md");
        aged_file(&note, 1);

        let moved = move_to_trash_impl(directory.path(), &note).unwrap().root;

        assert_eq!(
            moved,
            trash_path(directory.path()).join("Проекты").join("Идея.md"),
            "по месту в корзине виден исходный путь"
        );
    }

    #[test]
    fn a_folder_moves_to_the_trash_with_everything_inside() {
        let directory = workspace();
        let folder = directory.path().join("Проекты");
        fs::create_dir_all(folder.join("Глубже")).unwrap();
        fs::write(folder.join("Заметка.md"), "текст").unwrap();
        fs::write(folder.join("Глубже").join("Вторая.md"), "ещё").unwrap();

        let moved = move_to_trash_impl(directory.path(), &folder).unwrap().root;

        assert!(!folder.exists());
        assert_eq!(moved, trash_path(directory.path()).join("Проекты"));
        assert_eq!(fs::read_to_string(moved.join("Заметка.md")).unwrap(), "текст");
        assert_eq!(
            fs::read_to_string(moved.join("Глубже").join("Вторая.md")).unwrap(),
            "ещё"
        );
        assert_eq!(trash_state_impl(directory.path()).unwrap().count, 2, "считаются и вложенные файлы");
    }

    #[test]
    fn a_deleted_folder_ages_from_the_deletion_and_is_cleaned_as_a_whole() {
        let directory = workspace();
        let folder = directory.path().join("Архив");
        aged_file(&folder.join("Старая.md"), 900);
        aged_file(&folder.join("Глубже").join("Ещё старее.md"), 900);

        let moved = move_to_trash_impl(directory.path(), &folder).unwrap().root;
        assert_eq!(
            cleanup_trash_impl(directory.path(), 30).unwrap(),
            0,
            "срок считается от удаления, а не от последней правки файлов"
        );

        for entry in WalkDir::new(&moved).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                age(entry.path(), 40);
            }
        }
        assert_eq!(cleanup_trash_impl(directory.path(), 30).unwrap(), 2);
        assert!(!moved.exists(), "папка уходит целиком, вместе с вложенными");
        assert!(trash_path(directory.path()).exists(), "сама корзина остаётся");
    }

    #[test]
    fn deleting_the_same_path_twice_keeps_both_copies() {
        let directory = workspace();
        let note = directory.path().join("Идея.md");
        fs::write(&note, "первая").unwrap();
        let first = move_to_trash_impl(directory.path(), &note).unwrap().root;
        fs::write(&note, "вторая").unwrap();
        let second = move_to_trash_impl(directory.path(), &note).unwrap().root;

        assert_eq!(fs::read_to_string(&first).unwrap(), "первая");
        assert_eq!(second, trash_path(directory.path()).join(".1").join("Идея.md"));
        assert_eq!(fs::read_to_string(&second).unwrap(), "вторая");
        let originals = list_trash_impl(directory.path())
            .unwrap()
            .into_iter()
            .map(|item| item.original)
            .collect::<Vec<_>>();
        assert_eq!(originals, ["Идея.md", "Идея.md"], "обе копии помнят один прежний путь");

        let restored = restore_from_trash_impl(directory.path(), &second).unwrap().root;
        assert_eq!(restored.file_name().unwrap(), "Идея.md", "копия возвращается под своим именем");
        assert!(!trash_path(directory.path()).join(".1").exists());
    }

    #[test]
    fn a_folder_merges_with_what_is_already_in_the_trash() {
        let directory = workspace();
        let folder = directory.path().join("Проекты");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("А.md"), "раньше").unwrap();
        move_to_trash_impl(directory.path(), &folder.join("А.md")).unwrap();
        fs::write(folder.join("Б.md"), "позже").unwrap();
        fs::write(folder.join("А.md"), "новая А").unwrap();

        let moved = move_to_trash_impl(directory.path(), &folder).unwrap().root;

        assert!(!folder.exists());
        assert_eq!(fs::read_to_string(moved.join("А.md")).unwrap(), "раньше");
        assert_eq!(fs::read_to_string(moved.join(".1").join("А.md")).unwrap(), "новая А");
        assert_eq!(fs::read_to_string(moved.join("Б.md")).unwrap(), "позже");
    }

    #[test]
    fn a_note_comes_back_to_where_it_was() {
        let directory = workspace();
        let note = directory.path().join("Проекты").join("Идея.md");
        aged_file(&note, 1);
        let trashed = move_to_trash_impl(directory.path(), &note).unwrap().root;

        let restored = restore_from_trash_impl(directory.path(), &trashed).unwrap().root;

        let root = crate::search::paths::canonical_path(directory.path());
        assert_eq!(restored, root.join("Проекты").join("Идея.md"));
        assert_eq!(fs::read_to_string(&restored).unwrap(), "мусор");
        assert!(!trash_path(directory.path()).join("Проекты").exists(), "опустевшая папка в корзине убрана");
    }

    #[test]
    fn a_restored_note_does_not_overwrite_a_new_one_at_its_place() {
        let directory = workspace();
        let note = directory.path().join("Идея.md");
        fs::write(&note, "старая").unwrap();
        let trashed = move_to_trash_impl(directory.path(), &note).unwrap().root;
        fs::write(&note, "новая").unwrap();

        let restored = restore_from_trash_impl(directory.path(), &trashed).unwrap().root;

        assert_eq!(fs::read_to_string(&note).unwrap(), "новая");
        assert_eq!(restored.file_name().unwrap(), "Идея (1).md");
        assert_eq!(fs::read_to_string(&restored).unwrap(), "старая");
    }

    #[test]
    fn restore_refuses_paths_outside_the_trash() {
        let directory = workspace();
        let note = directory.path().join("Живая.md");
        fs::write(&note, "текст").unwrap();
        assert!(restore_from_trash_impl(directory.path(), &note).is_err());
        let missing = trash_path(directory.path()).join("Нет.md");
        assert!(restore_from_trash_impl(directory.path(), &missing).is_err());
    }

    #[test]
    fn the_list_names_original_paths_newest_first() {
        let directory = workspace();
        let older = directory.path().join("Старое").join("А.md");
        let newer = directory.path().join("Б.md");
        aged_file(&older, 1);
        aged_file(&newer, 1);
        let first = move_to_trash_impl(directory.path(), &older).unwrap().root;
        age(&first, 3);
        move_to_trash_impl(directory.path(), &newer).unwrap();

        let list = list_trash_impl(directory.path()).unwrap();

        let originals = list.iter().map(|item| item.original.as_str()).collect::<Vec<_>>();
        assert_eq!(originals, ["Б.md", "Старое/А.md"]);
        assert!(list[0].deleted_at_ms > list[1].deleted_at_ms);
        assert_eq!(list[0].bytes, "мусор".len() as u64);
    }

    #[test]
    fn a_page_of_the_trash_keeps_the_total() {
        let directory = workspace();
        for name in ["А.md", "Б.md", "В.md"] {
            let note = directory.path().join(name);
            aged_file(&note, 1);
            move_to_trash_impl(directory.path(), &note).unwrap();
        }

        let page = trash_page_impl(directory.path(), 1, 1).unwrap();

        assert_eq!(page.total, 3);
        assert_eq!(page.files.len(), 1);
        assert_eq!(page.files[0].original, list_trash_impl(directory.path()).unwrap()[1].original);
        assert!(trash_page_impl(directory.path(), 5, 10).unwrap().files.is_empty());
    }

    #[test]
    fn emptying_the_trash_removes_everything_but_the_trash_itself() {
        let directory = workspace();
        let folder = directory.path().join("Архив");
        aged_file(&folder.join("А.md"), 1);
        aged_file(&folder.join("Глубже").join("Б.md"), 1);
        move_to_trash_impl(directory.path(), &folder).unwrap();

        assert_eq!(empty_trash_impl(directory.path()).unwrap(), 2);
        assert_eq!(trash_state_impl(directory.path()).unwrap().count, 0);
        assert!(trash_path(directory.path()).exists());
        assert_eq!(fs::read_dir(trash_path(directory.path())).unwrap().count(), 0);
    }

    #[test]
    fn the_move_reports_where_every_file_landed() {
        let directory = workspace();
        let folder = directory.path().join("Проекты");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("А.md"), "раньше").unwrap();
        move_to_trash_impl(directory.path(), &folder.join("А.md")).unwrap();
        fs::write(folder.join("А.md"), "позже").unwrap();
        fs::write(folder.join("Б.md"), "ещё").unwrap();

        let moved = move_to_trash_impl(directory.path(), &folder).unwrap();

        let trash = trash_path(directory.path()).join("Проекты");
        let mut files = moved.files.clone();
        files.sort();
        assert_eq!(
            files,
            vec![
                (folder.join("А.md"), trash.join(".1").join("А.md")),
                (folder.join("Б.md"), trash.join("Б.md")),
            ],
            "совпавший путь уходит в различитель, и операция сама называет это место"
        );

        let trashed = crate::search::paths::canonical_path(&trash.join(".1").join("А.md"));
        let restored = restore_from_trash_impl(directory.path(), &trashed).unwrap();
        assert_eq!(
            restored.files,
            vec![(trashed, restored.root.clone())]
        );
    }
}
