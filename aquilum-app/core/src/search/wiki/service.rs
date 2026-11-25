use super::repository::candidate_sources;
use super::target::{notes_titled, relative_key, split, target_key, title_key};
use super::{
    backlinks, outgoing_links, plan_for_rename, resolve_many, LinkRewrite,
};
use crate::search::error::SearchError;
use crate::search::paths::markdown_files;
use crate::search::models::{Backlink, OutgoingLink, WikiLinkResolution};
use crate::search::paths::{canonical_path, inside};
use crate::search::service::SearchService;
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
                paths: resolve_many(&crate::sqlite::read(&open.metadata_path)?, &open.root, &source, targets)?
                    .into_iter()
                    .map(|path| path.map(|value| value.to_string_lossy().into_owned()))
                    .collect(),
                complete: !open.progress.snapshot().updating,
            })
        })
    }

    pub fn resolve_wiki_link(&self, workspace: &Path, source: &Path, target: &str) -> Option<PathBuf> {
        let found = self.resolve_wiki_links(&workspace.to_string_lossy(), &source.to_string_lossy(), &[target.to_owned()]).ok();
        if let Some(path) = found.and_then(|found| found.paths.into_iter().next().flatten()) {
            return Some(PathBuf::from(path));
        }
        let (key, kind) = target_key(target);
        let title = split(target).note.rsplit(['/', '\\']).next()?;
        notes_titled(workspace, title).into_iter().find(|path| kind == "name" || relative_key(workspace, path) == key)
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
            let connection = crate::sqlite::read(&open.metadata_path)?;
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

#[cfg(test)]
mod tests {
    use crate::search::service::SearchService;
    use std::sync::Arc;

    #[test]
    fn without_an_index_a_link_is_found_on_disk_by_its_name() {
        let vault = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(vault.path().join("Папка")).unwrap();
        let note = vault.path().join("Папка").join("Книга.MD");
        std::fs::write(&note, "").unwrap();
        let service = SearchService::new(vault.path().join("indexes"), Arc::new(|_: &std::path::Path| {}));
        let resolve = |target: &str| service.resolve_wiki_link(vault.path(), vault.path(), target);
        assert_eq!(resolve("книга#Раздел"), Some(note.clone()));
        assert_eq!(resolve("Папка/Книга.md|подпись"), Some(note));
        assert_eq!(resolve("Другая/Книга"), None);
        assert_eq!(resolve("#Раздел"), None);
        assert_eq!(resolve("Нет такой"), None);
    }
}
