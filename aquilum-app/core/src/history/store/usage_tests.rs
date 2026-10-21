use super::{clear, usage};
use crate::history::store::{history_dir, history_root};
use std::fs;
use std::path::{Path, PathBuf};

fn history_with_trash(vault: &Path) -> (PathBuf, PathBuf) {
    let note = history_dir(vault, Path::new("Идея.md"));
    let trashed = history_dir(&vault.join(".trash"), Path::new("Старое.md"));
    fs::create_dir_all(&note).unwrap();
    fs::create_dir_all(&trashed).unwrap();
    (note, trashed)
}

#[test]
fn usage_counts_versions_of_notes_but_not_names_nor_the_history_in_the_trash() {
    let vault = tempfile::tempdir().unwrap();
    let (note, trashed) = history_with_trash(vault.path());
    fs::write(note.join("1_dev_me.md"), "один").unwrap();
    fs::write(note.join("2_dev_me.delta"), "дельта").unwrap();
    fs::write(note.join("2_dev_me.delta.0000000000005_dev.name"), "имя").unwrap();
    fs::write(trashed.join("3_dev_me.md"), "три").unwrap();

    let usage = usage(vault.path());

    assert_eq!((usage.versions, usage.bytes), (2, 26));
}

#[test]
fn clearing_history_removes_every_version_and_leaves_the_trash_alone() {
    let vault = tempfile::tempdir().unwrap();
    let (note, trashed) = history_with_trash(vault.path());
    fs::write(vault.path().join("Идея.md"), "заметка").unwrap();
    fs::write(vault.path().join(".trash").join("Старое.md"), "удалённое").unwrap();
    fs::write(note.join("1_dev_me.md"), "один").unwrap();
    fs::write(trashed.join("3_dev_me.md"), "три").unwrap();

    clear(vault.path()).unwrap();
    clear(vault.path()).unwrap();

    assert_eq!(usage(vault.path()).versions, 0);
    assert!(!history_root(vault.path()).exists());
    assert_eq!(fs::read_to_string(vault.path().join("Идея.md")).unwrap(), "заметка");
    assert!(vault.path().join(".trash").join("Старое.md").exists());
    assert!(trashed.join("3_dev_me.md").exists());
}
