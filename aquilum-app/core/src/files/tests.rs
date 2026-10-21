use super::document::{hash_bytes, hash_file, normalize_line_endings, read_text, write_content_atomic};
use super::entries::{create_content_file, rename_file_impl};
use super::error::FileCommandError;
use super::models::FileItemType;
use super::workspace::read_directory_impl;
use std::fs;

#[test]
fn atomic_write_replaces_the_complete_file() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("note.md");
    fs::write(&path, "before").expect("fixture");

    let result = write_content_atomic(&path, &"after", None).expect("write");

    assert_eq!(fs::read_to_string(&path).expect("result"), "after");
    assert_eq!(result.hash, hash_bytes(b"after"));
}

#[test]
fn expected_hash_prevents_overwrite() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("note.md");
    fs::write(&path, "external change").expect("fixture");

    let error = write_content_atomic(&path, &"aquilum change", Some("stale-hash"))
        .expect_err("conflict expected");

    assert!(matches!(error, FileCommandError::Conflict { .. }));
    assert_eq!(
        fs::read_to_string(&path).expect("unchanged file"),
        "external change"
    );
}

#[test]
fn create_file_never_overwrites_an_existing_note() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("note.md");
    fs::write(&path, "keep me").expect("fixture");

    let error = create_content_file(&path, &"replacement").expect_err("already exists");

    assert!(matches!(error, FileCommandError::AlreadyExists { .. }));
    assert_eq!(fs::read_to_string(&path).expect("result"), "keep me");
}

#[test]
fn rename_file_never_overwrites_an_existing_note() {
    let directory = tempfile::tempdir().expect("temp directory");
    let old_path = directory.path().join("old.md");
    let new_path = directory.path().join("new.md");
    fs::write(&old_path, "old content").expect("old fixture");
    fs::write(&new_path, "new content").expect("new fixture");

    let error = rename_file_impl(&old_path, &new_path).expect_err("already exists");

    assert!(matches!(error, FileCommandError::AlreadyExists { .. }));
    assert_eq!(
        fs::read_to_string(old_path).expect("old result"),
        "old content"
    );
    assert_eq!(
        fs::read_to_string(new_path).expect("new result"),
        "new content"
    );
}

#[test]
fn rename_file_moves_content_to_a_new_name() {
    let directory = tempfile::tempdir().expect("temp directory");
    let old_path = directory.path().join("old.md");
    let new_path = directory.path().join("new.md");
    fs::write(&old_path, "content").expect("fixture");

    rename_file_impl(&old_path, &new_path).expect("rename");

    assert!(!old_path.exists());
    assert_eq!(fs::read_to_string(new_path).expect("result"), "content");
}

#[test]
fn invalid_utf8_is_rejected() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("note.md");
    fs::write(&path, [0xff, 0xfe]).expect("fixture");

    let error = read_text(&path).expect_err("invalid UTF-8");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn command_error_has_a_stable_serialized_code() {
    let error = FileCommandError::AlreadyExists {
        path: "note.md".to_owned(),
    };

    let value = serde_json::to_value(error).expect("serialized error");

    assert_eq!(value["code"], "already_exists");
    assert_eq!(value["details"]["path"], "note.md");
}

#[test]
fn workspace_filters_and_sorts_entries() {
    let directory = tempfile::tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("Folder")).expect("folder fixture");
    fs::write(directory.path().join("Beta.md"), "").expect("beta fixture");
    fs::write(directory.path().join("alpha.md"), "").expect("alpha fixture");
    fs::write(directory.path().join("Gamma.MD"), "").expect("gamma fixture");
    fs::write(directory.path().join("photo.png"), "").expect("image fixture");
    fs::write(directory.path().join("doc.pdf"), "").expect("pdf fixture");
    fs::write(directory.path().join("novel.epub"), "").expect("epub fixture");
    fs::write(directory.path().join("notes.txt"), "").expect("text fixture");
    fs::write(directory.path().join(".hidden.md"), "").expect("hidden fixture");

    let items = read_directory_impl(directory.path()).expect("workspace");
    let names: Vec<_> = items.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(
        names,
        vec![
            "Folder",
            "alpha",
            "Beta",
            "doc.pdf",
            "Gamma",
            "notes.txt",
            "novel.epub",
            "photo.png"
        ]
    );
    assert_eq!(items[0].item_type, FileItemType::Folder);
    assert_eq!(items[1].item_type, FileItemType::File);
}

#[test]
fn reading_normalises_crlf_but_the_hash_stays_the_hash_of_the_bytes_on_disk() {
    let directory = tempfile::tempdir().expect("workspace");
    let path = directory.path().join("Заметка.md");
    fs::write(&path, "первая\r\nвторая\r\nтретья").expect("fixture");

    assert_eq!(read_text(&path).expect("text"), "первая\nвторая\nтретья");
    assert_eq!(
        hash_file(&path).expect("hash"),
        hash_bytes("первая\r\nвторая\r\nтретья".as_bytes()),
        "хэш остаётся хэшем файла: на нём держится проверка при записи"
    );
}

#[test]
fn a_lone_carriage_return_is_normalised_too() {
    assert_eq!(normalize_line_endings("одна\rдве".to_owned()), "одна\nдве");
    assert_eq!(normalize_line_endings("без переносов".to_owned()), "без переносов");
    assert_eq!(normalize_line_endings("\r\rа\r\nб\r\r\nв\n\r".to_owned()), "\n\nа\nб\n\nв\n\n");
}

#[test]
fn spliced_write_streams_the_normalized_text_with_replacements() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("Source.md");
    fs::write(&path, "[[note]] и [[note|подпись]]\r\n").unwrap();
    let changes = vec![(2..6, "Note".to_owned()), (14..18, "Note".to_owned())];
    let written = super::document::write_spliced_atomic(&path, &changes, None).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(text, "[[Note]] и [[Note|подпись]]\n");
    assert_eq!(written.hash, hash_bytes(text.as_bytes()));
}
