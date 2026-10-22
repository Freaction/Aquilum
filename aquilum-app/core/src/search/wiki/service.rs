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
        self.with_metadata(workspace, |root, connection| backlinks(connection, root, &inside(root, document)?))
    }

    pub fn backlink_counts(
        &self,
        workspace: &str,
        documents: &[String],
    ) -> Result<Vec<usize>, SearchError> {
        self.with_metadata(workspace, |root, connection| {
            documents
                .iter()
                .map(|document| {
                    let document = canonical_path(Path::new(document));
                    let own = document.to_string_lossy();
                    let links = backlinks(connection, root, &document)?;
                    Ok(links
                        .iter()
                        .map(|link| link.path.as_str())
                        .filter(|path| *path != own)
                        .collect::<HashSet<_>>()
                        .len())
                })
                .collect()
        })
    }

    pub fn outgoing_links(
        &self,
        workspace: &str,
        document: &str,
    ) -> Result<Vec<OutgoingLink>, SearchError> {
        self.with_metadata(workspace, |root, connection| outgoing_links(connection, root, &inside(root, document)?))
    }

    pub fn resolve_wiki_links(
        &self,
        workspace: &str,
        source: &str,
        targets: &[String],
    ) -> Result<WikiLinkResolution, SearchError> {
        self.with_index(workspace, |open| {
            let source = inside(&open.root, source)?;
            Ok(WikiLinkResolution {
                paths: resolve_many(&Connection::open(&open.metadata_path)?, &open.root, &source, targets)?
                    .into_iter()
                    .map(|path| path.map(|value| value.to_string_lossy().into_owned()))
                    .collect(),
                complete: !open.progress.snapshot().updating,
            })
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
            let root = &open.root;
            if !new_path.starts_with(root) {
                return Err(SearchError::invalid_workspace("Rename is outside workspace"));
            }
            let updating = open.progress.snapshot().updating;
            let connection = Connection::open(&open.metadata_path)?;
            let sources = if updating {
                markdown_files(root).collect::<Vec<_>>()
            } else {
                candidate_sources(&connection, root, &old_path)?
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
            plan_for_rename(&connection, root, &old_path, &new_path, sources, fallback_candidates.as_deref())
        })?;
        match found {
            Some(plan) => Ok(plan),
            None if self.active_root().is_some() => Err(SearchError::invalid_workspace("Rename is outside workspace")),
            None => Ok(Vec::new()),
        }
    }
}
