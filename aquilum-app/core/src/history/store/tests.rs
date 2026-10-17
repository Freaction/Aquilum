use super::legacy::adopt_quantum_history;
use super::retention::DAY;
use super::{history_dir, history_root, names, remove_expired, version_path, versions, write_name, write_version};
use crate::history::milestone::Source;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

fn latest_text(dir: &Path) -> Option<String> {
    versions(dir).last().and_then(|version| fs::read_to_string(dir.join(&version.id)).ok())
}

#[test]
fn history_kept_under_the_quantum_name_moves_to_aquilum_without_overwriting() {
    let vault = tempfile::tempdir().unwrap();
    let legacy = vault.path().join(".quantum/history/Идея.md");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("1_dev_me.md"), "старая").unwrap();
    fs::write(legacy.join("2_dev_me.md"), "из quantum").unwrap();
    let current = history_dir(vault.path(), Path::new("Идея.md"));
    fs::create_dir_all(&current).unwrap();
    fs::write(current.join("2_dev_me.md"), "уже в aquilum").unwrap();

    adopt_quantum_history(vault.path());

    assert_eq!(fs::read_to_string(current.join("1_dev_me.md")).unwrap(), "старая");
    assert_eq!(fs::read_to_string(current.join("2_dev_me.md")).unwrap(), "уже в aquilum");
    assert!(legacy.join("2_dev_me.md").exists());
    assert!(!legacy.join("1_dev_me.md").exists());
}

#[test]
fn versions_written_in_one_millisecond_keep_their_order() {
    let dir = tempfile::tempdir().unwrap();
    let at = UNIX_EPOCH + Duration::from_millis(5);
    let first = version_path(dir.path(), "dev", Source::Agent, at, ".md");
    fs::write(&first, "первая").unwrap();
    let second = version_path(dir.path(), "dev", Source::Agent, at, ".delta");
    fs::write(&second, "вторая").unwrap();
    let ids = versions(dir.path()).into_iter().map(|version| version.id).collect::<Vec<_>>();
    assert_eq!(ids, [first, second].map(|path| path.file_name().unwrap().to_string_lossy().into_owned()));
}

#[test]
fn the_history_folder_repeats_the_note_path() {
    let vault = Path::new("C:/База");
    assert_eq!(
        history_dir(vault, Path::new("Проекты/Идея.md")),
        vault.join(".aquilum").join("history").join("Проекты/Идея.md")
    );
}

#[test]
fn versions_are_separate_files_ordered_by_time_and_never_overwritten() {
    let directory = tempfile::tempdir().unwrap();
    let dir = history_dir(directory.path(), Path::new("Идея.md"));
    write_version(&dir, "aaaa", Source::Agent, at(20), &"второй").unwrap();
    write_version(&dir, "aaaa", Source::Me, at(10), &"первый").unwrap();
    let clash = write_version(&dir, "aaaa", Source::Agent, at(20), &"тот же миг").unwrap();

    let found = versions(&dir);

    assert_eq!(found.iter().map(|version| version.source).collect::<Vec<_>>(), [Source::Me, Source::Agent, Source::Agent]);
    assert_eq!(dir.join(&found[2].id), clash);
    assert_eq!(found[2].at_ms, 20_001, "совпавшее имя сдвигает миллисекунду, а не перезаписывает файл");
    assert_eq!(latest_text(&dir).as_deref(), Some("тот же миг"));
}

#[test]
fn same_millisecond_start_precedes_edits_without_changing_device_order() {
    let directory = tempfile::tempdir().unwrap();
    let dir = history_dir(directory.path(), Path::new("Идея.md"));
    let at = UNIX_EPOCH + Duration::from_millis(20_000);
    write_version(&dir, "bbbb", Source::Start, at, &"старт").unwrap();
    write_version(&dir, "bbbb", Source::Me, at, &"правка").unwrap();
    write_version(&dir, "aaaa", Source::Agent, at, &"другое устройство").unwrap();

    let found = versions(&dir);

    assert_eq!(
        found.iter().map(|version| version.source).collect::<Vec<_>>(),
        [Source::Agent, Source::Start, Source::Me]
    );
    assert_eq!(latest_text(&dir).as_deref(), Some("правка"));
}

