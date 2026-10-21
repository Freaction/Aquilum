use super::*;
use aquilum_core::files::document::hash_bytes;
use aquilum_core::search::paths::canonical_path;
use aquilum_core::settings::models::SyncSettings;
use aquilum_core::sync::{Memory, Records, Size};
use crate::ui::settings::schema::Section;

#[test]
#[ignore]
fn sync_settings_snapshot() {
    let h = Harness::new();
    let (root, sync) = (h.vault.path().to_path_buf(), h._data.path().join("sync"));
    std::fs::create_dir_all(&sync).unwrap();
    std::fs::write(sync.join("dropbox.json"), r#"{"access":"x","refresh":"x","expires":99999999999999}"#).unwrap();
    std::fs::create_dir_all(root.join("Видео")).unwrap();
    let sized = |path: std::path::PathBuf, megabytes: u64| std::fs::File::create(path).unwrap().set_len(megabytes * 1024 * 1024).unwrap();
    sized(root.join("Видео").join("Лекция.mp4"), 24);
    sized(root.join("Книга.pdf"), 12);
    std::fs::create_dir_all(root.join(".trash")).unwrap();
    sized(root.join(".trash").join("Старое видео.mp4"), 16);
    let key = hash_bytes(canonical_path(&root).to_string_lossy().as_bytes());
    let memory = Memory {
        done: Some(1_761_000_000_000),
        large: Vec::new(),
        size: Size::default(),
        allowed: vec!["Книга.pdf".to_owned()],
    };
    Records::open(&sync.join(format!("{key}.sqlite3"))).unwrap().remember(&memory).unwrap();
    let enabled = SyncSettings { enabled: true, ..SyncSettings::default() };
    h.app.workspace.core.ui_state.set_sync(&root.to_string_lossy(), &enabled).unwrap();

    let mut h = h.restarted(1.0);
    let started = std::time::Instant::now();
    while h.app.workspace.core.sync.large().len() < 2 {
        assert!(started.elapsed().as_secs() < 10, "список больших файлов не появился");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    h.app.open_settings(&mut h.root);
    h.open_section(Section::Sync);
    h.snapshot("sync-settings.png");
    while h.app.workspace.core.sync.due().is_none() {
        assert!(started.elapsed().as_secs() < 30, "проход не закончился");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    h.app.refresh_settings(&mut h.root);
    h.anim(std::time::Duration::from_millis(100));
    h.snapshot("sync-settings-countdown.png");
    h.scroll((600.0, 400.0), 1.0, (0.0, -600.0));
    h.anim(std::time::Duration::from_secs(1));
    h.snapshot("sync-settings-large.png");
    h.open_section(Section::Trash);
    h.snapshot("trash-settings.png");
}
