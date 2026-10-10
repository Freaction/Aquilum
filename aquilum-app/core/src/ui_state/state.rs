use super::database::UiStateDatabase;
use super::error::UiStateError;
use super::models::{LoadedSession, OpenSessionInput, SaveStateBatchInput};
use rusqlite::{params, OptionalExtension};

impl UiStateDatabase {
    pub fn open_session(
        &mut self,
        input: &OpenSessionInput,
    ) -> Result<LoadedSession, UiStateError> {
        self.connection.execute(
            "INSERT INTO sessions(workspace_id, window_id, epoch, updated_at_ms)
             VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(workspace_id, window_id) DO UPDATE SET
                 epoch = excluded.epoch, last_sequence = -1, updated_at_ms = excluded.updated_at_ms",
            params![
                input.workspace_id.to_string(),
                input.window_id,
                input.epoch.to_string(),
                input.now_ms
            ],
        )?;
        self.load_session(input.workspace_id, &input.window_id)
    }

    pub fn save_batch(&mut self, input: &SaveStateBatchInput) -> Result<bool, UiStateError> {
        let transaction = self.connection.transaction()?;
        let current = transaction
            .query_row(
                "SELECT epoch, last_sequence FROM sessions
                 WHERE workspace_id = ?1 AND window_id = ?2",
                params![input.workspace_id.to_string(), input.window_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        let accepted = current
            .map(|(epoch, sequence)| epoch == input.epoch.to_string() && input.sequence > sequence)
            .unwrap_or(false);
        if !accepted {
            return Ok(false);
        }
        if let Some(session) = &input.session {
            if session.tabs.is_none() {
                if let Some(active_tab_id) = session.active_tab_id {
                    let exists = transaction.query_row(
                        "SELECT EXISTS(SELECT 1 FROM tabs
                         WHERE workspace_id = ?1 AND window_id = ?2 AND tab_id = ?3)",
                        params![
                            input.workspace_id.to_string(),
                            input.window_id,
                            active_tab_id.to_string()
                        ],
                        |row| row.get::<_, bool>(0),
                    )?;
                    if !exists {
                        return Err(UiStateError::InvalidInput {
                            message: "active tab is not present".to_owned(),
                        });
                    }
                }
            }
            if let Some(tabs) = &session.tabs {
                transaction.execute(
                    "DELETE FROM tabs WHERE workspace_id = ?1 AND window_id = ?2",
                    params![input.workspace_id.to_string(), input.window_id],
                )?;
                for tab in tabs {
                    transaction.execute(
                        "INSERT INTO tabs(workspace_id, window_id, tab_id, document_id, kind, position, relative_path)
                         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            input.workspace_id.to_string(),
                            input.window_id,
                            tab.tab_id.to_string(),
                            tab.document_id.map(|id| id.to_string()),
                            tab.kind.as_str(),
                            tab.position,
                            tab.relative_path
                        ],
                    )?;
                }
            }
            transaction.execute(
                "UPDATE sessions SET active_tab_id = ?3
                 WHERE workspace_id = ?1 AND window_id = ?2",
                params![
                    input.workspace_id.to_string(),
                    input.window_id,
                    session.active_tab_id.map(|id| id.to_string())
                ],
            )?;
        }
        for view in &input.views {
            transaction.execute(
                "INSERT INTO view_states(
                     workspace_id, window_id, document_id, pane_id, cursor_anchor, cursor_head,
                     fallback_anchor, fallback_head, scroll_anchor, fallback_scroll_anchor,
                     scroll_offset_px, focused_surface, updated_at_ms
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(workspace_id, window_id, document_id, pane_id) DO UPDATE SET
                     cursor_anchor = excluded.cursor_anchor, cursor_head = excluded.cursor_head,
                     fallback_anchor = excluded.fallback_anchor, fallback_head = excluded.fallback_head,
                     scroll_anchor = excluded.scroll_anchor,
                     fallback_scroll_anchor = excluded.fallback_scroll_anchor,
                     scroll_offset_px = excluded.scroll_offset_px,
                     focused_surface = excluded.focused_surface,
                     updated_at_ms = excluded.updated_at_ms",
                params![
                    input.workspace_id.to_string(),
                    input.window_id,
                    view.document_id.to_string(),
                    view.pane_id,
                    view.cursor_anchor,
                    view.cursor_head,
                    view.fallback_anchor,
                    view.fallback_head,
                    view.scroll_anchor,
                    view.fallback_scroll_anchor,
                    view.scroll_offset_px,
                    view.focused_surface,
                    input.now_ms
                ],
            )?;
        }
        if let Some(camera) = &input.graph_camera {
            transaction.execute(
                "UPDATE sessions SET graph_center_x = ?3, graph_center_y = ?4, graph_scale = ?5
                 WHERE workspace_id = ?1 AND window_id = ?2",
                params![
                    input.workspace_id.to_string(),
                    input.window_id,
                    camera.center_x,
                    camera.center_y,
                    camera.scale
                ],
            )?;
        }
        transaction.execute(
            "UPDATE sessions SET last_sequence = ?3, updated_at_ms = ?4
             WHERE workspace_id = ?1 AND window_id = ?2",
            params![
                input.workspace_id.to_string(),
                input.window_id,
                input.sequence,
                input.now_ms
            ],
        )?;
        transaction.commit()?;
        Ok(true)
    }
}
