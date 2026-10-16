use super::document::{hash_bytes, normalize_line_endings, read_file_snapshot_impl, write_file_atomic_impl};
use super::entries::rename_file_impl;
use super::error::FileCommandError;
use super::models::FileRenameResult;
use crate::search::wiki::LinkRewrite;
use crate::search::SearchService;
use std::path::{Path, PathBuf};

pub struct Renamed {
    pub result: FileRenameResult,
    pub rewritten: Vec<Rewritten>,
}

pub struct Rewritten {
    pub path: PathBuf,
    pub before: String,
    pub after: String,
    pub hash: String,
}

pub fn rename_with_links(
    service: &SearchService,
    old_path: &Path,
    new_path: &Path,
) -> Result<Renamed, FileCommandError> {
    if old_path.is_dir() {
        rename_file_impl(old_path, new_path)?;
        return Ok(Renamed {
            result: FileRenameResult {
                content: String::new(),
                hash: String::new(),
                text_hash: String::new(),
                updated_paths: Vec::new(),
            },
            rewritten: Vec::new(),
        });
    }
    let rewrites = service
        .plan_links_for_rename(old_path, new_path)
        .map_err(|error| FileCommandError::Task {
            message: error.to_string(),
        })?;
    apply_rename(old_path, new_path, rewrites)
}

fn apply_rename(
    old_path: &Path,
    new_path: &Path,
    rewrites: Vec<LinkRewrite>,
) -> Result<Renamed, FileCommandError> {
    for rewrite in &rewrites {
        let path = source_path(&rewrite.path, old_path, new_path, false);
        let actual = std::fs::read(path).map(|bytes| hash_bytes(&bytes))?;
        if actual != rewrite.original_hash {
            return Err(FileCommandError::Conflict {
                expected_hash: rewrite.original_hash.clone(),
                actual_hash: Some(actual),
            });
        }
    }
    rename_file_impl(old_path, new_path)?;
    let mut applied = Vec::new();
    for rewrite in rewrites {
        let path = source_path(&rewrite.path, old_path, new_path, true);
        match write_file_atomic_impl(&path, &rewrite.content, Some(&rewrite.original_hash)) {
            Ok(written) => applied.push((path, rewrite, written.hash)),
            Err(error) => {
                rollback(&applied);
                let _ = rename_file_impl(new_path, old_path);
                return Err(error);
            }
        }
    }
    let snapshot = read_file_snapshot_impl(new_path)?;
    Ok(Renamed {
        result: FileRenameResult {
            content: snapshot.content,
            hash: snapshot.hash,
            text_hash: snapshot.text_hash,
            updated_paths: applied
                .iter()
                .map(|(path, _, _)| path.to_string_lossy().into_owned())
                .collect(),
        },
        rewritten: applied
            .into_iter()
            .map(|(path, rewrite, hash)| Rewritten {
                path,
                before: normalize_line_endings(rewrite.original),
                after: rewrite.content,
                hash,
            })
            .collect(),
    })
}

fn rollback(applied: &[(PathBuf, LinkRewrite, String)]) {
    for (path, rewrite, _) in applied.iter().rev() {
        let expected = hash_bytes(rewrite.content.as_bytes());
        let _ = write_file_atomic_impl(path, &rewrite.original, Some(&expected));
    }
}

fn source_path(stored: &Path, old_path: &Path, new_path: &Path, renamed: bool) -> PathBuf {
    if renamed && crate::search::paths::same_path(stored, old_path) {
        new_path.to_path_buf()
    } else {
        stored.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_rename, hash_bytes};
    use crate::search::wiki::LinkRewrite;
    use std::fs;

    #[test]
    fn keeps_the_original_name_when_preflight_detects_a_conflict() {
        let directory = tempfile::tempdir().unwrap();
        let old_path = directory.path().join("Old.md");
        let new_path = directory.path().join("New.md");
        let source_path = directory.path().join("Source.md");
        fs::write(&old_path, "target").unwrap();
        fs::write(&source_path, "current").unwrap();
        let rewrite = LinkRewrite {
            path: source_path,
            original: "stale".to_owned(),
            original_hash: hash_bytes(b"stale"),
            content: "next".to_owned(),
        };

        assert!(apply_rename(&old_path, &new_path, vec![rewrite]).is_err());
        assert!(old_path.is_file());
        assert!(!new_path.exists());
    }

    #[test]
    fn changes_the_letter_case_and_rewrites_the_links() {
        let directory = tempfile::tempdir().unwrap();
        let old_path = directory.path().join("note.md");
        let new_path = directory.path().join("Note.md");
        let source_path = directory.path().join("Source.md");
        fs::write(&old_path, "target").unwrap();
        fs::write(&source_path, "[[note]]").unwrap();
        let rewrite = LinkRewrite {
            path: source_path.clone(),
            original: "[[note]]".to_owned(),
            original_hash: hash_bytes(b"[[note]]"),
            content: "[[Note]]".to_owned(),
        };

        apply_rename(&old_path, &new_path, vec![rewrite]).unwrap();

        assert_eq!(fs::read_to_string(&source_path).unwrap(), "[[Note]]");
        assert_eq!(fs::read_to_string(&new_path).unwrap(), "target");
        let names = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.eq_ignore_ascii_case("note.md"))
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Note.md".to_owned()]);
    }
}
