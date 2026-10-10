use super::milestone::{Snapshot, Source};
use crate::files::document::{read_text, write_file_atomic_impl};
use crate::files::error::FileCommandError;
use crate::files::relocate::{remove_empty_folders, remove_files};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub const AQUILUM_FOLDER: &str = ".aquilum";
const QUANTUM_FOLDER: &str = ".quantum";
const HISTORY_FOLDER: &str = "history";
const VERSION_EXTENSION: &str = ".md";
const NAME_EXTENSION: &str = ".name";
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionFile {
    pub id: String,
    pub at_ms: u64,
    pub device: String,
    pub source: Source,
    pub from_ms: Option<u64>,
    pub name: Option<String>,
    pub is_current: bool,
}

fn history_root(base: &Path) -> PathBuf {
    base.join(AQUILUM_FOLDER).join(HISTORY_FOLDER)
}

pub fn adopt_quantum_history(base: &Path) {
    let legacy = base.join(QUANTUM_FOLDER);
    let legacy_history = legacy.join(HISTORY_FOLDER);
    if !legacy_history.is_dir() {
        return;
    }
    let target = history_root(base);
    let files = WalkDir::new(&legacy_history)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    for file in files {
        let Ok(relative) = file.strip_prefix(&legacy_history) else {
            continue;
        };
        let destination = target.join(relative);
        if destination.exists() {
            continue;
        }
        let moved = destination
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::rename(&file, &destination));
        if let Err(error) = moved {
            eprintln!("[aquilum:history] версия не перенесена из {}: {error}", file.display());
        }
    }
    remove_empty_folders(&legacy);
    let _ = fs::remove_dir(&legacy);
}

pub fn history_dir(base: &Path, relative: &Path) -> PathBuf {
    history_root(base).join(relative)
}

pub fn write_version(
    dir: &Path,
    device: &str,
    snapshot: &Snapshot,
) -> Result<PathBuf, FileCommandError> {
    let label = snapshot.source.file_label();
    let path = free_name(dir, millis(snapshot.at), |at| {
        format!("{at:013}_{device}_{label}{VERSION_EXTENSION}")
    });
    fs::create_dir_all(dir)?;
    write_file_atomic_impl(&path, &snapshot.text, None)?;
    Ok(path)
}

pub fn versions(dir: &Path) -> Vec<VersionFile> {
    let mut versions = files(dir)
        .filter_map(|(file_name, _)| parse_version(file_name))
        .collect::<Vec<_>>();
    versions.sort_by(|left, right| {
        (
            left.at_ms,
            &left.device,
            left.source != Source::Start,
            &left.id,
        )
            .cmp(&(
                right.at_ms,
                &right.device,
                right.source != Source::Start,
                &right.id,
            ))
    });
    versions
}

pub fn read_version(dir: &Path, version: &VersionFile) -> Option<String> {
    read_text(&dir.join(&version.id)).ok()
}

pub fn has_text(dir: &Path, version: &VersionFile, text: &str) -> bool {
    let path = dir.join(&version.id);
    fs::metadata(&path).is_ok_and(|metadata| metadata.len() == text.len() as u64)
        && read_text(&path).is_ok_and(|stored| stored == text)
}

#[cfg(test)]
pub fn latest_text(dir: &Path) -> Option<String> {
    versions(dir).last().and_then(|version| read_version(dir, version))
}

pub fn version_at(id: &str) -> Option<u64> {
    parse_version(id.to_owned()).map(|version| version.at_ms)
}

pub fn update_version_text(
    dir: &Path,
    version_id: &str,
    text: &str,
) -> Result<(), FileCommandError> {
    write_file_atomic_impl(&dir.join(version_id), text, None).map(|_| ())
}

pub fn remove_version_and_names(dir: &Path, version_id: &str) {
    let _ = fs::remove_file(dir.join(version_id));
    for (file_name, path) in files(dir) {
        if let Some((version, _, _)) = name_mark(&file_name) {
            if version == version_id {
                let _ = fs::remove_file(path);
            }
        }
    }
}

pub fn system_time_from_ms(millis: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(millis)
}

pub fn write_name(
    dir: &Path,
    device: &str,
    version_id: &str,
    name: &str,
    at: SystemTime,
) -> Result<(), FileCommandError> {
    let path = free_name(dir, millis(at), |at| {
        format!("{version_id}.{at:013}_{device}{NAME_EXTENSION}")
    });
    write_file_atomic_impl(&path, name.trim(), None).map(|_| ())
}

pub fn names(dir: &Path) -> HashMap<String, String> {
    let mut marks = files(dir)
        .filter_map(|(file_name, path)| {
            let (version, at, device) = name_mark(&file_name)?;
            Some((version.to_owned(), at, device.to_owned(), path))
        })
        .collect::<Vec<_>>();
    marks.sort();
    marks
        .into_iter()
        .map(|(version, _, _, path)| (version, path))
        .collect::<HashMap<_, _>>()
        .into_iter()
        .filter_map(|(version, path)| {
            let name = read_text(&path).ok()?.trim().to_owned();
            (!name.is_empty()).then_some((version, name))
        })
        .collect()
}

pub fn remove_expired(vault: &Path, retention_days: u32) -> u64 {
    let root = history_root(vault);
    if retention_days == 0 || !root.is_dir() {
        return 0;
    }
    let cutoff = millis(SystemTime::now()).saturating_sub(millis(UNIX_EPOCH + DAY * retention_days));
    let mut named = HashMap::<PathBuf, HashMap<String, String>>::new();
    let expired = WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let version = parse_version(entry.file_name().to_string_lossy().into_owned())?;
            let labels = named
                .entry(entry.path().parent()?.to_path_buf())
                .or_insert_with_key(|dir| names(dir));
            (version.at_ms < cutoff && !labels.contains_key(&version.id)).then(|| entry.into_path())
        })
        .collect::<Vec<_>>();
    let removed = remove_files(&expired);
    let orphans = named.keys().flat_map(|dir| orphan_names(dir)).collect::<Vec<_>>();
    remove_files(&orphans);
    remove_empty_folders(&root);
    removed
}

