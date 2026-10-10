use super::card_membership::{scalar, unassigned, Membership};
use super::property::values_equal;
use super::{BaseDefinition, BaseError};
use crate::app_core::Core;
use crate::files::document::read_raw_file_snapshot_impl;
use crate::files::error::FileCommandError;
use crate::files::gate;
use crate::files::models::FileWriteResult;
use crate::history::Source;
use crate::search::paths::{is_markdown, relative_slash_path};
use serde_json::Value;
use std::path::Path;

#[derive(Debug)]
pub enum BaseCardError {
    Base(BaseError),
    File(FileCommandError),
    Invalid(String),
    GroupConflict { expected: Value, actual: Value },
}

impl std::fmt::Display for BaseCardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base(e) => e.fmt(f),
            Self::File(e) => e.fmt(f),
            Self::Invalid(e) => f.write_str(e),
            Self::GroupConflict { .. } => f.write_str("The card group changed on disk"),
        }
    }
}
impl std::error::Error for BaseCardError {}
impl From<BaseError> for BaseCardError {
    fn from(e: BaseError) -> Self {
        Self::Base(e)
    }
}
impl From<FileCommandError> for BaseCardError {
    fn from(e: FileCommandError) -> Self {
        Self::File(e)
    }
}

impl Core {
    pub fn create_base_card(
        &self,
        workspace: &Path,
        definition: &BaseDefinition,
        view_index: usize,
        path: &Path,
        text: &str,
        group: &Value,
    ) -> Result<FileWriteResult, BaseCardError> {
        let relative = card_path(workspace, path)?;
        let membership = Membership::new(definition, view_index)?;
        membership.row(relative.clone(), text)?;
        let text = membership.update(text, group, true)?;
        if !membership.matches(&membership.row(relative, &text)?) {
            return Err(BaseCardError::Invalid(
                "The card does not belong to this board".into(),
            ));
        }
        Ok(gate::create(self, path, &text, Source::Me, None)?)
    }

    pub fn move_base_card(
        &self,
        workspace: &Path,
        definition: &BaseDefinition,
        view_index: usize,
        path: &Path,
        expected_group: &Value,
        group: &Value,
    ) -> Result<FileWriteResult, BaseCardError> {
        self.move_base_card_with(
            workspace,
            definition,
            view_index,
            path,
            expected_group,
            group,
            |_, _| {},
        )
    }

    pub(super) fn move_base_card_with(
        &self,
        workspace: &Path,
        definition: &BaseDefinition,
        view_index: usize,
        path: &Path,
        expected_group: &Value,
        group: &Value,
        mut before_write: impl FnMut(usize, &Path),
    ) -> Result<FileWriteResult, BaseCardError> {
        let relative = card_path(workspace, path)?;
        let membership = Membership::new(definition, view_index)?;
        scalar(expected_group, &membership.group)?;
        for attempt in 0..2 {
            let snapshot = read_raw_file_snapshot_impl(path)?;
            let row = membership.row(relative.clone(), &snapshot.content)?;
            let actual = row.fields.get(&membership.group).unwrap_or(&Value::Null);
            if !values_equal(&unassigned(actual), &unassigned(expected_group)) {
                return Err(BaseCardError::GroupConflict {
                    expected: expected_group.clone(),
                    actual: actual.clone(),
                });
            }
            if !membership.matches(&row) {
                return Err(BaseCardError::Invalid(
                    "The card does not belong to this board".into(),
                ));
            }
            let text = membership.update(&snapshot.content, group, false)?;
            before_write(attempt, path);
            match gate::write(self, path, &text, Some(&snapshot.hash), Source::Me, None) {
                Err(FileCommandError::Conflict { .. }) if attempt == 0 => continue,
                Ok(result) => {
                    self.documents.reconcile_paths(self, &[path.to_path_buf()]);
                    return Ok(result);
                }
                Err(error) => return Err(error.into()),
            }
        }
        unreachable!()
    }
}

fn card_path(workspace: &Path, path: &Path) -> Result<String, BaseCardError> {
    let root = std::fs::canonicalize(workspace).map_err(FileCommandError::from)?;
    let parent = path
        .parent()
        .ok_or_else(|| BaseCardError::Invalid("Card path has no parent".into()))?;
    let parent = std::fs::canonicalize(parent).map_err(FileCommandError::from)?;
    let canonical = if path.exists() {
        std::fs::canonicalize(path).map_err(FileCommandError::from)?
    } else {
        parent.join(
            path.file_name()
                .ok_or_else(|| BaseCardError::Invalid("Invalid card path".into()))?,
        )
    };
    if !canonical.starts_with(&root) || !is_markdown(&canonical) {
        return Err(BaseCardError::Invalid(
            "Card must be a Markdown file inside the workspace".into(),
        ));
    }
    Ok(relative_slash_path(&root, &canonical))
}
