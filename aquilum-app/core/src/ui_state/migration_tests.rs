use super::error::UiStateError;
use super::migrations::migrate;
use super::service::UiStateService;
use rusqlite::{params, Connection};
use uuid::Uuid;

const VERSION_ONE: &str = "CREATE TABLE workspaces (
         id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE, last_seen_ms INTEGER NOT NULL
     );
     CREATE TABLE documents (
         id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, relative_path TEXT NOT NULL,
         status TEXT NOT NULL DEFAULT 'active', missing_since_ms INTEGER,
         UNIQUE(workspace_id, relative_path), UNIQUE(workspace_id, id),
         FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
     );
     CREATE TABLE view_states (
         workspace_id TEXT NOT NULL, window_id TEXT NOT NULL, document_id TEXT NOT NULL,
         pane_id TEXT NOT NULL, cursor_anchor BLOB NOT NULL, cursor_head BLOB NOT NULL,
         fallback_anchor INTEGER NOT NULL, fallback_head INTEGER NOT NULL,
         scroll_anchor BLOB NOT NULL, scroll_offset_px REAL NOT NULL,
         focused_surface TEXT NOT NULL, updated_at_ms INTEGER NOT NULL,
         PRIMARY KEY(workspace_id, window_id, document_id, pane_id),
         FOREIGN KEY(workspace_id, document_id)
             REFERENCES documents(workspace_id, id) ON DELETE CASCADE
     );
     CREATE TABLE sessions (
         workspace_id TEXT NOT NULL, window_id TEXT NOT NULL, epoch TEXT NOT NULL,
         last_sequence INTEGER NOT NULL DEFAULT -1, active_tab_id TEXT,
         updated_at_ms INTEGER NOT NULL,
         PRIMARY KEY(workspace_id, window_id),
         FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
     );
     CREATE TABLE tabs (
         workspace_id TEXT NOT NULL, window_id TEXT NOT NULL, tab_id TEXT NOT NULL,
         document_id TEXT, kind TEXT NOT NULL, position INTEGER NOT NULL,
         PRIMARY KEY(workspace_id, window_id, tab_id),
         FOREIGN KEY(workspace_id, window_id)
             REFERENCES sessions(workspace_id, window_id) ON DELETE CASCADE,
         FOREIGN KEY(workspace_id, document_id)
             REFERENCES documents(workspace_id, id) ON DELETE CASCADE
     );
     PRAGMA user_version = 1;";

#[test]
fn version_one_allows_new_file_beside_missing_tombstone_after_migration() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch(VERSION_ONE)
        .expect("legacy schema");
    let workspace_id = Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO workspaces(id, path, last_seen_ms) VALUES(?1, 'C:/notes', 1)",
            [&workspace_id],
        )
        .expect("workspace");
    connection
        .execute(
            "INSERT INTO documents VALUES(?1, ?2, 'note.md', 'missing', 2)",
            params![Uuid::new_v4().to_string(), workspace_id],
        )
        .expect("tombstone");
    migrate(&connection).expect("migration");
    connection
        .execute(
            "INSERT INTO documents VALUES(?1, ?2, 'note.md', 'active', NULL)",
            params![Uuid::new_v4().to_string(), workspace_id],
        )
        .expect("replacement");
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .expect("version");
    assert_eq!(version, 9);
}

#[test]
fn newer_schema_is_rejected_without_downgrade() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch("PRAGMA user_version = 99;")
        .expect("version");
    assert!(matches!(
        migrate(&connection),
        Err(UiStateError::UnsupportedSchema { version: 99 })
    ));
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .expect("version");
    assert_eq!(version, 99);
}

#[test]
fn unavailable_database_does_not_prevent_service_creation() {
    let directory = tempfile::tempdir().expect("directory");
    let service = UiStateService::open(directory.path());
    assert!(matches!(
        service.resolve_workspace("C:/notes", 1),
        Err(UiStateError::Unavailable { .. })
    ));
}

