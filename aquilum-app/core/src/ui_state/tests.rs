use super::database::UiStateDatabase;
use super::models::{
    GraphCameraState, OpenSessionInput, SaveStateBatchInput, SessionStateInput, TabKind, TabState,
    ViewStateInput,
};
use uuid::Uuid;

pub fn setup() -> (UiStateDatabase, Uuid, Uuid, Uuid) {
    let mut database = UiStateDatabase::memory().expect("database");
    let workspace_id = database
        .resolve_workspace("C:/notes", 1)
        .expect("workspace");
    let document_id = database
        .resolve_document(workspace_id, "note.md")
        .expect("document");
    let epoch = Uuid::new_v4();
    database
        .open_session(&OpenSessionInput {
            workspace_id,
            window_id: "main".to_owned(),
            epoch,
            now_ms: 2,
        })
        .expect("session");
    (database, workspace_id, document_id, epoch)
}

#[test]
fn lists_known_workspaces_most_recently_seen_first() {
    let (mut database, _workspace_id, _document_id, _epoch) = setup();
    database
        .resolve_workspace("C:/archive", 9)
        .expect("archive");
    database.resolve_workspace("C:/drafts", 5).expect("drafts");

    let paths = database
        .list_workspaces()
        .expect("workspaces")
        .into_iter()
        .map(|workspace| workspace.path)
        .collect::<Vec<_>>();

    assert_eq!(paths, ["C:/archive", "C:/drafts", "C:/notes"]);
}

#[test]
fn forgetting_a_workspace_removes_it_from_the_list() {
    let (mut database, workspace_id, _document_id, _epoch) = setup();
    database.resolve_workspace("C:/drafts", 5).expect("drafts");

    database.forget_workspace(workspace_id).expect("forget");

    let paths = database
        .list_workspaces()
        .expect("workspaces")
        .into_iter()
        .map(|workspace| workspace.path)
        .collect::<Vec<_>>();

    assert_eq!(paths, ["C:/drafts"]);
}

#[test]
fn listed_paths_drop_the_windows_verbatim_prefix() {
    use super::paths::display_workspace;

    assert_eq!(
        display_workspace(r"\\?\D:\База знаний копия\NeuroNet"),
        r"D:\База знаний копия\NeuroNet"
    );
    assert_eq!(
        display_workspace(r"\\?\UNC\server\share"),
        r"\\server\share"
    );
    assert_eq!(display_workspace("/home/user/notes"), "/home/user/notes");
}

fn view(document_id: Uuid, position: i64) -> ViewStateInput {
    ViewStateInput {
        path: None,
        document_id,
        pane_id: "main".to_owned(),
        cursor_anchor: vec![1, 2],
        cursor_head: vec![1, 3],
        fallback_anchor: position,
        fallback_head: position,
        scroll_anchor: vec![4, 5],
        fallback_scroll_anchor: position,
        scroll_offset_px: 12.5,
        focused_surface: "body".to_owned(),
    }
}

pub fn batch(
    workspace_id: Uuid,
    document_id: Uuid,
    epoch: Uuid,
    sequence: i64,
    position: i64,
) -> SaveStateBatchInput {
    let tab_id = Uuid::new_v4();
    SaveStateBatchInput {
        workspace_id,
        window_id: "main".to_owned(),
        epoch,
        sequence,
        now_ms: sequence + 10,
        session: Some(SessionStateInput {
            active_tab_id: Some(tab_id),
            tabs: Some(vec![TabState {
                tab_id,
                document_id: Some(document_id),
                kind: TabKind::Document,
                relative_path: None,
                position: 0,
            }]),
        }),
        views: vec![view(document_id, position)],
        graph_camera: None,
    }
}

#[test]
fn stale_sequence_cannot_replace_newer_state() {
    let (mut database, workspace_id, document_id, epoch) = setup();
    assert!(database
        .save_batch(&batch(workspace_id, document_id, epoch, 2, 20))
        .expect("new batch"));
    assert!(!database
        .save_batch(&batch(workspace_id, document_id, epoch, 1, 10))
        .expect("stale batch"));
    let loaded = database.load_session(workspace_id, "main").expect("load");
    assert_eq!(loaded.views[0].fallback_anchor, 20);
}

