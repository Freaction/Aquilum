use super::database::UiStateDatabase;
use super::error::UiStateError;
use super::models::KnownWorkspace;
use super::paths::display_workspace;
use crate::settings::models::SyncSettings;
use crate::search::paths::{identity, same_path};
use rusqlite::{params, OptionalExtension};
use std::collections::HashSet;
use std::path::Path;
use uuid::Uuid;

fn parse_uuid(value: String) -> Result<Uuid, UiStateError> {
    Uuid::parse_str(&value).map_err(|error| UiStateError::Database {
        message: error.to_string(),
    })
}

impl UiStateDatabase {
    pub fn list_workspaces(&self) -> Result<Vec<KnownWorkspace>, UiStateError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, path, last_seen_ms, home_page, sync FROM workspaces
                 ORDER BY last_seen_ms DESC",
            )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let (mut workspaces, mut seen) = (Vec::new(), HashSet::new());
        for row in rows {
            let (id, path, last_seen_ms, home_page, sync) = row?;
            let path = display_workspace(&path);
            if seen.insert(identity(Path::new(&path))) {
                let sync = serde_json::from_str(&sync).unwrap_or_default();
                workspaces.push(KnownWorkspace { id: parse_uuid(id)?, path, last_seen_ms, home_page, sync });
            }
        }
        Ok(workspaces)
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
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let is_new = existing.is_none();
        let id = match existing {
            Some(value) => parse_uuid(value)?,
            None => Uuid::new_v4(),
        };
        if is_new {
            self.connection.execute(
                "INSERT INTO documents(id, workspace_id, relative_path) VALUES(?1, ?2, ?3)",
                params![id.to_string(), workspace_id.to_string(), relative_path],
            )?;
        }
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
