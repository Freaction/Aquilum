use super::database::UiStateDatabase;
use super::error::UiStateError;
use super::paths::normalize_base_relative;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

impl UiStateDatabase {
    pub fn load_base_view(
        &self,
        workspace_id: Uuid,
        relative_path: &str,
        view_count: usize,
    ) -> Result<usize, UiStateError> {
        let relative_path = normalize_base_relative(relative_path)?;
        let selected = self.connection.query_row(
            "SELECT selected_view FROM base_view_states WHERE workspace_id = ?1 AND relative_path = ?2",
            params![workspace_id.to_string(), relative_path],
            |row| row.get::<_, i64>(0),
        ).optional()?.unwrap_or_default();
        let selected = usize::try_from(selected).map_err(|_| UiStateError::InvalidInput {
            message: "base view index is invalid".into(),
        })?;
        Ok(selected.min(view_count.saturating_sub(1)))
    }

    pub fn save_base_view(
        &self,
        workspace_id: Uuid,
        relative_path: &str,
        selected_view: usize,
    ) -> Result<(), UiStateError> {
        let relative_path = normalize_base_relative(relative_path)?;
        let selected_view =
            i64::try_from(selected_view).map_err(|_| UiStateError::InvalidInput {
                message: "base view index is too large".into(),
            })?;
        self.connection.execute(
            "INSERT INTO base_view_states(workspace_id, relative_path, selected_view)
             VALUES(?1, ?2, ?3)
             ON CONFLICT(workspace_id, relative_path) DO UPDATE SET selected_view = excluded.selected_view",
            params![workspace_id.to_string(), relative_path, selected_view],
        )?;
        Ok(())
    }
}