fn orphan_names(dir: &Path) -> Vec<PathBuf> {
    files(dir)
        .filter_map(|(file_name, path)| {
            let (version, _, _) = name_mark(&file_name)?;
            (!dir.join(version).exists()).then_some(path)
        })
        .collect()
}

fn name_mark(file_name: &str) -> Option<(&str, u64, &str)> {
    let (version, stamp) = file_name.strip_suffix(NAME_EXTENSION)?.rsplit_once('.')?;
    let (at, device) = stamp.split_once('_')?;
    Some((version, at.parse().ok()?, device))
}

fn files(dir: &Path) -> impl Iterator<Item = (String, PathBuf)> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| (entry.file_name().to_string_lossy().into_owned(), entry.path()))
}

fn parse_version(name: String) -> Option<VersionFile> {
    let (at_ms, device, source) = {
        let mut parts = name.strip_suffix(VERSION_EXTENSION)?.splitn(3, '_');
        let at_ms = parts.next()?.parse().ok()?;
        let device = parts.next()?.to_owned();
        (at_ms, device, Source::from_label(parts.next()?)?)
    };
    Some(VersionFile {
        id: name,
        at_ms,
        device,
        from_ms: source.origin_ms(),
        source,
        name: None,
        is_current: false,
    })
}

fn millis(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}

fn free_name(dir: &Path, at_ms: u64, name: impl Fn(u64) -> String) -> PathBuf {
    (at_ms..)
        .map(|at| dir.join(name(at)))
        .find(|candidate| !candidate.exists())
        .unwrap_or_else(|| dir.join(name(at_ms)))
}

#[cfg(test)]
mod tests {
    use super::{
        adopt_quantum_history, history_dir, history_root, latest_text, names, remove_expired,
        versions, write_name, write_version, DAY,
    };
    use crate::history::milestone::{Snapshot, Source};
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn snapshot(text: &str, source: Source, seconds: u64) -> Snapshot {
        Snapshot {
            text: text.to_owned(),
            source,
            at: UNIX_EPOCH + Duration::from_secs(seconds),
        }
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
        write_version(&dir, "aaaa", &snapshot("второй", Source::Agent, 20)).unwrap();
        write_version(&dir, "aaaa", &snapshot("первый", Source::Me, 10)).unwrap();
        let clash = write_version(&dir, "aaaa", &snapshot("тот же миг", Source::Agent, 20)).unwrap();

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
        let snapshot = |text: &str, source| Snapshot {
            text: text.to_owned(),
            source,
            at,
        };
        write_version(&dir, "bbbb", &snapshot("старт", Source::Start)).unwrap();
        write_version(&dir, "bbbb", &snapshot("правка", Source::Me)).unwrap();
        write_version(&dir, "aaaa", &snapshot("другое устройство", Source::Agent)).unwrap();

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
        let written = write_version(&dir, "aaaa", &snapshot("старое", Source::Restore(10_000), 20)).unwrap();

        let found = versions(&dir);

        assert!(written.to_string_lossy().ends_with("_aaaa_restore-0000000010000.md"));
        assert_eq!((found[0].source, found[0].from_ms), (Source::Restore(10_000), Some(10_000)));
    }

    #[test]
    fn the_latest_name_of_a_version_wins_and_an_empty_one_removes_it() {
        let directory = tempfile::tempdir().unwrap();
        let dir = history_dir(directory.path(), Path::new("Идея.md"));
        let id = write_version(&dir, "aaaa", &snapshot("текст", Source::Me, 10)).unwrap();
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
        let named = write_version(&dir, "aaaa", &snapshot("важная", Source::Me, old)).unwrap();
        let plain = write_version(&dir, "aaaa", &snapshot("обычная", Source::Me, old + 1)).unwrap();
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
        let first = write_version(&dir, "aaaa", &snapshot("с первого", Source::Me, 10)).unwrap();
        let second = write_version(&dir, "bbbb", &snapshot("со второго", Source::Me, 10)).unwrap();
        assert_ne!(first, second);
        assert_eq!(versions(&dir).len(), 2);
    }

    #[test]
    fn retention_removes_versions_older_than_the_limit_by_their_own_time() {
        let directory = tempfile::tempdir().unwrap();
        let old_note = history_dir(directory.path(), Path::new("Архив/Старая.md"));
        let fresh_note = history_dir(directory.path(), Path::new("Свежая.md"));
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        write_version(&old_note, "aaaa", &snapshot("давно", Source::Me, now - 40 * DAY.as_secs())).unwrap();
        write_version(&fresh_note, "aaaa", &snapshot("давно", Source::Me, now - 40 * DAY.as_secs())).unwrap();
        write_version(&fresh_note, "aaaa", &snapshot("вчера", Source::Me, now - DAY.as_secs())).unwrap();
        fs::write(fresh_note.join("заметки.txt"), "чужой файл").unwrap();

        assert_eq!(remove_expired(directory.path(), 0), 0, "ноль — хранить всегда");
        assert_eq!(remove_expired(directory.path(), 30), 2);

        assert!(!history_root(directory.path()).join("Архив").exists(), "опустевшие папки убраны");
        assert_eq!(latest_text(&fresh_note).as_deref(), Some("вчера"));
        assert!(fresh_note.join("заметки.txt").exists(), "трогаются только файлы версий");
    }
}