#[test]
fn previous_epoch_cannot_write_after_session_reopens() {
    let (mut database, workspace_id, document_id, old_epoch) = setup();
    let new_epoch = Uuid::new_v4();
    database
        .open_session(&OpenSessionInput {
            workspace_id,
            window_id: "main".to_owned(),
            epoch: new_epoch,
            now_ms: 20,
        })
        .expect("reopen");
    assert!(!database
        .save_batch(&batch(workspace_id, document_id, old_epoch, 99, 99))
        .expect("old epoch"));
    assert!(database
        .save_batch(&batch(workspace_id, document_id, new_epoch, 1, 30))
        .expect("new epoch"));
}

#[test]
fn missing_document_is_retained_until_cutoff_then_cascades() {
    let (mut database, workspace_id, document_id, epoch) = setup();
    database
        .save_batch(&batch(workspace_id, document_id, epoch, 1, 20))
        .expect("batch");
    database
        .mark_document_missing(document_id, 100)
        .expect("missing");
    assert_eq!(database.purge_missing(99, 500).expect("early purge"), 0);
    let replacement = database
        .resolve_document(workspace_id, "note.md")
        .expect("replacement");
    assert_ne!(replacement, document_id);
    assert_eq!(database.purge_missing(100, 500).expect("purge"), 1);
    let loaded = database.load_session(workspace_id, "main").expect("load");
    assert!(loaded.tabs.is_empty());
    assert!(loaded.views.is_empty());
    assert!(loaded.active_tab_id.is_none());
    let resolved = database
        .resolve_document(workspace_id, "note.md")
        .expect("resolve replacement");
    assert_eq!(resolved, replacement);
}

#[test]
fn rename_preserves_document_identity() {
    let (mut database, workspace_id, document_id, _) = setup();
    database
        .rename_document(document_id, "renamed.md")
        .expect("rename");
    let resolved = database
        .resolve_document(workspace_id, "renamed.md")
        .expect("resolve");
    assert_eq!(resolved, document_id);
}

#[test]
fn zero_cleanup_limit_does_not_delete_tombstone() {
    let (mut database, _, document_id, _) = setup();
    database
        .mark_document_missing(document_id, 100)
        .expect("missing");
    assert_eq!(database.purge_missing(100, 0).expect("purge"), 0);
}

#[test]
fn session_survives_database_reopen() {
    let directory = tempfile::tempdir().expect("directory");
    let path = directory.path().join("ui-state.sqlite3");
    let workspace_id;
    let document_id;
    {
        let mut database = UiStateDatabase::open(&path).expect("open");
        workspace_id = database
            .resolve_workspace("C:/notes", 1)
            .expect("workspace");
        document_id = database
            .resolve_document(workspace_id, "note.md")
            .expect("document");
        let epoch = Uuid::new_v4();
        database
            .open_session(&OpenSessionInput {
                workspace_id,
                window_id: "main".to_owned(),
                epoch,
                now_ms: 2,
            })
            .expect("session");
        database
            .save_batch(&batch(workspace_id, document_id, epoch, 1, 42))
            .expect("save");
    }
    let database = UiStateDatabase::open(&path).expect("reopen");
    let loaded = database.load_session(workspace_id, "main").expect("load");
    assert_eq!(loaded.tabs[0].document_id, Some(document_id));
    assert_eq!(loaded.tabs[0].relative_path.as_deref(), Some("note.md"));
    assert_eq!(loaded.views[0].fallback_anchor, 42);
    assert_eq!(loaded.views[0].fallback_scroll_anchor, 42);
}

