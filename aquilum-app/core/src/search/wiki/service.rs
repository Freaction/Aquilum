use super::repository::candidate_sources;
use super::target::title_key;
use super::{
    backlinks, outgoing_links, plan_for_rename, resolve_many, LinkRewrite,
};
use crate::search::error::SearchError;
use crate::search::paths::markdown_files;
use crate::search::models::{Backlink, OutgoingLink, WikiLinkResolution};
use crate::search::paths::{canonical_path, inside};
use crate::search::service::SearchService;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

impl SearchService {
    pub fn backlinks(&self, workspace: &str, document: &str) -> Result<Vec<Backlink>, SearchError> {
        let (root, metadata_path) = self.index_paths(workspace)?;
        let document = inside(&root, document)?;
        backlinks(&Connection::open(metadata_path)?, &root, &document)
    }

    pub fn backlink_counts(
        &self,
        workspace: &str,
        documents: &[String],
    ) -> Result<Vec<usize>, SearchError> {
        let (root, metadata_path) = self.index_paths(workspace)?;
        let connection = Connection::open(metadata_path)?;
        documents
            .iter()
            .map(|document| {
                let document = canonical_path(Path::new(document));
                let own = document.to_string_lossy();
                let links = backlinks(&connection, &root, &document)?;
                Ok(links
                    .iter()
                    .map(|link| link.path.as_str())
                    .filter(|path| *path != own)
                    .collect::<HashSet<_>>()
                    .len())
            })
            .collect()
    }

    pub fn outgoing_links(
        &self,
        workspace: &str,
        document: &str,
    ) -> Result<Vec<OutgoingLink>, SearchError> {
        let (root, metadata_path) = self.index_paths(workspace)?;
        let document = inside(&root, document)?;
        outgoing_links(&Connection::open(metadata_path)?, &root, &document)
    }

    pub fn resolve_wiki_links(
        &self,
        workspace: &str,
        source: &str,
        targets: &[String],
    ) -> Result<WikiLinkResolution, SearchError> {
        let (root, metadata_path, complete) = self.with_index(workspace, |open| {
            Ok((
                open.root.clone(),
                open.metadata_path.clone(),
                !open.progress.snapshot().updating,
            ))
        })?;
        let source = inside(&root, source)?;
        Ok(WikiLinkResolution {
            paths: resolve_many(&Connection::open(metadata_path)?, &root, &source, targets)?
                .into_iter()
                .map(|path| path.map(|value| value.to_string_lossy().into_owned()))
                .collect(),
            complete,
        })
    }

    pub fn plan_links_for_rename(
        &self,
        old_path: &Path,
        new_path: &Path,
    ) -> Result<Vec<LinkRewrite>, SearchError> {
        let old_path = canonical_path(old_path);
        let new_path = canonical_path(new_path);
        let found = self.with_index_containing(&old_path, |open| {
            Ok((
                open.root.clone(),
                open.metadata_path.clone(),
                open.progress.snapshot().updating,
            ))
        })?;
        let Some((root, metadata_path, updating)) = found else {
            if self.active_root().is_some() {
                return Err(SearchError::invalid_workspace("Rename is outside workspace"));
            }
            return Ok(Vec::new());
        };
        if !new_path.starts_with(&root) {
            return Err(SearchError::invalid_workspace("Rename is outside workspace"));
        }
        let connection = Connection::open(metadata_path)?;
        let sources = if updating {
            markdown_files(&root).collect::<Vec<_>>()
        } else {
            candidate_sources(&connection, &root, &old_path)?
                .into_iter()
                .map(PathBuf::from)
                .collect()
        };
        let fallback_candidates = updating.then(|| {
            let old_title = title_key(&old_path);
            sources
                .iter()
                .filter(|path| title_key(path) == old_title)
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        });
        plan_for_rename(
            &connection,
            &root,
            &old_path,
            &new_path,
            sources,
            fallback_candidates.as_deref(),
        )
    }

}
