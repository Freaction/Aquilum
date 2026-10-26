use super::device::{Device, NOTES_ONLY};
use super::fake::FakeRemote;
use crate::search::paths::canonical_path;
use crate::settings::models::SyncSettings;
use crate::sync::service::SyncService;
use crate::sync::{Large, Remote, RemoteError, SyncError, SyncStatus, LARGE};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const QUIET: Duration = Duration::from_millis(300);

fn started(device: &Device, remote: &Arc<FakeRemote>, folder: &Path) -> SyncService {
    fs::create_dir_all(folder).unwrap();
    fs::write(folder.join("dropbox.json"), "{}").unwrap();
    let remote = Arc::clone(remote);
    let service = SyncService::with(folder.to_path_buf(), Box::new(move |_, _| Ok(Arc::clone(&remote) as Arc<dyn Remote>)));
    service.start(Arc::downgrade(&device.core));
    service.set_vault(Some((device.root.clone(), NOTES_ONLY)), 0);
    service
}

fn finish(service: SyncService) {
    let shared = Arc::clone(&service.shared);
    drop(service);
    until("поток синхронизации завершился", || Arc::strong_count(&shared) == 1);
}

fn synced(service: &SyncService) -> bool {
    service.status() == SyncStatus::Ready && service.last().is_some()
}

fn until(what: &str, condition: impl Fn() -> bool) {
    let start = Instant::now();
    while !condition() {
        assert!(start.elapsed() < Duration::from_secs(20), "не дождались: {what}");
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn turning_the_vault_off_holds_the_pass_until_it_is_turned_on() {
    let (device, remote, folder) = (Device::new(), Arc::new(FakeRemote::default()), tempfile::tempdir().unwrap());
    remote.slow_transfers(Duration::from_millis(5));
    for index in 0..100 {
        device.write(&format!("Заметка {index}.md"), &index.to_string());
    }
    let service = started(&device, &remote, folder.path());
    until("проход начал отправку", || matches!(service.status(), SyncStatus::Running(progress) if progress.done >= 1));
    service.set_vault(Some((device.root.clone(), SyncSettings { enabled: false, ..NOTES_ONLY })), 0);
    thread::sleep(QUIET);
    let sent = remote.paths().len();

    for _ in 0..3 {
        service.request(Duration::ZERO);
    }
    thread::sleep(QUIET);

    assert!(sent < 100);
    assert_eq!(remote.paths().len(), sent);
    service.set_vault(Some((device.root.clone(), NOTES_ONLY)), 0);
    until("проход завершился", || synced(&service));
    assert_eq!(remote.paths().len(), 100);
    finish(service);
}

#[test]
fn the_next_pass_waits_for_the_vault_interval() {
    let (device, remote, folder) = (Device::new(), Arc::new(FakeRemote::default()), tempfile::tempdir().unwrap());
    device.write("Заметка.md", "текст");
    let service = started(&device, &remote, folder.path());
    until("проход завершился", || synced(&service));
    thread::sleep(QUIET);

    let left = service.due().map(|due| due.saturating_duration_since(Instant::now()));
    assert!(left.is_some_and(|left| left > Duration::from_secs(NOTES_ONLY.interval - 10)), "{left:?}");

    let ready = service.status();
    service.set_vault(Some((device.root.clone(), SyncSettings { interval: 30, ..NOTES_ONLY })), 0);
    let left = service.due().map(|due| due.saturating_duration_since(Instant::now()));
    assert!(left.is_some_and(|left| left > Duration::from_secs(29) && left <= Duration::from_secs(30)), "{left:?}");
    thread::sleep(QUIET);
    assert_eq!(service.status(), ready, "смена периода не запускает проход");
    finish(service);
}

#[test]
fn only_real_edits_count_as_touched() {
    let (laptop, phone, remote, folder) = (Device::new(), Device::new(), Arc::new(FakeRemote::default()), tempfile::tempdir().unwrap());
    laptop.write("Заметка.md", "из облака");
    laptop.write("Удалить.md", "лишнее");
    laptop.write("Папка/Вложенная.md", "в папке");
    laptop.sync(remote.as_ref());
    let service = started(&phone, &remote, folder.path());
    until("проход завершился", || synced(&service));
    let touched = |name: &str| service.touched(&[canonical_path(&phone.root).join(name)], false);

    assert!(!touched("Заметка.md"), "скачанный синхронизацией файл — не правка");
    phone.write(".obsidian/Заметка.md", "скрытое");
    phone.write(".trash/Старое.md", "корзина вне настроек");
    assert!(!touched(".obsidian/Заметка.md") && !touched(".trash/Старое.md"));
    phone.write("Заметка.md", "правка здесь");
    phone.write("Новая.md", "новая");
    fs::remove_file(phone.root.join("Удалить.md")).unwrap();
    fs::remove_dir_all(phone.root.join("Папка")).unwrap();
    assert!(touched("Заметка.md") && touched("Новая.md") && touched("Удалить.md") && touched("Папка"));
    assert!(service.touched(&[], true), "пересканирование папки — всегда проход");
    finish(service);
}

#[test]
fn large_files_and_the_size_are_known_without_the_cloud() {
    let (device, remote, folder) = (Device::new(), Arc::new(FakeRemote::default()), tempfile::tempdir().unwrap());
    remote.go_offline();
    device.write("Видео.mp4", &"в".repeat(LARGE as usize / 2 + 1));
    device.write("Заметка.md", "текст");

    let service = started(&device, &remote, folder.path());
    until("проход упал без сети", || service.status() == SyncStatus::Offline);

    assert_eq!(service.large(), vec![Large { path: "Видео.mp4".to_owned(), size: LARGE + 2, kept: true }]);
    assert_eq!(service.size().synced, 10);
    finish(service);
}

#[test]
fn a_pass_without_network_is_offline_and_waits_for_its_turn() {
    let (device, remote, folder) = (Device::new(), Arc::new(FakeRemote::default()), tempfile::tempdir().unwrap());
    device.write("Заметка.md", "текст");
    remote.go_offline();

    let service = started(&device, &remote, folder.path());
    until("проход упал без сети", || !matches!(service.status(), SyncStatus::Ready | SyncStatus::Running(_)));

    assert_eq!(service.status(), SyncStatus::Offline);
    assert!(service.due().is_some_and(|due| due > Instant::now() + Duration::from_secs(NOTES_ONLY.interval - 10)));
    finish(service);
}

#[test]
fn the_cloud_vaults_need_a_login() {
    let (remote, folder) = (Arc::new(FakeRemote::default()), tempfile::tempdir().unwrap());
    let service = SyncService::with(folder.path().to_path_buf(), Box::new(move |_, _| Ok(Arc::clone(&remote) as Arc<dyn Remote>)));

    assert!(matches!(service.vaults(), Err(SyncError::Remote(RemoteError::Auth))));
}

#[test]
fn sending_a_large_file_marks_it_at_once_and_uploads_it() {
    let (device, remote, folder) = (Device::new(), Arc::new(FakeRemote::default()), tempfile::tempdir().unwrap());
    device.write("Видео.mp4", &"в".repeat(LARGE as usize / 2 + 1));
    let service = started(&device, &remote, folder.path());
    until("большой файл в списке", || service.large().first().is_some_and(|large| large.kept));
    until("проход завершился", || synced(&service));

    service.allow(vec!["Видео.mp4".to_owned()]);

    assert!(!service.large()[0].kept);
    until("файл в облаке", || remote.paths() == vec!["Видео.mp4".to_owned()]);
    finish(service);
}