#[test]
fn the_graph_camera_survives_a_reopened_session() {
    let (mut database, workspace_id, document_id, epoch) = setup();
    let mut input = batch(workspace_id, document_id, epoch, 1, 0);
    input.graph_camera = Some(GraphCameraState {
        center_x: -3.5,
        center_y: 7.25,
        scale: 42.0,
    });
    assert!(database.save_batch(&input).expect("saved"));

    let reopened = database
        .open_session(&OpenSessionInput {
            workspace_id,
            window_id: "main".to_owned(),
            epoch: Uuid::new_v4(),
            now_ms: 20,
        })
        .expect("session");

    assert_eq!(
        reopened.graph_camera,
        Some(GraphCameraState {
            center_x: -3.5,
            center_y: 7.25,
            scale: 42.0,
        })
    );
}

#[test]
fn a_session_without_a_graph_camera_reports_none() {
    let (mut database, workspace_id, document_id, epoch) = setup();
    assert!(database
        .save_batch(&batch(workspace_id, document_id, epoch, 1, 0))
        .expect("saved"));

    let reopened = database
        .open_session(&OpenSessionInput {
            workspace_id,
            window_id: "main".to_owned(),
            epoch: Uuid::new_v4(),
            now_ms: 20,
        })
        .expect("session");

    assert_eq!(reopened.graph_camera, None);
}

#[test]
fn base_view_selection_survives_reopen_and_is_scoped_and_clamped() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state.sqlite3");
    let mut database = UiStateDatabase::open(&path).unwrap();
    let workspace = database.resolve_workspace("first", 1).unwrap();
    let other = database.resolve_workspace("second", 1).unwrap();
    database
        .save_base_view(workspace, "Boards/tasks.base", 4)
        .unwrap();
    assert_eq!(
        database
            .load_base_view(workspace, "Boards/tasks.base", 7)
            .unwrap(),
        4
    );
    assert_eq!(
        database
            .load_base_view(workspace, "Boards/tasks.base", 2)
            .unwrap(),
        1
    );
    assert_eq!(
        database
            .load_base_view(workspace, "Boards/tasks.base", 0)
            .unwrap(),
        0
    );
    assert_eq!(
        database
            .load_base_view(workspace, "Boards/other.base", 7)
            .unwrap(),
        0
    );
    assert_eq!(
        database
            .load_base_view(other, "Boards/tasks.base", 7)
            .unwrap(),
        0
    );
    for invalid in ["../tasks.base", "tasks.md", "", "/tasks.base"] {
        assert!(database.save_base_view(workspace, invalid, 0).is_err());
        assert!(database.load_base_view(workspace, invalid, 2).is_err());
    }
    drop(database);
    let database = UiStateDatabase::open(&path).unwrap();
    assert_eq!(
        database
            .load_base_view(workspace, "Boards/tasks.base", 7)
            .unwrap(),
        4
    );
}

#[test]
fn base_tabs_round_trip_without_creating_document_identity() {
    let (mut database, workspace_id, document_id, epoch) = setup();
    let mut input = batch(workspace_id, document_id, epoch, 1, 0);
    let tab = &mut input.session.as_mut().unwrap().tabs.as_mut().unwrap()[0];
    tab.kind = TabKind::Base;
    tab.document_id = None;
    tab.relative_path = Some("Boards/tasks.BASE".into());
    input.views.clear();
    super::validation::validate_batch(&input).unwrap();
    assert!(database.save_batch(&input).unwrap());
    let loaded = database.load_session(workspace_id, "main").unwrap();
    assert_eq!(loaded.tabs[0].kind, TabKind::Base);
    assert_eq!(loaded.tabs[0].document_id, None);
    assert_eq!(
        loaded.tabs[0].relative_path.as_deref(),
        Some("Boards/tasks.BASE")
    );
    let count: i64 = database
        .connection
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    let tab = &mut input.session.as_mut().unwrap().tabs.as_mut().unwrap()[0];
    tab.document_id = Some(document_id);
    assert!(super::validation::validate_batch(&input).is_err());
    let tab = &mut input.session.as_mut().unwrap().tabs.as_mut().unwrap()[0];
    tab.document_id = None;
    tab.relative_path = None;
    assert!(super::validation::validate_batch(&input).is_err());
}
