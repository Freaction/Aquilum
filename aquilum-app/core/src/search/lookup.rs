use super::error::SearchError;
use super::models::{SearchIndexStatus, SearchResponse};
use super::paths::{canonical_workspace, same_path};
use super::query::Request;
use super::service::{OpenIndex, SearchService};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

impl SearchService {
    pub fn search(&self, workspace: &str, request: &Request) -> Result<SearchResponse, SearchError> {
        let found = self.with_index(workspace, |open| {
            let found = open.index.search(&Request { metadata: Some(&open.metadata_path), cancel: request.cancel.clone(), ..*request })?;
            Ok(SearchResponse { query_terms: found.terms, results: found.results, has_more: found.has_more, status: open.progress.snapshot() })
        });
        match found {
            Err(SearchError::Unavailable { .. }) => Ok(SearchResponse::idle()),
            found => found,
        }
    }

    pub fn with_index<T>(
        &self,
        workspace: &str,
        read: impl FnOnce(&OpenIndex) -> Result<T, SearchError>,
    ) -> Result<T, SearchError> {
        let root = canonical_workspace(workspace).map_err(|_| self.missing_index())?;
        self.locate(|open| same_path(&open.root, &root), read)?
            .ok_or_else(|| self.missing_index())
    }

    pub fn with_index_containing<T>(
        &self,
        path: &Path,
        read: impl FnOnce(&OpenIndex) -> Result<T, SearchError>,
    ) -> Result<Option<T>, SearchError> {
        self.locate(|open| path.starts_with(&open.root), read)
    }

    fn locate<T>(
        &self,
        fits: impl Fn(&OpenIndex) -> bool,
        read: impl FnOnce(&OpenIndex) -> Result<T, SearchError>,
    ) -> Result<Option<T>, SearchError> {
        {
            let active = self.active.read().map_err(SearchError::task)?;
            if let Some(open) = active.as_ref().filter(|open| fits(open)) {
                return read(open).map(Some);
            }
        }
        let guest = self.guest.read().map_err(SearchError::task)?;
        match guest.as_ref().filter(|guest| fits(&guest.index)) {
            Some(guest) => {
                guest.touch();
                read(&guest.index).map(Some)
            }
            None => Ok(None),
        }
    }

    fn missing_index(&self) -> SearchError {
        if self.active_root().is_some() {
            SearchError::invalid_workspace("Index belongs to a different workspace")
        } else {
            SearchError::Unavailable {
                message: "Index is not ready".to_owned(),
            }
        }
    }

    pub fn with_metadata<T>(&self, workspace: &str, work: impl FnOnce(&Path, &Connection) -> Result<T, SearchError>) -> Result<T, SearchError> {
        if self.with_index(workspace, |_| Ok(())).is_err() {
            self.prepare(workspace)?;
        }
        self.with_index(workspace, |open| work(&open.root, &Connection::open(&open.metadata_path)?))
    }

    pub fn active_root(&self) -> Option<PathBuf> {
        self.active
            .read()
            .ok()
            .and_then(|active| active.as_ref().map(|value| value.root.clone()))
    }

    pub fn status(&self, workspace: Option<&str>) -> SearchIndexStatus {
        match workspace {
            Some(workspace) => self
                .with_index(workspace, |open| Ok(open.progress.snapshot()))
                .ok(),
            None => self
                .active
                .read()
                .ok()
                .and_then(|active| active.as_ref().map(|open| open.progress.snapshot())),
        }
        .unwrap_or_else(SearchIndexStatus::idle)
    }
}
