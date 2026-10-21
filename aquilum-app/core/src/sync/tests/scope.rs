use super::device::{Device, EVERYTHING, NOTES_ONLY};
use super::fake::FakeRemote;
use crate::settings::models::SyncSettings;

const DELETED_AT: u64 = 1_767_225_600_000;
const VERSION: &str = ".aquilum/history/Идея.md/1000_aaaa_me.md";

#[test]
fn trash_and_history_travel_with_their_dates() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::with(EVERYTHING), Device::with(EVERYTHING));
    laptop.write(".trash/Старое.md", "удалённое");
    laptop.set_modified(".trash/Старое.md", DELETED_AT);
    laptop.write(VERSION, "версия");

    laptop.sync(&remote);
    phone.sync(&remote);

    assert_eq!(phone.read(".trash/Старое.md").as_deref(), Some("удалённое"));
    assert_eq!(phone.modified(".trash/Старое.md"), DELETED_AT);
    assert_eq!(phone.read(VERSION).as_deref(), Some("версия"));
}

#[test]
fn switched_off_trash_and_history_stay_on_the_device() {
    let remote = FakeRemote::default();
    let device = Device::with(NOTES_ONLY);
    device.write(".trash/Старое.md", "удалённое");
    device.write(VERSION, "версия");
    device.write("Идея.md", "текст");

    device.sync(&remote);

    assert_eq!(remote.paths(), vec!["Идея.md".to_owned()]);
}

#[test]
fn trash_without_history_leaves_out_the_history_of_trashed_notes() {
    let remote = FakeRemote::default();
    let device = Device::with(SyncSettings { history: false, ..EVERYTHING });
    device.write(".trash/Старое.md", "удалённое");
    device.write(".trash/.aquilum/history/Старое.md/1000_aaaa_me.md", "версия");

    device.sync(&remote);

    assert_eq!(remote.paths(), vec![".trash/Старое.md".to_owned()]);
}

#[test]
fn a_cleaned_trash_item_disappears_everywhere_without_a_second_trash() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::with(EVERYTHING), Device::with(EVERYTHING));
    laptop.write(".trash/Старое.md", "удалённое");
    laptop.sync(&remote);
    phone.sync(&remote);

    laptop.delete(".trash/Старое.md");
    laptop.sync(&remote);
    phone.sync(&remote);

    assert!(phone.read(".trash/Старое.md").is_none());
    assert!(!phone.root.join(".trash").join(".trash").exists());
    assert_eq!(remote.paths(), Vec::<String>::new());
}

#[test]
fn a_deleted_note_reaches_the_trash_of_every_device() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::with(EVERYTHING), Device::with(EVERYTHING));
    laptop.write("Заметка.md", "текст");
    laptop.sync(&remote);
    phone.sync(&remote);

    phone.delete("Заметка.md");
    phone.sync(&remote);
    laptop.sync(&remote);
    laptop.sync(&remote);
    phone.sync(&remote);

    assert!(laptop.read("Заметка.md").is_none());
    assert_eq!(phone.read(".trash/Заметка.md").as_deref(), Some("текст"));
}

#[test]
fn a_pulled_note_reuses_the_history_version_that_came_with_it() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::with(EVERYTHING), Device::with(EVERYTHING));
    laptop.keep_history();
    phone.keep_history();
    laptop.save("(Черновик).md", "первый текст");

    laptop.sync(&remote);
    phone.sync(&remote);
    phone.sync(&remote);
    laptop.sync(&remote);

    assert_eq!(phone.read("(Черновик).md").as_deref(), Some("первый текст"));
    assert_eq!((laptop.versions("(Черновик).md"), phone.versions("(Черновик).md")), (1, 1));
}

#[test]
fn turning_history_on_later_brings_what_the_cloud_already_has() {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::with(EVERYTHING), Device::with(NOTES_ONLY));
    laptop.write(VERSION, "версия");
    laptop.sync(&remote);
    phone.sync(&remote);
    assert!(phone.read(VERSION).is_none());

    phone.scope.set(EVERYTHING);
    phone.sync(&remote);

    assert_eq!(phone.read(VERSION).as_deref(), Some("версия"));
}
