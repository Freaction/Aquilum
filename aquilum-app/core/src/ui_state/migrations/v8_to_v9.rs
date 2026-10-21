use super::super::error::UiStateError;
use rusqlite::Connection;

pub fn migrate_v8_to_v9(connection: &Connection) -> Result<(), UiStateError> {
    connection.execute_batch(
        "BEGIN IMMEDIATE;
         ALTER TABLE workspaces ADD COLUMN sync TEXT NOT NULL DEFAULT '';
         PRAGMA user_version = 9;
         COMMIT;",
    )?;
    Ok(())
}
