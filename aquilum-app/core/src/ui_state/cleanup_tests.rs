use super::models::OpenSessionInput;
use super::tests::batch;
use super::UiStateService;
use std::path::PathBuf;
use uuid::Uuid;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const NOTE: &str = "folder/note.md";

struct Vault {
    service: UiStateService,
    workspace_id: Uuid,
    document_id: Uuid,
    root: PathBuf,
    note: PathBuf,
    _dir: tempfile::TempDir,
}

fn vault() -> Vault {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("vault");
    std::fs::create_dir_all(root.join("folder")).unwrap();
    let note = root.join("folder").join("note.md");
    std::fs::write(&note, "").unwrap();
    let service = UiStateService::open(&dir.path().join("ui-state.sqlite3"));
    let workspace_id = service.resolve_workspace(&root.to_string_lossy(), 1).unwrap();
    let document_id = service.resolve_document(workspace_id, NOTE).unwrap();
    Vault { service, workspace_id, document_id, root, note, _dir: dir }
}

#[test]
fn a_note_deleted_from_disk_is_forgotten_after_retention() {
    let v = vault();
    let epoch = Uuid::new_v4();
    v.service.open_session(&OpenSessionInput { workspace_id: v.workspace_id, window_id: "main".to_owned(), epoch, now_ms: 2 }).unwrap();
    v.service.save_batch(&batch(v.workspace_id, v.document_id, epoch, 1, 20)).unwrap();
    std::fs::remove_file(&v.note).unwrap();
    assert_eq!(v.service.cleanup(30, DAY_MS).unwrap(), 0);
    assert!(v.service.load_view(v.workspace_id, "main", v.document_id, "main").unwrap().is_some());
    assert_eq!(v.service.cleanup(30, 31 * DAY_MS).unwrap(), 1);
    assert!(v.service.load_view(v.workspace_id, "main", v.document_id, "main").unwrap().is_none());
    assert_ne!(v.service.resolve_document(v.workspace_id, NOTE).unwrap(), v.document_id);
}

#[test]
fn a_note_that_exists_or_came_back_keeps_its_state() {
    let v = vault();
    assert_eq!(v.service.cleanup(0, DAY_MS).unwrap(), 0);
    std::fs::remove_file(&v.note).unwrap();
    v.service.cleanup(30, DAY_MS).unwrap();
    std::fs::write(&v.note, "").unwrap();
    assert_eq!(v.service.cleanup(30, 31 * DAY_MS).unwrap(), 0);
    assert_eq!(v.service.resolve_document(v.workspace_id, NOTE).unwrap(), v.document_id);
}

#[test]
fn documents_of_an_unreachable_workspace_are_kept() {
    let v = vault();
    std::fs::remove_dir_all(&v.root).unwrap();
    assert_eq!(v.service.cleanup(0, DAY_MS).unwrap(), 0);
    std::fs::create_dir_all(v.note.parent().unwrap()).unwrap();
    std::fs::write(&v.note, "").unwrap();
    assert_eq!(v.service.resolve_document(v.workspace_id, NOTE).unwrap(), v.document_id);
}
