use super::device::Device;
use super::fake::FakeRemote;

#[test]
fn a_folder_deleted_in_the_cloud_takes_its_notes_along() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Папка/Первая.md", "один");
    device.write("Папка/Вложенная/Вторая.md", "два");
    device.write("Остаётся.md", "три");
    device.sync(&remote);

    remote.remove_folder("Папка");
    device.sync(&remote);

    assert_eq!(device.files(), vec!["Остаётся.md".to_owned()]);
    assert!(device.trashed());
}

#[test]
fn an_empty_cloud_after_a_reset_stops_the_sync_instead_of_deleting() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Заметка.md", "текст");
    device.sync(&remote);

    remote.wipe_and_expire();

    assert!(device.try_sync(&remote).is_err());
    assert_eq!(device.read("Заметка.md").as_deref(), Some("текст"));
}

#[test]
fn a_stopped_pass_keeps_its_place_and_the_next_one_finishes() {
    let remote = FakeRemote::default();
    let device = Device::new();
    for name in ["Один", "Два", "Три"] {
        device.write(&format!("{name}.md"), name);
    }

    let stopped = device.sync_until(&remote, 1);

    assert!(stopped.stopped);
    assert_eq!((remote.paths().len(), device.records.state("cursor").unwrap()), (1, None));
    let finished = device.sync(&remote);
    assert!(!finished.stopped && finished.failed.is_empty());
    assert_eq!(remote.paths().len(), 3);
}

#[test]
fn deleting_most_files_waits_for_confirmation() {
    let remote = FakeRemote::default();
    let device = Device::new();
    for index in 0..10 {
        device.write(&format!("Заметка {index}.md"), "текст");
    }
    device.sync(&remote);
    for index in 0..6 {
        device.delete(&format!("Заметка {index}.md"));
    }

    assert!(matches!(device.try_sync(&remote), Err(crate::sync::SyncError::Deletions { planned: 6, synced: 10 })));
    assert_eq!(remote.paths().len(), 10);
    device.allow_deletions.set(true);
    device.sync(&remote);

    assert_eq!(remote.paths().len(), 4);
}

#[test]
fn an_emptied_vault_folder_deletes_nothing_in_the_cloud() {
    let remote = FakeRemote::default();
    let device = Device::new();
    for index in 0..10 {
        device.write(&format!("Папка/Заметка {index}.md"), "текст");
    }
    device.sync(&remote);
    std::fs::remove_dir_all(device.root.join("Папка")).unwrap();

    assert!(device.try_sync(&remote).is_err());

    assert_eq!(remote.paths().len(), 10);
}

#[test]
fn the_last_pass_is_remembered_across_reopening() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("records.sqlite3");
    let memory = crate::sync::Memory {
        done: Some(1),
        large: vec![crate::sync::Large { path: "Видео.mp4".to_owned(), size: 20, kept: true }],
        size: crate::sync::Size { synced: 10, files: 1 },
        allowed: Vec::new(),
    };
    crate::sync::Records::open(&path).unwrap().remember(&memory).unwrap();

    let recalled = crate::sync::Records::open(&path).unwrap().recall();

    assert_eq!((recalled.done, recalled.large, recalled.size), (memory.done, memory.large, memory.size));
}
