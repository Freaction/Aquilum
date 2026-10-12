use super::plan_for_rename;
use crate::search::wiki::{index_document, open_schema};
use rusqlite::Connection;
use std::fs;

#[test]
fn preserves_aliases_when_a_target_is_renamed() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let old_path = root.join("Old.md");
    let new_path = root.join("New.md");
    let source_path = root.join("Source.md");
    fs::write(&old_path, "target").unwrap();
    fs::write(&source_path, "[[Old]] and [[Old|visible]]").unwrap();
    let mut connection = Connection::open_in_memory().unwrap();
    open_schema(&connection).unwrap();
    let transaction = connection.transaction().unwrap();
    index_document(&transaction, root, &old_path, "target").unwrap();
    index_document(
        &transaction,
        root,
        &source_path,
        "[[Old]] and [[Old|visible]]",
    )
    .unwrap();
    transaction.commit().unwrap();

    let plan = plan_for_rename(
        &connection,
        root,
        &old_path,
        &new_path,
        vec![source_path],
        None,
    )
    .unwrap();

    assert_eq!(plan[0].content, "[[New]] and [[New|visible]]");
}

#[test]
fn rename_keeps_anchors_labels_and_ignores_attachments() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let old_path = root.join("Old.md");
    let new_path = root.join("New.md");
    let source_path = root.join("Source.md");
    let text = "[[Old#Раздел]] [[Old#^abc|метка]] [[Old | пробелы ]] ![[Old.png|200]] [[#Раздел]]";
    fs::write(&old_path, "target").unwrap();
    fs::write(&source_path, text).unwrap();
    let mut connection = Connection::open_in_memory().unwrap();
    open_schema(&connection).unwrap();
    let transaction = connection.transaction().unwrap();
    index_document(&transaction, root, &old_path, "target").unwrap();
    index_document(&transaction, root, &source_path, text).unwrap();
    transaction.commit().unwrap();

    let plan = plan_for_rename(&connection, root, &old_path, &new_path, vec![source_path], None)
        .unwrap();

    assert_eq!(
        plan[0].content,
        "[[New#Раздел]] [[New#^abc|метка]] [[New | пробелы ]] ![[Old.png|200]] [[#Раздел]]"
    );
}

#[test]
fn rewrites_links_in_a_crlf_note_and_keeps_the_hash_of_its_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let old_path = root.join("Old.md");
    let new_path = root.join("New.md");
    let source_path = root.join("Source.md");
    let raw = "# Title\r\n\r\nsee [[Old]]\r\n";
    fs::write(&old_path, "target").unwrap();
    fs::write(&source_path, raw).unwrap();
    let mut connection = Connection::open_in_memory().unwrap();
    open_schema(&connection).unwrap();
    let transaction = connection.transaction().unwrap();
    index_document(&transaction, root, &old_path, "target").unwrap();
    index_document(&transaction, root, &source_path, "# Title\n\nsee [[Old]]\n").unwrap();
    transaction.commit().unwrap();

    let plan = plan_for_rename(&connection, root, &old_path, &new_path, vec![source_path], None)
        .unwrap();

    assert_eq!(plan[0].content, "# Title\n\nsee [[New]]\n");
    assert_eq!(plan[0].original, raw);
    assert_eq!(plan[0].original_hash, blake3::hash(raw.as_bytes()).to_hex().to_string());
}
