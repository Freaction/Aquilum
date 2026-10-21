use super::{Core, CoreEvent, EventSink};
use crate::files::gate;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Recording(Mutex<Vec<CoreEvent>>);

impl EventSink for Recording {
    fn emit(&self, event: CoreEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl Recording {
    fn names(&self) -> Vec<&'static str> {
        self.0.lock().unwrap().iter().map(name).collect()
    }

    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

fn name(event: &CoreEvent) -> &'static str {
    match event {
        CoreEvent::NotesRelocated(_) => "notes-relocated",
        CoreEvent::NoteHistoryChanged(_) => "note-history-changed",
        CoreEvent::WorkspaceChanged(_) => "workspace-changed",
        CoreEvent::ContentChanged(_) => "content-changed",
        CoreEvent::WorkspaceRescanned => "workspace-rescanned",
        _ => "other",
    }
}

fn open_core() -> (tempfile::TempDir, Arc<Core>, Arc<Recording>) {
    let directory = tempfile::tempdir().unwrap();
    let events = Arc::new(Recording::default());
    let core = Core::open(&directory.path().join("data"), Arc::clone(&events) as Arc<dyn EventSink>);
    (directory, core, events)
}

#[test]
fn trashing_a_note_announces_the_removal() {
    let (directory, core, events) = open_core();
    let workspace = directory.path().join("База");
    fs::create_dir_all(&workspace).unwrap();
    let note = workspace.join("Удалить.md");
    fs::write(&note, "текст\n").unwrap();

    gate::trash(&core, &workspace, &note).unwrap();

    assert!(events.names().contains(&"notes-relocated"), "{:?}", events.names());
    assert!(!Path::new(&note).exists());
}

#[test]
fn a_rename_announces_the_move_before_the_old_path_disappears_from_the_listing() {
    let (directory, core, events) = open_core();
    let workspace = directory.path().join("База");
    fs::create_dir_all(&workspace).unwrap();
    let note = workspace.join("Без названия.md");
    fs::write(&note, "текст\n").unwrap();
    core.search.prepare(&workspace.to_string_lossy()).unwrap();
    events.clear();

    gate::rename(&core, &note, &workspace.join("Новая.md")).unwrap();

    let names = events.names();
    let moved = names.iter().position(|name| *name == "notes-relocated");
    let listed = names.iter().position(|name| *name == "workspace-changed");
    assert!(moved.is_some() && listed.is_some() && moved < listed, "{names:?}");
}

#[test]
fn watch_batches_reach_the_host_as_the_event_it_can_act_on() {
    use crate::files::watcher::{WatchBatch, WatchScope};
    use std::path::PathBuf;

    let (_directory, core, events) = open_core();
    let batch = |paths: &[&str], scope| WatchBatch::new(paths.iter().map(PathBuf::from).collect(), scope);

    core.ingest_watch(batch(&["a.md"], WatchScope::Content));
    core.ingest_watch(batch(&["b.md"], WatchScope::Structure));
    core.ingest_watch(batch(&["картинка.png"], WatchScope::Listing));
    core.ingest_watch(batch(&["Папка"], WatchScope::Rescan));
    core.ingest_watch(batch(&[], WatchScope::Rescan));

    assert_eq!(events.names(), ["content-changed", "workspace-changed", "workspace-changed", "workspace-changed", "workspace-rescanned"]);
}
