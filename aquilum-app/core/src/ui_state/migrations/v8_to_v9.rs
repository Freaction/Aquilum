use super::super::error::UiStateError;
use rusqlite::Connection;

pub fn migrate_v8_to_v9(connection: &Connection) -> Result<(), UiStateError> {
    connection.execute_batch(
        "BEGIN IMMEDIATE;
         ALTER TABLE tabs ADD COLUMN relative_path TEXT;
         CREATE TABLE base_view_states (
             workspace_id TEXT NOT NULL,
             relative_path TEXT NOT NULL,
             selected_view INTEGER NOT NULL CHECK(selected_view >= 0),
             PRIMARY KEY(workspace_id, relative_path),
             FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
         );
         PRAGMA user_version = 9;
         COMMIT;",
    )?;
    Ok(())
}
