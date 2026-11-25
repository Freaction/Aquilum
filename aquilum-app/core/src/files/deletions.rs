use crate::search::paths::slash_path;
use super::error::FileCommandError;
use super::trash::{original_path, trash_path, trashed_notes};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashedDeletion {
    pub id: String,
    pub original: String,
    pub files: usize,
    pub deleted_at_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionPage {
    pub total: usize,
    pub deletions: Vec<TrashedDeletion>,
}

struct Deletion {
    root: PathBuf,
    files: usize,
}

pub fn deletion_page_impl(
    workspace: &Path,
    offset: usize,
    limit: usize,
) -> Result<DeletionPage, FileCommandError> {
    let mut deletions = deletions(workspace);
    deletions.sort_by(|(left_at, left), (right_at, right)| {
        right_at.cmp(left_at).then_with(|| crate::text::collate::compare(&left.root.to_string_lossy(), &right.root.to_string_lossy()))
    });
    Ok(DeletionPage {
        total: deletions.len(),
        deletions: deletions
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|(deleted_at_nanos, deletion)| TrashedDeletion {
                id: deleted_at_nanos.to_string(),
                original: slash_path(&deletion.root),
                files: deletion.files,
                deleted_at_ms: (deleted_at_nanos / 1_000_000) as u64,
            })
            .collect(),
    })
}

pub fn deletion_files(workspace: &Path, id: &str) -> Vec<PathBuf> {
    let Ok(id) = id.parse::<u128>() else {
        return Vec::new();
    };
    let trash = trash_path(workspace);
    trashed_notes(&trash)
        .filter(|(_, metadata)| deleted_at_nanos(metadata) == id)
        .map(|(path, _)| path)
        .collect()
}

fn deletions(workspace: &Path) -> Vec<(u128, Deletion)> {
    let trash = trash_path(workspace);
    let mut grouped = HashMap::<u128, Deletion>::new();
    for (path, metadata) in trashed_notes(&trash) {
        let original = original_path(&trash, &path);
        let deletion = grouped
            .entry(deleted_at_nanos(&metadata))
            .or_insert_with(|| Deletion { root: original.clone(), files: 0 });
        if deletion.files > 0 {
            deletion.root = shared_folder(&deletion.root, deletion.files == 1, &original);
        }
        deletion.files += 1;
    }
    grouped.into_iter().collect()
}

fn shared_folder(root: &Path, root_is_file: bool, original: &Path) -> PathBuf {
    let start = if root_is_file { root.parent() } else { Some(root) };
    start
        .into_iter()
        .flat_map(Path::ancestors)
        .find(|folder| original.starts_with(folder))
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn deleted_at_nanos(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos())
}

#[cfg(test)]
mod tests {
    use super::{deletion_files, deletion_page_impl};
    use crate::files::trash::move_to_trash_impl;
    use std::fs;

    #[test]
    fn a_deleted_folder_is_one_deletion_and_a_deleted_note_is_another() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let folder = root.join("Архив");
        fs::create_dir_all(folder.join("Глубже")).unwrap();
        fs::write(folder.join("А.md"), "а").unwrap();
        fs::write(folder.join("Глубже").join("Б.md"), "бб").unwrap();
        fs::write(root.join("В.md"), "в").unwrap();
        move_to_trash_impl(root, &root.join("В.md")).unwrap();
        move_to_trash_impl(root, &folder).unwrap();

        let page = deletion_page_impl(root, 0, 20).unwrap();

        assert_eq!(page.total, 2);
        let folder_deletion = &page.deletions[0];
        assert_eq!(folder_deletion.original, "Архив");
        assert_eq!(folder_deletion.files, 2);
        assert_eq!(page.deletions[1].original, "В.md");
        assert_eq!(page.deletions[1].files, 1);
        assert_eq!(deletion_files(root, &folder_deletion.id).len(), 2);
    }

    #[test]
    fn a_folder_merged_into_the_trash_stays_apart_from_what_was_there() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let folder = root.join("Архив");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("Раньше.md"), "1").unwrap();
        move_to_trash_impl(root, &folder.join("Раньше.md")).unwrap();
        fs::write(folder.join("А.md"), "2").unwrap();
        fs::write(folder.join("Б.md"), "3").unwrap();
        move_to_trash_impl(root, &folder).unwrap();

        let page = deletion_page_impl(root, 0, 20).unwrap();

        assert_eq!(page.total, 2, "слияние с корзиной не склеивает удаления");
        assert_eq!(page.deletions[0].files, 2);
        assert_eq!(page.deletions[1].original, "Архив/Раньше.md");
    }
}