#[test]
fn a_restored_version_keeps_the_time_of_its_original() {
    let directory = tempfile::tempdir().unwrap();
    let dir = history_dir(directory.path(), Path::new("Идея.md"));
    let written = write_version(&dir, "aaaa", Source::Restore(10_000), at(20), &"старое").unwrap();

    let found = versions(&dir);

    assert!(written.to_string_lossy().ends_with("_aaaa_restore-0000000010000.md"));
    assert_eq!((found[0].source, found[0].from_ms), (Source::Restore(10_000), Some(10_000)));
}

#[test]
fn the_latest_name_of_a_version_wins_and_an_empty_one_removes_it() {
    let directory = tempfile::tempdir().unwrap();
    let dir = history_dir(directory.path(), Path::new("Идея.md"));
    let id = write_version(&dir, "aaaa", Source::Me, at(10), &"текст").unwrap();
    let id = id.file_name().unwrap().to_string_lossy().into_owned();
    let at = |seconds| UNIX_EPOCH + Duration::from_secs(seconds);

    write_name(&dir, "aaaa", &id, " Черновик ", at(20)).unwrap();
    write_name(&dir, "bbbb", &id, "До рефакторинга", at(30)).unwrap();
    assert_eq!(names(&dir).get(&id).map(String::as_str), Some("До рефакторинга"));
    assert!(versions(&dir).len() == 1, "файл названия — не версия");

    write_name(&dir, "aaaa", &id, "", at(40)).unwrap();
    assert!(names(&dir).is_empty());
}

#[test]
fn retention_keeps_named_versions_and_drops_names_of_removed_ones() {
    let directory = tempfile::tempdir().unwrap();
    let dir = history_dir(directory.path(), Path::new("Идея.md"));
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let old = now - 40 * DAY.as_secs();
    let named = write_version(&dir, "aaaa", Source::Me, at(old), &"важная").unwrap();
    let plain = write_version(&dir, "aaaa", Source::Me, at(old + 1), &"обычная").unwrap();
    let id = |path: &Path| path.file_name().unwrap().to_string_lossy().into_owned();
    write_name(&dir, "aaaa", &id(&named), "Важная", UNIX_EPOCH + Duration::from_secs(old)).unwrap();
    write_name(&dir, "aaaa", &id(&plain), "", UNIX_EPOCH + Duration::from_secs(old)).unwrap();

    assert_eq!(remove_expired(directory.path(), 30), 1);

    assert!(named.exists() && !plain.exists());
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 2, "осталась версия и её название");
}

#[test]
fn two_devices_never_write_the_same_file() {
    let directory = tempfile::tempdir().unwrap();
    let dir = directory.path().join("Идея.md");
    let first = write_version(&dir, "aaaa", Source::Me, at(10), &"с первого").unwrap();
    let second = write_version(&dir, "bbbb", Source::Me, at(10), &"со второго").unwrap();
    assert_ne!(first, second);
    assert_eq!(versions(&dir).len(), 2);
}

#[test]
fn retention_removes_versions_older_than_the_limit_by_their_own_time() {
    let directory = tempfile::tempdir().unwrap();
    let old_note = history_dir(directory.path(), Path::new("Архив/Старая.md"));
    let fresh_note = history_dir(directory.path(), Path::new("Свежая.md"));
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    write_version(&old_note, "aaaa", Source::Me, at(now - 40 * DAY.as_secs()), &"давно").unwrap();
    write_version(&fresh_note, "aaaa", Source::Me, at(now - 40 * DAY.as_secs()), &"давно").unwrap();
    write_version(&fresh_note, "aaaa", Source::Me, at(now - DAY.as_secs()), &"вчера").unwrap();
    fs::write(fresh_note.join("заметки.txt"), "чужой файл").unwrap();

    assert_eq!(remove_expired(directory.path(), 0), 0, "ноль — хранить всегда");
    assert_eq!(remove_expired(directory.path(), 30), 2);

    assert!(!history_root(directory.path()).join("Архив").exists(), "опустевшие папки убраны");
    assert_eq!(latest_text(&fresh_note).as_deref(), Some("вчера"));
    assert!(fresh_note.join("заметки.txt").exists(), "трогаются только файлы версий");
}
