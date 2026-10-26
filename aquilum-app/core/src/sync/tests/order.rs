use super::device::{Device, EVERYTHING, NOTES_ONLY};
use super::fake::FakeRemote;
use crate::sync::scan::service;
use std::time::Duration;

const FOLDERS: usize = 6;

fn cloud_vault(remote: &FakeRemote) {
    for index in 0..FOLDERS * 2 {
        remote.put(&format!("Папка {}/Заметка {index}.md", index % FOLDERS), "текст");
        remote.put(&format!(".aquilum/history/Заметка {index}.md/1000_aaaa_me.md"), "версия");
    }
    remote.put(".trash/Старое.md", "удалённое");
}

#[test]
fn folders_come_first_then_notes_then_service_files() {
    let remote = FakeRemote::default();
    cloud_vault(&remote);
    remote.slow_transfers(Duration::from_millis(20));
    let device = Device::with(EVERYTHING);

    assert!(device.sync_until(&remote, 1).stopped);
    let folders = (0..FOLDERS).filter(|index| device.root.join(format!("Папка {index}")).is_dir()).count();
    assert_eq!(folders, FOLDERS, "все папки из облака появляются до первой заметки");
    assert!(device.files().len() < FOLDERS * 2);

    assert!(device.sync(&remote).failed.is_empty());
    let downloads = remote.downloads();
    let first_service = downloads.iter().position(|path| service(path)).unwrap();
    assert!(downloads[first_service..].iter().all(|path| service(path)), "служебные файлы — после всех заметок: {downloads:?}");
    assert_eq!(downloads.len() - first_service, FOLDERS * 2 + 1);
}

#[test]
fn without_synced_history_a_pulled_note_keeps_a_version_of_its_own() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::with(NOTES_ONLY), Device::with(NOTES_ONLY));
    laptop.keep_history();
    phone.keep_history();
    laptop.save("Заметка.md", "первый текст");

    laptop.sync(&remote);
    phone.sync(&remote);

    assert_eq!((laptop.versions("Заметка.md"), phone.versions("Заметка.md")), (1, 1));
}

#[test]
fn transfers_run_four_at_a_time_and_report_every_file() {
    let remote = FakeRemote::default();
    cloud_vault(&remote);
    remote.slow_transfers(Duration::from_millis(20));
    let device = Device::with(EVERYTHING);

    let outcome = device.sync(&remote);

    assert!(outcome.failed.is_empty() && !outcome.stopped);
    assert_eq!(remote.peak(), 4);
    let total = FOLDERS * 4 + 1;
    let moves = device.probe.moves.lock().unwrap().clone();
    assert_eq!(moves.iter().map(|progress| (progress.done, progress.total)).collect::<Vec<_>>(), (0..=total).map(|done| (done, total)).collect::<Vec<_>>());
    assert!(device.records.state("cursor").unwrap().is_some());
    assert_eq!(device.files().len(), FOLDERS * 2);
}

#[test]
fn the_tree_hears_about_every_arrived_folder_and_note_at_once() {
    let remote = FakeRemote::default();
    remote.put("Папка/Вложенная/Заметка.md", "текст");
    let device = Device::new();

    assert!(device.sync(&remote).failed.is_empty());

    let announced = device.announced.lock().unwrap().clone();
    for path in ["Папка", "Папка/Вложенная", "Папка/Вложенная/Заметка.md"] {
        let full = crate::sync::scan::absolute(&device.root, path).to_string_lossy().into_owned();
        assert!(announced.contains(&full), "{path} не объявлен: {announced:?}");
    }
}
