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
    let connection = crate::sqlite::memory();
    connection
        .execute_batch(
            VERSION_ONE,
        )
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
    assert_eq!(version, 10);
}

#[test]
fn newer_schema_is_rejected_without_downgrade() {
    let connection = crate::sqlite::memory();
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
    let connection = crate::sqlite::memory();
    connection
        .execute_batch(
            &format!("PRAGMA foreign_keys = ON; {VERSION_ONE}"),
        )
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
    let connection = crate::sqlite::memory();
    migrate(&connection).expect("fresh database");
    connection
        .execute_batch(
            "ALTER TABLE sessions DROP COLUMN graph_center_x;
             ALTER TABLE sessions DROP COLUMN graph_center_y;
             ALTER TABLE sessions DROP COLUMN graph_scale;
             ALTER TABLE workspaces DROP COLUMN home_page;
             ALTER TABLE workspaces DROP COLUMN sync;
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
    assert_eq!(version, 10);
}

#[test]
fn unused_versions_table_is_dropped() {
    let connection = crate::sqlite::memory();
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
    let connection = crate::sqlite::memory();
    migrate(&connection).expect("migration");
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .expect("version");
    assert_eq!(version, 10);
    assert!(
        !schema_shape(&connection).iter().any(|line| line.contains("document_versions")),
        "новая база не создаёт таблицу, которую потом удаляет миграция"
    );
}

#[test]
fn a_migrated_database_ends_with_the_same_schema_as_a_fresh_one() {
    let fresh = crate::sqlite::memory();
    migrate(&fresh).expect("fresh migration");
    let migrated = crate::sqlite::memory();
    migrated.execute_batch(VERSION_ONE).expect("legacy schema");
    migrate(&migrated).expect("legacy migration");
    assert_eq!(schema_shape(&migrated), schema_shape(&fresh));
}