#[test]
fn migration_preserves_foreign_keys_and_cascades() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch(&format!("PRAGMA foreign_keys = ON; {VERSION_ONE}"))
        .expect("legacy schema");
    let workspace_id = Uuid::new_v4().to_string();
    let document_id = Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO workspaces(id, path, last_seen_ms) VALUES(?1, 'C:/notes', 1)",
            [&workspace_id],
        )
        .expect("workspace");
    connection
        .execute(
            "INSERT INTO documents VALUES(?1, ?2, 'note.md', 'active', NULL)",
            params![document_id, workspace_id],
        )
        .expect("document");
    connection
        .execute(
            "INSERT INTO view_states VALUES(
                 ?1, 'main', ?2, 'main', X'', X'', 0, 0, X'', 0.0, 'body', 1
             )",
            params![workspace_id, document_id],
        )
        .expect("view");
    migrate(&connection).expect("migration");
    connection
        .execute("DELETE FROM documents WHERE id = ?1", [&document_id])
        .expect("delete");
    let views = connection
        .query_row("SELECT COUNT(*) FROM view_states", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("views");
    assert_eq!(views, 0);
}

#[test]
fn upgrading_from_five_keeps_the_session_and_adds_the_graph_camera() {
    let connection = Connection::open_in_memory().expect("database");
    migrate(&connection).expect("fresh database");
    connection
        .execute_batch(
            "ALTER TABLE sessions DROP COLUMN graph_center_x;
             ALTER TABLE sessions DROP COLUMN graph_center_y;
             ALTER TABLE sessions DROP COLUMN graph_scale;
             ALTER TABLE workspaces DROP COLUMN home_page;
             ALTER TABLE tabs DROP COLUMN relative_path;
             DROP TABLE base_view_states;
             PRAGMA user_version = 5;",
        )
        .expect("roll the schema back to five");
    let workspace_id = Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO workspaces(id, path, last_seen_ms) VALUES(?1, 'C:/notes', 1)",
            [&workspace_id],
        )
        .expect("workspace");
    connection
        .execute(
            "INSERT INTO sessions(workspace_id, window_id, epoch, active_tab_id, updated_at_ms)
             VALUES(?1, 'main', 'epoch', 'tab', 1)",
            [&workspace_id],
        )
        .expect("session");

    migrate(&connection).expect("migration");

    let row = connection
        .query_row(
            "SELECT active_tab_id, graph_center_x, graph_center_y, graph_scale FROM sessions
             WHERE workspace_id = ?1 AND window_id = 'main'",
            [&workspace_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<f64>>(1)?,
                    row.get::<_, Option<f64>>(2)?,
                    row.get::<_, Option<f64>>(3)?,
                ))
            },
        )
        .expect("session row survives");
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .expect("version");

    assert_eq!(row, ("tab".to_owned(), None, None, None));
    assert_eq!(version, 9);
}

#[test]
fn unused_versions_table_is_dropped() {
    let connection = Connection::open_in_memory().expect("database");
    migrate(&connection).expect("migration");
    let tables = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'document_versions'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("tables");
    assert_eq!(tables, 0);
}

