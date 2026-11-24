use super::device::Device;
use super::fake::FakeRemote;
use crate::sync::Records;
use std::collections::HashSet;

#[test]
fn a_case_only_rename_is_the_same_file() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Заметка.md", "текст");
    device.sync(&remote);

    std::fs::rename(device.root.join("Заметка.md"), device.root.join("заметка.md")).unwrap();
    let quiet = device.sync(&remote);
    assert!(quiet.failed.is_empty(), "{:?}", quiet.failed);
    assert_eq!((quiet.progress.total, remote.uploads(), remote.paths().len()), (0, 1, 1));

    device.write("заметка.md", "правка");
    let laptop = Device::new();
    assert!(device.sync(&remote).failed.is_empty());
    assert!(laptop.sync(&remote).failed.is_empty());
    assert_eq!(remote.paths().len(), 1);
    assert_eq!(remote.text("Заметка.md").as_deref(), Some("правка"));
    assert_eq!(laptop.read("Заметка.md").or_else(|| laptop.read("заметка.md")).as_deref(), Some("правка"));
}

fn notes(device: &Device) -> Vec<String> {
    let names = std::fs::read_dir(&device.root).unwrap().filter_map(Result::ok).map(|entry| entry.file_name().to_string_lossy().into_owned());
    names.filter(|name| name.ends_with(".md")).collect()
}

fn twin(device: &Device) -> bool {
    let lower = device.root.join("заметка.md");
    if lower.exists() {
        return false;
    }
    std::fs::write(&lower, "строчная").unwrap();
    true
}

#[test]
fn a_local_case_twin_becomes_a_conflict_copy_and_both_reach_the_cloud() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Заметка.md", "заглавная");
    if !twin(&device) {
        return;
    }

    assert!(device.sync(&remote).failed.is_empty());

    let names = notes(&device);
    assert_eq!((names.len(), remote.uploads(), remote.paths().len()), (2, 2, 2), "{names:?}");
    assert!(names.iter().any(|name| name.contains("(конфликт")), "{names:?}");
    let texts: HashSet<String> = names.iter().filter_map(|name| device.read(name)).collect();
    assert_eq!(texts, HashSet::from(["заглавная".to_owned(), "строчная".to_owned()]));
}

#[test]
fn the_synced_name_stays_and_the_new_twin_is_set_aside() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Заметка.md", "заглавная");
    device.sync(&remote);
    if !twin(&device) {
        return;
    }

    assert!(device.sync(&remote).failed.is_empty());

    assert_eq!(device.read("Заметка.md").as_deref(), Some("заглавная"));
    assert_eq!(remote.text("Заметка.md").as_deref(), Some("заглавная"));
    assert_eq!(remote.paths().len(), 2);
}

#[test]
fn old_records_gain_a_case_key_and_keep_one_row_per_name() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("records.sqlite3");
    let old = rusqlite::Connection::open(&path).unwrap();
    old.execute_batch(
        "CREATE TABLE records (path TEXT PRIMARY KEY, hash TEXT NOT NULL, size INTEGER NOT NULL, modified INTEGER NOT NULL, remote_id TEXT NOT NULL, remote_rev TEXT NOT NULL, checksum TEXT NOT NULL, remote_size INTEGER NOT NULL, remote_modified INTEGER NOT NULL);
         INSERT INTO records VALUES ('Папка/Заметка.md', 'h', 1, 1, 'a', 'r', 'c', 1, 1), ('папка/заметка.md', 'h', 1, 1, 'b', 'r', 'c', 1, 1), ('Другая.md', 'h', 1, 1, 'c', 'r', 'c', 1, 1);",
    )
    .unwrap();
    drop(old);

    let records = Records::open(&path).unwrap();

    assert_eq!(records.all().unwrap().len(), 2);
    assert!(records.get("ПАПКА/ЗАМЕТКА.md").unwrap().is_some());
    assert!(records.holds_under("папка").unwrap());
    records.remove("другая.MD").unwrap();
    assert!(records.get("Другая.md").unwrap().is_none());
    drop(records);
    assert_eq!(Records::open(&path).unwrap().all().unwrap().len(), 1);
}
