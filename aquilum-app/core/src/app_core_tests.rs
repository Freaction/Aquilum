use super::{Core, CoreEvent, EventSink};
use crate::documents::session::SYSTEM_CLIENT;
use crate::documents::DocumentEvent;
use crate::files::gate;
use crate::history::Source;
use crate::mcp::bridge::{Disposition, Navigation};
use serde_json::json;
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
        self.0.lock().unwrap().iter().map(CoreEvent::name).collect()
    }

    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

fn open_core() -> (tempfile::TempDir, Arc<Core>, Arc<Recording>) {
    let directory = tempfile::tempdir().unwrap();
    let events = Arc::new(Recording::default());
    let core = Core::open(
        &directory.path().join("data"),
        Arc::clone(&events) as Arc<dyn EventSink>,
    );
    (directory, core, events)
}

#[test]
fn the_frontend_receives_the_same_event_names_and_payloads_as_before() {
    let saved = CoreEvent::DocumentSaved(DocumentEvent {
        path: "a.md".into(),
        version: 3,
    });
    assert_eq!(saved.name(), "document-saved");
    assert_eq!(
        serde_json::to_value(&saved).unwrap(),
        json!({ "path": "a.md", "version": 3 })
    );

    let note = CoreEvent::Navigate(Navigation::Note {
        path: "a.md".into(),
        disposition: Disposition::NewTab,
    });
    assert_eq!(note.name(), "mcp-navigate");
    assert_eq!(
        serde_json::to_value(&note).unwrap(),
        json!({ "kind": "note", "path": "a.md", "disposition": "new-tab" })
    );
    let workspace = CoreEvent::Navigate(Navigation::Workspace {
        path: "base".into(),
    });
    assert_eq!(
        serde_json::to_value(&workspace).unwrap(),
        json!({ "kind": "workspace", "path": "base" })
    );

    let changed = CoreEvent::WorkspaceChanged(vec!["a.md".into()]);
    assert_eq!(changed.name(), "workspace-changed");
    assert_eq!(serde_json::to_value(&changed).unwrap(), json!(["a.md"]));
}

#[test]
fn a_replaced_document_announces_the_change_before_the_save() {
    let (directory, core, events) = open_core();
    let note = directory.path().join("Заметка.md");
    fs::write(&note, "первая строка\n").unwrap();
    core.documents.open(&core, &note, 0).unwrap();
    events.clear();

    core.documents
        .replace_text(
            &core,
            &note,
            "первая строка\nвторая\n",
            SYSTEM_CLIENT,
            Source::Me,
            None,
        )
        .unwrap();

    assert_eq!(events.names(), ["document-changed", "document-saved"]);
    assert_eq!(
        fs::read_to_string(&note).unwrap(),
        "первая строка\nвторая\n"
    );
}

#[test]
fn a_document_whose_file_vanished_reports_it_missing_instead_of_saved() {
    let (directory, core, events) = open_core();
    let note = directory.path().join("Заметка.md");
    fs::write(&note, "текст\n").unwrap();
    core.documents.open(&core, &note, 0).unwrap();
    fs::remove_file(&note).unwrap();
    events.clear();

    core.documents
        .replace_text(
            &core,
            &note,
            "новый текст\n",
            SYSTEM_CLIENT,
            Source::Me,
            None,
        )
        .unwrap();

    let names = events.names();
    assert!(names.contains(&"document-missing"), "{names:?}");
    assert!(!names.contains(&"document-saved"), "{names:?}");
}

#[test]
fn trashing_a_note_announces_the_removal() {
    let (directory, core, events) = open_core();
    let workspace = directory.path().join("База");
    fs::create_dir_all(&workspace).unwrap();
    let note = workspace.join("Удалить.md");
    fs::write(&note, "текст\n").unwrap();

    gate::trash(&core, &workspace, &note).unwrap();

    assert!(
        events.names().contains(&"notes-relocated"),
        "{:?}",
        events.names()
    );
    assert!(!Path::new(&note).exists());
}

#[test]
fn base_content_writes_notify_the_host_to_refresh_the_board() {
    let (_directory, core, events) = open_core();
    core.ingest_watch(crate::files::watcher::WatchBatch::new(
        vec![std::path::PathBuf::from("Board.BASE")],
        crate::files::watcher::WatchScope::Content,
    ));
    assert_eq!(events.names(), vec!["workspace-changed"]);
}
