use super::database::UiStateDatabase;
use super::error::UiStateError;
use super::models::{parse_uuid_column, KnownWorkspace};
use crate::settings::models::SyncSettings;
use crate::search::paths::same_path;
use rusqlite::{params, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

impl UiStateDatabase {
    pub fn list_workspaces(&self) -> Result<Vec<KnownWorkspace>, UiStateError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, path, last_seen_ms, home_page, sync FROM workspaces
                 ORDER BY last_seen_ms DESC",
            )?;
        let rows = statement.query_map([], |row| {
            Ok(KnownWorkspace {
                id: parse_uuid_column(row.get(0)?, 0)?,
                path: row.get(1)?,
                last_seen_ms: row.get(2)?,
                home_page: row.get(3)?,
                sync: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn find_workspace(&self, path: &str) -> Result<Option<Uuid>, UiStateError> {
        Ok(self.list_workspaces()?.into_iter().find(|known| same_path(Path::new(&known.path), Path::new(path))).map(|known| known.id))
    }

    pub fn set_home_page(&mut self, path: &str, home_page: &str) -> Result<(), UiStateError> {
        if let Some(id) = self.find_workspace(path)? {
            self.connection.execute("UPDATE workspaces SET home_page = ?2 WHERE id = ?1", params![id.to_string(), home_page])?;
        }
        Ok(())
    }

    pub fn set_sync(&mut self, path: &str, sync: &SyncSettings) -> Result<(), UiStateError> {
        if let Some(id) = self.find_workspace(path)? {
            let sync = serde_json::to_string(sync).map_err(|error| UiStateError::Database { message: error.to_string() })?;
            self.connection.execute("UPDATE workspaces SET sync = ?2 WHERE id = ?1", params![id.to_string(), sync])?;
        }
        Ok(())
    }

    pub fn forget_workspace(&mut self, workspace_id: Uuid) -> Result<(), UiStateError> {
        self.connection.execute(
            "DELETE FROM workspaces WHERE id = ?1",
            params![workspace_id.to_string()],
        )?;
        Ok(())
    }

    pub fn move_workspace(&mut self, workspace_id: Uuid, path: &str) -> Result<(), UiStateError> {
        if let Some(stale) = self.find_workspace(path)?.filter(|id| *id != workspace_id) {
            self.forget_workspace(stale)?;
        }
        self.connection.execute("UPDATE workspaces SET path = ?2 WHERE id = ?1", params![workspace_id.to_string(), path])?;
        Ok(())
    }

    pub fn resolve_workspace(&mut self, path: &str, now_ms: i64) -> Result<Uuid, UiStateError> {
        let id = self.find_workspace(path)?.unwrap_or_else(Uuid::new_v4);
        self.connection.execute(
            "INSERT INTO workspaces(id, path, last_seen_ms) VALUES(?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET last_seen_ms = excluded.last_seen_ms",
            params![id.to_string(), path, now_ms],
        )?;
        Ok(id)
    }

    pub fn resolve_document(
        &mut self,
        workspace_id: Uuid,
        relative_path: &str,
    ) -> Result<Uuid, UiStateError> {
        let existing = self
            .connection
            .query_row(
                "SELECT id FROM documents
                 WHERE workspace_id = ?1 AND relative_path = ?2 AND status = 'active'",
                params![workspace_id.to_string(), relative_path],
                |row| parse_uuid_column(row.get(0)?, 0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        let id = Uuid::new_v4();
        self.connection.execute(
            "INSERT INTO documents(id, workspace_id, relative_path) VALUES(?1, ?2, ?3)",
            params![id.to_string(), workspace_id.to_string(), relative_path],
        )?;
        Ok(id)
    }

    pub fn rename_document(
        &mut self,
        document_id: Uuid,
        relative_path: &str,
    ) -> Result<(), UiStateError> {
        self.connection.execute(
            "UPDATE documents SET relative_path = ?2, status = 'active', missing_since_ms = NULL
             WHERE id = ?1",
            params![document_id.to_string(), relative_path],
        )?;
        Ok(())
    }
}
