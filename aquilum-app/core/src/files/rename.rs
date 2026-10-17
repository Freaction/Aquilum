use super::document::hash_file;
use super::entries::rename_file_impl;
use super::error::FileCommandError;
use crate::search::paths::same_path;
use crate::search::wiki::LinkRewrite;
use crate::search::SearchService;
use std::path::Path;

pub fn rename_with_links(service: &SearchService, old_path: &Path, new_path: &Path) -> Result<Vec<LinkRewrite>, FileCommandError> {
    let rewrites = match old_path.is_dir() {
        true => Vec::new(),
        false => service.plan_links_for_rename(old_path, new_path).map_err(|error| FileCommandError::Task { message: error.to_string() })?,
    };
    renamed(old_path, new_path, rewrites)
}

fn renamed(old_path: &Path, new_path: &Path, rewrites: Vec<LinkRewrite>) -> Result<Vec<LinkRewrite>, FileCommandError> {
    for rewrite in &rewrites {
        let actual = hash_file(&rewrite.path)?;
        if actual != rewrite.original_hash {
            return Err(FileCommandError::Conflict { expected_hash: rewrite.original_hash.clone(), actual_hash: Some(actual) });
        }
    }
    rename_file_impl(old_path, new_path)?;
    Ok(rewrites
        .into_iter()
        .map(|rewrite| match same_path(&rewrite.path, old_path) {
            true => LinkRewrite { path: new_path.to_path_buf(), ..rewrite },
            false => rewrite,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::renamed;
    use crate::files::document::hash_bytes;
    use crate::search::wiki::LinkRewrite;
    use std::fs;

    #[test]
    fn keeps_the_original_name_when_preflight_detects_a_conflict() {
        let directory = tempfile::tempdir().unwrap();
        let (old_path, new_path, source_path) = (directory.path().join("Old.md"), directory.path().join("New.md"), directory.path().join("Source.md"));
        fs::write(&old_path, "target").unwrap();
        fs::write(&source_path, "current").unwrap();
        let rewrite = LinkRewrite { path: source_path, original_hash: hash_bytes(b"stale"), changes: vec![(0..1, "x".to_owned())] };

        assert!(renamed(&old_path, &new_path, vec![rewrite]).is_err());
        assert!(old_path.is_file());
        assert!(!new_path.exists());
    }

    #[test]
    fn a_note_linking_to_itself_is_rewritten_at_its_new_path() {
        let directory = tempfile::tempdir().unwrap();
        let (old_path, new_path) = (directory.path().join("note.md"), directory.path().join("Note.md"));
        fs::write(&old_path, "[[note]]").unwrap();
        let rewrite = LinkRewrite { path: old_path.clone(), original_hash: hash_bytes(b"[[note]]"), changes: vec![(2..6, "Note".to_owned())] };

        let planned = renamed(&old_path, &new_path, vec![rewrite]).unwrap();

        assert_eq!(planned[0].path, new_path);
        let names = fs::read_dir(directory.path()).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect::<Vec<_>>();
        assert_eq!(names, vec!["Note.md".to_owned()]);
    }
}
