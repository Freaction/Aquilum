use super::device::Device;
use super::fake::FakeRemote;

fn two_synced(text: &str) -> (FakeRemote, Device, Device) {
    let remote = FakeRemote::default();
    let (laptop, phone) = (Device::new(), Device::new());
    laptop.write("Заметка.md", text);
    laptop.sync(&remote);
    phone.sync(&remote);
    (remote, laptop, phone)
}

#[test]
fn an_edit_with_the_same_size_and_time_is_not_stuck_forever() {
    let (remote, laptop, phone) = two_synced("abc\n");
    let synced = laptop.modified("Заметка.md");
    laptop.write("Заметка.md", "abd\n");
    laptop.set_modified("Заметка.md", synced);

    phone.write("Заметка.md", "abc\nс телефона\n");
    phone.sync(&remote);
    laptop.sync(&remote);

    let last = laptop.sync(&remote);
    assert!(last.failed.is_empty(), "{:?}", last.failed);
    let text = laptop.read("Заметка.md").unwrap();
    assert!(text.contains("с телефона"));
    assert!(text.contains("abd") || laptop.copies().iter().any(|copy| laptop.read(copy).unwrap().contains("abd")));
}

#[test]
fn a_cloud_file_missing_from_the_changes_is_found_by_a_full_listing() {
    let remote = FakeRemote::default();
    let device = Device::new();
    device.write("Старая.md", "есть");
    device.sync(&remote);

    remote.put_quietly("Новая.md", "из облака");
    device.write("Новая.md", "с устройства");
    assert_eq!(device.sync(&remote).failed.len(), 1);
    let second = device.sync(&remote);

    assert!(second.failed.is_empty(), "{:?}", second.failed);
    assert_eq!(device.read("Новая.md").as_deref(), Some("с устройства"));
    assert_eq!(device.copies().iter().map(|copy| device.read(copy).unwrap()).collect::<Vec<_>>(), ["из облака"]);
}

#[test]
fn a_failed_upload_leaves_no_extra_conflict_copies() {
    let remote = FakeRemote::default();
    remote.put("Спор.md", "из облака");
    let device = Device::new();
    device.write("Спор.md", "с устройства");

    remote.refuse_uploads(true);
    device.sync(&remote);
    device.sync(&remote);
    assert!(device.copies().is_empty());
    remote.refuse_uploads(false);
    device.sync(&remote);

    assert_eq!(device.copies().len(), 1);
}

#[test]
fn identical_edits_on_both_devices_make_no_copy() {
    let (remote, laptop, phone) = two_synced("один\nдва\nтри\n");

    laptop.write("Заметка.md", "один\nДВА\nтри\n");
    laptop.sync(&remote);
    phone.write("Заметка.md", "один\nДВА\nтри\nчетыре\n");
    phone.sync(&remote);

    assert_eq!(phone.read("Заметка.md").as_deref(), Some("один\nДВА\nтри\nчетыре\n"));
    assert!(phone.copies().is_empty());
}

#[test]
fn a_note_that_is_not_utf8_keeps_both_versions() {
    let (remote, laptop, phone) = two_synced("текст");

    laptop.write_bytes("Заметка.md", &[0xFF, 0xFE, 0x01]);
    laptop.sync(&remote);
    phone.write_bytes("Заметка.md", &[0xFF, 0xFE, 0x02, 0x03]);
    let outcome = phone.sync(&remote);

    assert!(outcome.failed.is_empty(), "{:?}", outcome.failed);
    assert_eq!(phone.copies().len(), 1);
}

#[test]
fn a_truncated_base_version_never_reaches_the_note() {
    let (remote, laptop, phone) = two_synced("один\nдва\nтри\n");
    laptop.write("Заметка.md", "ОДИН\nдва\nтри\n");
    laptop.sync(&remote);
    phone.write("Заметка.md", "один\nдва\nтри!\n");

    remote.short_downloads(true);
    assert!(!phone.sync(&remote).failed.is_empty());
    assert_eq!(phone.read("Заметка.md").as_deref(), Some("один\nдва\nтри!\n"));
    remote.short_downloads(false);
    phone.sync(&remote);

    assert_eq!(phone.read("Заметка.md").as_deref(), Some("ОДИН\nдва\nтри!\n"));
}

#[test]
fn a_download_left_by_a_crash_is_cleared_by_the_next_pass() {
    let remote = FakeRemote::default();
    let device = Device::new();
    std::fs::create_dir_all(device.staging()).unwrap();
    std::fs::write(device.staging().join("оборванная загрузка"), "часть файла").unwrap();

    device.sync(&remote);

    assert!(!device.staging().join("оборванная загрузка").exists());
}