fn schema_shape(connection: &Connection) -> Vec<String> {
    let mut statement = connection
        .prepare(
            "SELECT m.type, m.name, p.name, p.type, p.\"notnull\", p.dflt_value, p.pk
             FROM sqlite_master m JOIN pragma_table_info(m.name) p
             WHERE m.type = 'table'
             UNION ALL
             SELECT m.type, m.name, i.name, i.\"unique\", i.partial, NULL, NULL
             FROM sqlite_master m JOIN pragma_index_list(m.name) i
             WHERE m.type = 'table'
             UNION ALL
             SELECT 'fk', m.name, f.\"table\", f.\"from\", f.\"to\", f.on_delete, NULL
             FROM sqlite_master m JOIN pragma_foreign_key_list(m.name) f
             WHERE m.type = 'table'",
        )
        .expect("schema query");
    let mut shape = statement
        .query_map([], |row| {
            Ok((0..7)
                .map(|index| format!("{:?}", row.get::<_, rusqlite::types::Value>(index)))
                .collect::<Vec<_>>()
                .join("|"))
        })
        .expect("schema rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("schema");
    shape.sort();
    shape
}

#[test]
fn a_fresh_database_is_created_at_the_latest_version_without_dead_tables() {
    let connection = Connection::open_in_memory().expect("database");
    migrate(&connection).expect("migration");
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .expect("version");
    assert_eq!(version, 9);
    assert!(
        !schema_shape(&connection)
            .iter()
            .any(|line| line.contains("document_versions")),
        "новая база не создаёт таблицу, которую потом удаляет миграция"
    );
}

#[test]
fn a_migrated_database_ends_with_the_same_schema_as_a_fresh_one() {
    let fresh = Connection::open_in_memory().expect("fresh");
    migrate(&fresh).expect("fresh migration");
    let migrated = Connection::open_in_memory().expect("legacy");
    migrated.execute_batch(VERSION_ONE).expect("legacy schema");
    migrate(&migrated).expect("legacy migration");
    assert_eq!(schema_shape(&migrated), schema_shape(&fresh));
}

#[test]
fn a_database_from_a_newer_version_is_set_aside_by_reset_and_replaced_with_a_fresh_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ui-state.sqlite3");
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("PRAGMA user_version = 99;")
        .unwrap();
    let service = crate::ui_state::UiStateService::open(&path);
    assert!(service.list_workspaces().is_err());

    service.reset(1_234).unwrap();

    assert!(service.list_workspaces().unwrap().is_empty());
    assert!(dir.path().join("ui-state.sqlite3.broken-1234").exists());
}

#[test]
fn reset_sets_aside_a_database_that_was_open_and_in_use() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ui-state.sqlite3");
    let service = crate::ui_state::UiStateService::open(&path);
    service.resolve_workspace("C:/vault", 1).unwrap();
    assert_eq!(service.list_workspaces().unwrap().len(), 1);

    service.reset(42).unwrap();

    assert!(service.list_workspaces().unwrap().is_empty());
    assert!(dir.path().join("ui-state.sqlite3.broken-42").exists());
}

#[test]
fn upgrading_from_eight_preserves_document_and_graph_tabs_and_adds_base_paths() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(VERSION_ONE).unwrap();
    super::migrations::v1_to_v2::migrate_v1_to_v2(&connection).unwrap();
    super::migrations::v2_to_v3::migrate_v2_to_v3(&connection).unwrap();
    super::migrations::v3_to_v4::migrate_v3_to_v4(&connection).unwrap();
    super::migrations::v4_to_v5::migrate_v4_to_v5(&connection).unwrap();
    super::migrations::v5_to_v6::migrate_v5_to_v6(&connection).unwrap();
    super::migrations::v6_to_v7::migrate_v6_to_v7(&connection).unwrap();
    super::migrations::v7_to_v8::migrate_v7_to_v8(&connection).unwrap();
    let workspace = Uuid::new_v4().to_string();
    let document = Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO workspaces(id, path, last_seen_ms) VALUES(?1, 'vault', 1)",
            [&workspace],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO documents(id, workspace_id, relative_path) VALUES(?1, ?2, 'note.md')",
            params![document, workspace],
        )
        .unwrap();
    connection.execute("INSERT INTO sessions(workspace_id, window_id, epoch, updated_at_ms, active_tab_id) VALUES(?1, 'main', 'epoch', 1, 'graph')", [&workspace]).unwrap();
    connection
        .execute(
            "INSERT INTO tabs VALUES(?1, 'main', 'note', ?2, 'document', 0)",
            params![workspace, document],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO tabs VALUES(?1, 'main', 'graph', NULL, 'graph', 1)",
            [&workspace],
        )
        .unwrap();
    migrate(&connection).unwrap();
    let mut query = connection
        .prepare("SELECT kind, document_id, relative_path FROM tabs ORDER BY position")
        .unwrap();
    let tabs = query
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        tabs,
        [
            ("document".into(), Some(document), None),
            ("graph".into(), None, None)
        ]
    );
    connection.execute("INSERT INTO tabs(workspace_id, window_id, tab_id, kind, position, relative_path) VALUES(?1, 'main', 'base', 'base', 2, 'board.base')", [&workspace]).unwrap();
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    let active: String = connection
        .query_row(
            "SELECT active_tab_id FROM sessions WHERE workspace_id = ?1",
            [&workspace],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(active, "graph");
    assert_eq!(version, 9);
}
