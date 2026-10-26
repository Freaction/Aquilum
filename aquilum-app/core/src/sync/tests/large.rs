use super::device::{Device, EVERYTHING};
use super::fake::FakeRemote;
use crate::sync::{Large, LARGE};

fn large_text() -> String {
    "в".repeat(LARGE as usize / 2 + 1)
}

fn listed(device: &Device) -> Vec<Large> {
    device.probe.measured.lock().unwrap().0.clone()
}

fn large(path: &str, size: u64, kept: bool) -> Large {
    Large { path: path.to_owned(), size, kept }
}

#[test]
fn a_large_file_stays_on_the_device_and_is_listed() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Видео/Лекция.mp4", &large_text());
    device.write("Заметка.md", "текст");

    device.sync(&remote);

    assert_eq!(remote.paths(), vec!["Заметка.md".to_owned()]);
    assert_eq!(listed(&device), vec![large("Видео/Лекция.mp4", LARGE + 2, true)]);
}

#[test]
fn a_synced_file_that_grows_large_is_deleted_nowhere() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::new(), Device::new());
    laptop.write("Схема.pdf", "маленькая");
    laptop.sync(&remote);
    phone.sync(&remote);

    laptop.write("Схема.pdf", &large_text());
    laptop.sync(&remote);
    phone.sync(&remote);

    assert_eq!(remote.text("Схема.pdf").as_deref(), Some("маленькая"));
    assert_eq!(phone.read("Схема.pdf").as_deref(), Some("маленькая"));
    assert!(!phone.trashed());
}

#[test]
fn a_large_file_already_in_the_cloud_is_downloaded_and_listed_as_in_the_cloud() {
    let remote = FakeRemote::default();
    remote.put("Архив.zip", &large_text());
    let device = Device::new();

    device.sync(&remote);
    device.sync(&remote);

    assert_eq!(device.read("Архив.zip"), Some(large_text()));
    assert_eq!(listed(&device), vec![large("Архив.zip", LARGE + 2, false)]);
}

#[test]
fn a_large_file_sent_by_hand_reaches_other_devices_and_keeps_syncing() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::new(), Device::new());
    laptop.write("Лекция.mp4", &large_text());
    laptop.sync(&remote);
    assert!(remote.text("Лекция.mp4").is_none());

    laptop.allowed.borrow_mut().push("Лекция.mp4".to_owned());
    laptop.sync(&remote);
    phone.sync(&remote);
    laptop.allowed.borrow_mut().clear();
    laptop.write("Лекция.mp4", &(large_text() + "!"));
    laptop.sync(&remote);
    phone.sync(&remote);

    assert_eq!(phone.read("Лекция.mp4"), Some(large_text() + "!"));
    assert_eq!(listed(&laptop), vec![large("Лекция.mp4", LARGE + 3, false)]);
}

#[test]
fn the_cloud_share_leaves_large_files_out() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Видео.mp4", &large_text());
    device.write("Заметка.md", "текст");

    device.sync(&remote);

    let size = device.probe.measured.lock().unwrap().1;
    assert_eq!((size.synced, size.files), (10, 1));
}

#[test]
fn history_and_trash_out_of_scope_stay_out_of_the_size_and_the_list() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write(".aquilum/history/Папка/Лекция.md/1", &large_text());
    device.write(".trash/.aquilum/history/Старое.md/1", &large_text());
    device.write(".trash/Старое.mp4", "маленькое");
    device.write("Заметка.md", "текст");

    device.sync(&remote);

    assert_eq!(device.probe.measured.lock().unwrap().1.synced, 10);
    assert!(listed(&device).is_empty());
}

#[test]
fn large_history_and_trash_stay_on_the_device_when_they_are_synced() {
    let remote = FakeRemote::default();
    let device = Device::with(EVERYTHING);
    device.write(".aquilum/history/Лекция.md/1", &large_text());
    device.write(".aquilum/history/Лекция.md/2", "маленькая");
    device.write(".trash/Старое.mp4", &large_text());

    device.sync(&remote);

    assert_eq!(remote.paths(), vec![".aquilum/history/Лекция.md/2".to_owned()]);
    assert!(listed(&device).is_empty());
}
