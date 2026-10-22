mod device;
mod fake;
mod large;
mod live;
mod random;
mod recovery;
mod safety;
mod scope;
mod service;

use device::Device;
use fake::FakeRemote;

#[test]
fn first_sync_sends_local_notes_and_brings_cloud_notes() {
    let remote = FakeRemote::default();
    remote.put("Облако.md", "из облака");
    let device = Device::new();
    device.write("Папка/Здесь.md", "отсюда");

    assert!(device.sync(&remote).failed.is_empty());

    assert_eq!(remote.text("Папка/Здесь.md").as_deref(), Some("отсюда"));
    assert_eq!(device.read("Облако.md").as_deref(), Some("из облака"));
}

#[test]
fn a_quiet_second_sync_does_nothing() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Заметка.md", "текст");
    device.sync(&remote);

    let outcome = device.sync(&remote);

    assert_eq!((outcome.progress.total, outcome.failed.len(), remote.uploads()), (0, 0, 1));
}

#[test]
fn edits_travel_between_devices() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::new(), Device::new());
    laptop.write("Идея.md", "черновик");
    laptop.sync(&remote);
    phone.sync(&remote);

    phone.write("Идея.md", "черновик, дописанный с телефона");
    phone.sync(&remote);
    laptop.sync(&remote);

    assert_eq!(laptop.read("Идея.md").as_deref(), Some("черновик, дописанный с телефона"));
}

#[test]
fn deletions_go_to_the_trash_on_the_other_side() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::new(), Device::new());
    laptop.write("Первая.md", "один");
    laptop.write("Вторая.md", "два");
    laptop.sync(&remote);
    phone.sync(&remote);

    phone.delete("Первая.md");
    phone.sync(&remote);
    laptop.sync(&remote);
    laptop.delete("Вторая.md");
    laptop.sync(&remote);

    assert_eq!(laptop.files(), Vec::<String>::new());
    assert!(laptop.trashed());
    assert_eq!(remote.paths(), Vec::<String>::new());
}

#[test]
fn an_edit_wins_over_a_deletion() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::new(), Device::new());
    laptop.write("Заметка.md", "было");
    laptop.sync(&remote);
    phone.sync(&remote);

    laptop.delete("Заметка.md");
    laptop.sync(&remote);
    phone.write("Заметка.md", "было и стало");
    phone.sync(&remote);
    laptop.sync(&remote);

    assert_eq!(remote.text("Заметка.md").as_deref(), Some("было и стало"));
    assert_eq!(laptop.read("Заметка.md").as_deref(), Some("было и стало"));
}

#[test]
fn edits_on_both_sides_merge_line_by_line() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::new(), Device::new());
    laptop.write("Список.md", "один\nдва\nтри\n");
    laptop.sync(&remote);
    phone.sync(&remote);

    laptop.write("Список.md", "ОДИН\nдва\nтри\n");
    laptop.sync(&remote);
    phone.write("Список.md", "один\nдва\nтри!\n");
    assert!(phone.sync(&remote).failed.is_empty());
    laptop.sync(&remote);

    let merged = Some("ОДИН\nдва\nтри!\n");
    assert_eq!((phone.read("Список.md").as_deref(), remote.text("Список.md").as_deref()), (merged, merged));
    assert_eq!(laptop.read("Список.md").as_deref(), merged);
    assert_eq!(phone.files(), vec!["Список.md".to_owned()]);
}

#[test]
fn without_a_common_version_both_texts_survive() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::new(), Device::new());
    laptop.write("Спор.md", "общее начало");
    laptop.sync(&remote);
    phone.sync(&remote);

    laptop.write("Спор.md", "версия ноутбука");
    laptop.sync(&remote);
    remote.forget_history();
    phone.write("Спор.md", "версия телефона");
    phone.sync(&remote);
    phone.sync(&remote);

    assert_eq!(remote.text("Спор.md").as_deref(), Some("версия телефона"));
    let copy = phone.files().into_iter().find(|path| path.contains("(конфликт")).expect("копия");
    assert_eq!(phone.read(&copy).as_deref(), Some("версия ноутбука"));
    assert_eq!(remote.text(&copy).as_deref(), Some("версия ноутбука"));
}

#[test]
fn equal_files_meet_without_uploading() {
    let remote = FakeRemote::default();
    remote.put("Общая.md", "одинаково");
    let device = Device::new();
    device.write("Общая.md", "одинаково");

    device.sync(&remote);

    assert_eq!((remote.uploads(), device.files()), (0, vec!["Общая.md".to_owned()]));
}

#[test]
fn a_failed_upload_keeps_the_cursor_and_the_next_sync_finishes() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Старая.md", "есть");
    device.sync(&remote);
    let cursor = device.records.state("cursor").unwrap();

    device.write("Новая.md", "ещё нет");
    remote.refuse_uploads(true);
    assert_eq!(device.sync(&remote).failed.len(), 1);
    assert_eq!(device.records.state("cursor").unwrap(), cursor);

    remote.refuse_uploads(false);
    assert!(device.sync(&remote).failed.is_empty());
    assert_eq!(remote.text("Новая.md").as_deref(), Some("ещё нет"));
}

#[test]
fn service_folders_stay_on_their_device_and_dot_folders_sync() {
    let remote = FakeRemote::default();
    remote.put(".obsidian/app.json", "{}");
    remote.put(".архив/Старое.md", "из облака");
    let device = Device::new();
    device.write(".aquilum/заметка.md", "служебное");
    device.write(".DS_Store", "finder");
    device.write(".заметки/Идея.md", "с точкой");
    device.write("Видимая.md", "текст");

    device.sync(&remote);

    assert_eq!(remote.paths(), [".obsidian/app.json", ".архив/Старое.md", ".заметки/Идея.md", "Видимая.md"]);
    assert!(device.read(".obsidian/app.json").is_none());
    assert_eq!(device.read(".архив/Старое.md").as_deref(), Some("из облака"));
}
