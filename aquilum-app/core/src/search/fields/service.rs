use super::query;
use super::{as_object, head_fields, NoteFieldRows, NoteFields};
use crate::search::error::SearchError;
use crate::search::models::SearchIndexState;
use crate::search::paths::canonical_path;
use crate::search::service::SearchService;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

impl SearchService {
    pub fn base_rows(
        &self,
        workspace: &str,
    ) -> Result<Option<Vec<crate::bases::BaseRow>>, SearchError> {
        let found = self.with_index(workspace, |open| {
            let status = open.progress.snapshot();
            if status.state != SearchIndexState::Ready || status.updating {
                return Ok(None);
            }
            super::bases::read(&Connection::open(&open.metadata_path)?, &open.root).map(Some)
        });
        match found {
            Err(SearchError::Unavailable { .. }) => Ok(None),
            found => found,
        }
    }

    pub fn note_fields(
        &self,
        workspace: &str,
        paths: &[String],
    ) -> Result<Vec<NoteFields>, SearchError> {
        let (root, metadata_path) = self.index_paths(workspace)?;
        let requested = paths
            .iter()
            .map(|path| {
                let canonical = canonical_path(Path::new(path));
                let inside = canonical.starts_with(&root);
                (
                    path.clone(),
                    inside.then(|| canonical.to_string_lossy().into_owned()),
                )
            })
            .collect::<Vec<_>>();
        let inside = requested
            .iter()
            .filter_map(|(_, canonical)| canonical.clone())
            .collect::<Vec<_>>();
        let mut indexed = query::read(&Connection::open(metadata_path)?, &inside)?;
        Ok(requested
            .into_iter()
            .map(|(path, canonical)| {
                let found = canonical
                    .map(|canonical| {
                        indexed
                            .remove(&canonical)
                            .unwrap_or_else(|| head_fields(Path::new(&canonical)))
                    })
                    .unwrap_or_default();
                NoteFields {
                    path,
                    fields: as_object(&found),
                }
            })
            .collect())
    }

    pub fn note_fields_for_filter(
        &self,
        workspace: &str,
        equals: &[(String, String)],
        has: &[String],
    ) -> Result<Option<NoteFieldRows>, SearchError> {
        let Some(metadata_path) = self.ready_metadata_path(workspace) else {
            return Ok(None);
        };
        let connection = Connection::open(metadata_path)?;
        let paths = query::candidates(&connection, equals, has)?;
        let mut fields = query::read(&connection, &paths)?;
        Ok(Some(
            paths
                .into_iter()
                .map(|path| {
                    let found = fields.remove(&path).unwrap_or_default();
                    (path, found)
                })
                .collect(),
        ))
    }

    fn ready_metadata_path(&self, workspace: &str) -> Option<PathBuf> {
        self.with_index(workspace, |open| {
            let status = open.progress.snapshot();
            Ok(
                (status.state == SearchIndexState::Ready && !status.updating)
                    .then(|| open.metadata_path.clone()),
            )
        })
        .ok()
        .flatten()
    }
}
