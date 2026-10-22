use super::IndexSynchronizer;
use crate::search::index::SearchIndex;
use crate::search::progress::IndexProgress;
use crate::search::query::Request;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn open(directory: &Path) -> (PathBuf, Arc<SearchIndex>, IndexSynchronizer) {
    let vault = crate::search::paths::canonical_path(directory).join("vault");
    fs::create_dir_all(&vault).unwrap();
    let index = Arc::new(SearchIndex::open(&directory.join("index")).unwrap());
    let synchronizer = IndexSynchronizer::new(vault.clone(), Arc::clone(&index), &directory.join("documents.sqlite3"), Arc::new(IndexProgress::new(0, 1))).unwrap();
    (vault, index, synchronizer)
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn the_writer_lives_only_while_there_is_something_to_write() {
    let directory = tempfile::tempdir().unwrap();
    let (vault, index, mut synchronizer) = open(directory.path());
    let note = vault.join("Заметка.md");
    write(&note, "первое слово");
    let cancelled = AtomicBool::new(false);
    assert!(!synchronizer.holds_writer(), "открытие не создаёт писателя");

    synchronizer.full_scan(&cancelled).unwrap();
    assert!(!synchronizer.holds_writer(), "писатель не отпущен после записи");
    drop(index.index.writer::<tantivy::TantivyDocument>(15_000_000).expect("блокировка индекса отпущена"));

    synchronizer.full_scan(&cancelled).unwrap();
    assert!(!synchronizer.holds_writer(), "неизменённая база писателя не создаёт");

    write(&note, "второе слово");
    synchronizer.apply_paths(HashSet::from([note])).unwrap();
    assert!(!synchronizer.holds_writer(), "писатель не отпущен после правки");
    assert!(!index.search(&Request::new("второе")).unwrap().results.is_empty(), "правка видна поиску сразу");
}

#[test]
fn notes_in_dot_folders_are_searchable_and_service_folders_are_not() {
    let directory = tempfile::tempdir().unwrap();
    let (vault, index, mut synchronizer) = open(directory.path());
    write(&vault.join(".заметки/Идея.md"), "точечная");
    write(&vault.join(".obsidian/Плагин.md"), "плагинное");
    synchronizer.full_scan(&AtomicBool::new(false)).unwrap();

    let fresh = vault.join(".заметки/Новая.md");
    write(&fresh, "свежая");
    synchronizer.apply_paths(HashSet::from([fresh, vault.join(".obsidian/Плагин.md")])).unwrap();

    let found = |word: &str| index.search(&Request::new(word)).unwrap().results.len();
    assert_eq!((found("точечная"), found("свежая"), found("плагинное")), (1, 1, 0));
}

#[test]
fn a_folder_path_indexes_or_forgets_everything_under_it() {
    let directory = tempfile::tempdir().unwrap();
    let (vault, index, mut synchronizer) = open(directory.path());
    let folder = vault.join("2024.10");
    write(&folder.join("Встреча.md"), "перенесённая");
    write(&folder.join("Вложенная").join("Итоги.md"), "вложенная");
    write(&vault.join("2024.10 итог.md"), "соседняя");
    synchronizer.full_scan(&AtomicBool::new(false)).unwrap();
    let found = |word: &str| index.search(&Request::new(word)).unwrap().results.len();
    fs::remove_dir_all(&folder).unwrap();
    write(&vault.join("Архив").join("Встреча.md"), "переехала");

    assert!(synchronizer.apply_paths(HashSet::from([folder.clone(), vault.join("Архив")])).unwrap());

    assert_eq!((found("перенесённая"), found("вложенная"), found("соседняя"), found("переехала")), (0, 0, 1, 1));
    assert!(!synchronizer.apply_paths(HashSet::from([folder])).unwrap(), "повторное удаление не трогает индекс");
}
