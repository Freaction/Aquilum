use super::IndexSynchronizer;
use crate::search::index::SearchIndex;
use crate::search::progress::IndexProgress;
use std::collections::HashSet;
use std::fs;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[test]
fn the_writer_lives_only_while_there_is_something_to_write() {
    let directory = tempfile::tempdir().unwrap();
    let vault = crate::search::paths::canonical_path(directory.path()).join("vault");
    fs::create_dir_all(&vault).unwrap();
    let note = vault.join("Заметка.md");
    fs::write(&note, "первое слово").unwrap();
    let index = Arc::new(SearchIndex::open(&directory.path().join("index")).unwrap());
    let mut synchronizer = IndexSynchronizer::new(
        vault.clone(),
        Arc::clone(&index),
        &directory.path().join("documents.sqlite3"),
        Arc::new(IndexProgress::new(0, 1)),
    )
    .unwrap();
    let cancelled = AtomicBool::new(false);
    assert!(!synchronizer.holds_writer(), "открытие не создаёт писателя");

    synchronizer.full_scan(&cancelled).unwrap();
    assert!(synchronizer.holds_writer(), "новая заметка записана писателем");
    synchronizer.release_writer().unwrap();
    assert!(!synchronizer.holds_writer());
    drop(index.index.writer::<tantivy::TantivyDocument>(15_000_000).expect("блокировка индекса отпущена"));

    synchronizer.full_scan(&cancelled).unwrap();
    assert!(!synchronizer.holds_writer(), "неизменённая база писателя не создаёт");

    fs::write(&note, "второе слово").unwrap();
    synchronizer.apply_paths(HashSet::from([note])).unwrap();
    assert!(synchronizer.holds_writer());
    assert!(!index.search(&crate::search::query::Request::new("второе")).unwrap().results.is_empty(), "правка видна поиску сразу");
}
