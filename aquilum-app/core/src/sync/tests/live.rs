use super::device::Device;
use crate::sync::dropbox::{Dropbox, CLIENT};
use crate::sync::{Account, Outcome};
use crate::web_agent::{agent, web_agent};
use std::env;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

fn dropbox(folder: &str) -> Dropbox {
    let path = PathBuf::from(env::var("AQUILUM_DROPBOX_ACCOUNT").expect("путь к файлу токенов в AQUILUM_DROPBOX_ACCOUNT"));
    let account = match Account::open(CLIENT, web_agent("sync"), path.clone()).unwrap() {
        Some(account) => account,
        None => {
            let login = CLIENT.login(None).unwrap();
            eprintln!("ВОЙТИ: {}", login.url);
            let tokens = login.finish(&web_agent("sync"), &AtomicBool::new(false)).unwrap();
            Account::new(CLIENT, web_agent("sync"), path, tokens).unwrap()
        }
    };
    Dropbox::new(account, agent("sync", Duration::from_secs(60)), folder)
}

struct Remove<'a>(&'a Dropbox);

impl Drop for Remove<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.0.remove() {
            eprintln!("папка проверки в Dropbox не удалена: {error}");
        }
    }
}

fn clean(outcome: Outcome) {
    assert!(outcome.failed.is_empty(), "{:?}", outcome.failed);
}

#[test]
#[ignore]
fn two_devices_sync_through_dropbox() {
    let remote = dropbox(&format!("Проверка {}", uuid::Uuid::new_v4().simple()));
    let _remove = Remove(&remote);
    let (laptop, phone) = (Device::new(), Device::new());
    laptop.write("Заметка.md", "один\nдва\nтри\n");
    laptop.write("Папка/Вложение.bin", "\u{0}\u{1}\u{2}");
    clean(laptop.sync(&remote));
    clean(phone.sync(&remote));
    assert_eq!(phone.read("Заметка.md").as_deref(), Some("один\nдва\nтри\n"));
    assert!(phone.modified("Заметка.md").abs_diff(laptop.modified("Заметка.md")) < 1000);

    laptop.write("Заметка.md", "ОДИН\nдва\nтри\n");
    phone.write("Заметка.md", "один\nдва\nтри!\n");
    clean(laptop.sync(&remote));
    clean(phone.sync(&remote));
    clean(laptop.sync(&remote));
    assert_eq!(laptop.read("Заметка.md").as_deref(), Some("ОДИН\nдва\nтри!\n"));
    assert_eq!(phone.read("Заметка.md").as_deref(), Some("ОДИН\nдва\nтри!\n"));

    phone.delete("Папка/Вложение.bin");
    clean(phone.sync(&remote));
    clean(laptop.sync(&remote));
    assert!(laptop.read("Папка/Вложение.bin").is_none());

    laptop.delete("Заметка.md");
    clean(laptop.sync(&remote));
    clean(phone.sync(&remote));
    assert_eq!(phone.files(), Vec::<String>::new());
}
