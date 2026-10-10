use super::error::UiStateError;
use super::models::{OpenSessionInput, SaveStateBatchInput, TabKind};
use super::paths::normalize_base_relative;
use std::collections::HashSet;

fn invalid(message: impl Into<String>) -> UiStateError {
    UiStateError::InvalidInput {
        message: message.into(),
    }
}

fn validate_label(value: &str, name: &str) -> Result<(), UiStateError> {
    if value.is_empty() || value.len() > 128 {
        return Err(invalid(format!("invalid {name}")));
    }
    Ok(())
}

pub fn validate_open_session(input: &OpenSessionInput) -> Result<(), UiStateError> {
    validate_label(&input.window_id, "window id")
}

pub fn validate_batch(input: &SaveStateBatchInput) -> Result<(), UiStateError> {
    validate_label(&input.window_id, "window id")?;
    if input.sequence < 0 {
        return Err(invalid("sequence must not be negative"));
    }
    if let Some(session) = &input.session {
        let tabs = session.tabs.as_deref().unwrap_or(&[]);
        let mut tab_ids = HashSet::with_capacity(tabs.len());
        for tab in tabs {
            if !tab_ids.insert(tab.tab_id) {
                return Err(invalid("duplicate tab id"));
            }
            match (&tab.kind, tab.document_id) {
                (TabKind::Document, None)
                | (TabKind::Empty, Some(_))
                | (TabKind::Graph, Some(_))
                | (TabKind::Base, Some(_)) => {
                    return Err(invalid("tab kind does not match document id"));
                }
                _ => {}
            }
            if tab.kind == TabKind::Base {
                normalize_base_relative(
                    tab.relative_path
                        .as_deref()
                        .ok_or_else(|| invalid("base tab path is missing"))?,
                )?;
            } else if tab.relative_path.is_some() {
                return Err(invalid("only base tabs have a persisted path"));
            }
        }
        if session.tabs.is_some()
            && session
                .active_tab_id
                .is_some_and(|active| !tab_ids.contains(&active))
        {
            return Err(invalid("active tab is not present"));
        }
    }
    if let Some(camera) = &input.graph_camera {
        if !camera.center_x.is_finite() || !camera.center_y.is_finite() {
            return Err(invalid("graph camera centre must be finite"));
        }
        if !camera.scale.is_finite() || camera.scale <= 0.0 {
            return Err(invalid("graph camera scale must be positive"));
        }
    }
    for view in &input.views {
        validate_label(&view.pane_id, "pane id")?;
        validate_label(&view.focused_surface, "focused surface")?;
        if view.fallback_anchor < 0 || view.fallback_head < 0 || view.fallback_scroll_anchor < 0 {
            return Err(invalid("fallback position must not be negative"));
        }
        if !view.scroll_offset_px.is_finite() {
            return Err(invalid("scroll offset must be finite"));
        }
        if view.cursor_anchor.len() > 4_096
            || view.cursor_head.len() > 4_096
            || view.scroll_anchor.len() > 4_096
        {
            return Err(invalid("relative position is too large"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_batch;
    use crate::ui_state::models::SaveStateBatchInput;
    use uuid::Uuid;

    #[test]
    fn stale_or_malformed_batch_is_rejected_before_storage() {
        let input = SaveStateBatchInput {
            workspace_id: Uuid::new_v4(),
            window_id: "main".to_owned(),
            epoch: Uuid::new_v4(),
            sequence: -1,
            now_ms: 1,
            session: None,
            views: Vec::new(),
            graph_camera: None,
        };
        assert!(validate_batch(&input).is_err());
    }
}
