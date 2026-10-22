use crate::files::remove_matching;
use std::path::Path;

const REBUILT_CACHES: [&str; 1] = ["search-v2"];
const TAURI_FILES: [&str; 3] = ["documents.sqlite3", "documents.sqlite3-shm", "documents.sqlite3-wal"];

pub fn remove_tauri_leftovers(data_dir: &Path, webview_dir: Option<&Path>) {
    remove_matching(data_dir, |name| TAURI_FILES.contains(&name) || name.starts_with(".window-state.json."));
    if let Some(webview_dir) = webview_dir {
        remove_matching(webview_dir, |name| name == "EBWebView");
        let _ = std::fs::remove_dir(webview_dir);
    }
}

pub fn migrate_legacy_data(target_dir: &Path) {
    if target_dir.join("ui-state.sqlite3").exists() || target_dir.join("settings.json").exists() {
        return;
    }
    let Some(parent) = target_dir.parent() else {
        return;
    };
    let legacy_dir = parent.join("com.dmitriy.quantum-app");
    if !legacy_dir.is_dir() {
        return;
    }
    let entries = match std::fs::create_dir_all(target_dir).and_then(|()| std::fs::read_dir(&legacy_dir)) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("[aquilum:migration] данные quantum-app не перенесены: {error}");
            return;
        }
    };
    for entry in entries.flatten() {
        if REBUILT_CACHES.iter().any(|cache| entry.file_name() == *cache) {
            continue;
        }
        if let Err(error) = copy_entry(&entry, &target_dir.join(entry.file_name())) {
            eprintln!("[aquilum:migration] не перенесено {}: {error}", entry.path().display());
        }
    }
}

fn copy_entry(entry: &std::fs::DirEntry, target: &Path) -> std::io::Result<()> {
    let file_type = entry.file_type()?;
    if file_type.is_dir() {
        copy_directory_contents(&entry.path(), target)?;
    } else if file_type.is_file() && !target.exists() {
        std::fs::copy(entry.path(), target)?;
    }
    Ok(())
}

fn copy_directory_contents(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        copy_entry(&entry, &target.join(entry.file_name()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{migrate_legacy_data, remove_tauri_leftovers};
    use tempfile::tempdir;

    #[test]
    fn tauri_leftovers_go_and_the_native_data_stays() {
        let root = tempdir().unwrap();
        let (data, local) = (root.path().join("data"), root.path().join("local"));
        std::fs::create_dir_all(local.join("EBWebView/Default")).unwrap();
        std::fs::create_dir_all(&data).unwrap();
        for name in ["documents.sqlite3", "documents.sqlite3-wal", ".window-state.json.moajxL", "window-state.json", "ui-state.sqlite3", "settings.json"] {
            std::fs::write(data.join(name), "").unwrap();
        }

        remove_tauri_leftovers(&data, Some(&local));

        let mut left = std::fs::read_dir(&data).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect::<Vec<_>>();
        left.sort();
        assert_eq!(left, ["settings.json", "ui-state.sqlite3", "window-state.json"]);
        assert!(!local.exists(), "профиль WebView2 и опустевшая папка удалены");
    }

    #[test]
    fn migrates_legacy_data_when_target_is_empty() {
        let root = tempdir().unwrap();
        let legacy = root.path().join("com.dmitriy.quantum-app");
        let target = root.path().join("com.dmitriy.aquilum-app");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("settings.json"), "{\"test\":true}").unwrap();
        std::fs::write(legacy.join("ui-state.sqlite3"), "mock-db").unwrap();
        std::fs::create_dir_all(legacy.join("search-v2/workspace/index-v9")).unwrap();
        std::fs::write(legacy.join("search-v2/workspace/index-v9/meta.json"), "{}").unwrap();

        migrate_legacy_data(&target);

        assert!(!target.join("search-v2").exists(), "поисковый кэш строится заново, его не копируют");
        assert!(target.join("settings.json").exists());
        assert!(target.join("ui-state.sqlite3").exists());
        assert_eq!(
            std::fs::read_to_string(target.join("settings.json")).unwrap(),
            "{\"test\":true}"
        );
    }

    #[test]
    fn does_not_overwrite_existing_target_data() {
        let root = tempdir().unwrap();
        let legacy = root.path().join("com.dmitriy.quantum-app");
        let target = root.path().join("com.dmitriy.aquilum-app");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(legacy.join("settings.json"), "legacy").unwrap();
        std::fs::write(target.join("settings.json"), "current").unwrap();

        migrate_legacy_data(&target);

        assert_eq!(
            std::fs::read_to_string(target.join("settings.json")).unwrap(),
            "current"
        );
    }
}
