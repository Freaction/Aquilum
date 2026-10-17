use super::{HistoryService, NoteWrite};
use crate::history::milestone::Source;
use crate::history::rebuild::rebuild;
use crate::text::content::collect;
use crate::history::store::{history_dir, versions};
use std::fs;
use std::path::{Path, PathBuf};

mod moves;
mod read;

struct Fixture {
    _directory: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = crate::search::paths::canonical_path(directory.path());
        let vault = root.join("vault");
        fs::create_dir_all(&vault).unwrap();
        Self {
            data: root.join("data"),
            vault,
            _directory: directory,
        }
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![self.vault.clone()]
    }

    fn service(&self) -> HistoryService {
        HistoryService::new(&self.data)
    }

    fn save(&self, history: &HistoryService, note: &Path, text: &str, source: Source) {
        let original = history.original(&|| self.roots(), note);
        fs::create_dir_all(note.parent().unwrap()).unwrap();
        fs::write(note, text).unwrap();
        let hash = crate::files::document::hash_bytes(text.as_bytes());
        history.saved(&|| self.roots(), note, NoteWrite { text: &text, hash: &hash, source, name: None, change: None, original });
    }

    fn history_of(&self, relative: &str) -> PathBuf {
        history_dir(&self.vault, Path::new(relative))
    }
}

fn texts(dir: &Path) -> Vec<(String, Source)> {
    versions(dir)
        .into_iter()
        .map(|version| (text_of(dir, &version.id), version.source))
        .collect()
}

fn text_of(dir: &Path, id: &str) -> String {
    collect(&rebuild(dir, id).unwrap())
}

fn write<'a>(text: &'a &'a str, source: Source, hash: &'a str, name: Option<&'a str>) -> NoteWrite<'a> {
    NoteWrite { text, hash, source, name, change: None, original: None }
}

#[test]
fn typing_creates_baseline_and_coalesces_subsequent_typing() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let note = fixture.vault.join("Идея.md");
    fs::write(&note, "было").unwrap();

    fixture.save(&history, &note, "было и", Source::Me);
    fixture.save(&history, &note, "было и стало", Source::Me);

    assert_eq!(
        texts(&fixture.history_of("Идея.md")),
        [("было".to_owned(), Source::Start), ("было и стало".to_owned(), Source::Me)]
    );
}

#[test]
fn every_agent_edit_becomes_a_version() {
    let fixture = Fixture::new();
    let history = fixture.service();
    let note = fixture.vault.join("Агент.md");

    fixture.save(&history, &note, "от агента", Source::Agent);
    fixture.save(&history, &note, "от агента и снова", Source::Agent);

    assert_eq!(
        texts(&fixture.history_of("Агент.md")),
        [("от агента".to_owned(), Source::Agent), ("от агента и снова".to_owned(), Source::Agent)]
    );
}

#[test]
fn a_restart_keeps_what_was_on_disk_as_the_starting_text() {
    let fixture = Fixture::new();
    let note = fixture.vault.join("Заметка.md");
    let first = fixture.service();
    fixture.save(&first, &note, "первая", Source::Agent);
    fixture.save(&first, &note, "до закрытия", Source::Agent);

    fs::write(&note, "правка снаружи").unwrap();
    let second = fixture.service();
    fixture.save(&second, &note, "правка снаружи и моя", Source::Me);

    assert_eq!(
        texts(&fixture.history_of("Заметка.md")),
        [
            ("первая".to_owned(), Source::Agent),
            ("до закрытия".to_owned(), Source::Agent),
            ("правка снаружи и моя".to_owned(), Source::Me)
        ]
    );
}

#[test]
fn the_device_id_is_kept_between_launches() {
    let fixture = Fixture::new();
    let first = fixture.service();
    let second = fixture.service();
    assert_eq!(first.device, second.device);
    assert_eq!(first.device.len(), 8);
}
