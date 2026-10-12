use super::super::error::SearchError;
use super::target::{select_candidate, target_key};
use crate::search::paths::is_markdown;
use rusqlite::{Connection, Statement};
use std::path::{Path, PathBuf};

pub struct LinkResolver<'connection> {
    by_title: Statement<'connection>,
    by_relative_path: Statement<'connection>,
}

impl<'connection> LinkResolver<'connection> {
    pub fn new(connection: &'connection Connection) -> Result<Self, SearchError> {
        Ok(Self {
            by_title: connection.prepare("SELECT path FROM wiki_documents WHERE title_key=?1")?,
            by_relative_path: connection
                .prepare("SELECT path FROM wiki_documents WHERE relative_key=?1")?,
        })
    }

    pub fn resolve(
        &mut self,
        root: &Path,
        source: &Path,
        target: &str,
    ) -> Result<Option<PathBuf>, SearchError> {
        let (key, kind) = target_key(target);
        if key.is_empty() {
            return Ok(is_markdown(source).then(|| source.to_path_buf()));
        }
        let candidates = if kind == "path" {
            query(&mut self.by_relative_path, &key)?
        } else {
            query(&mut self.by_title, &key)?
        };
        Ok(select_candidate(root, source, &candidates))
    }
}

pub fn resolve_many(
    connection: &Connection,
    root: &Path,
    source: &Path,
    targets: &[String],
) -> Result<Vec<Option<PathBuf>>, SearchError> {
    let mut resolver = LinkResolver::new(connection)?;
    targets
        .iter()
        .map(|target| resolver.resolve(root, source, target))
        .collect()
}

pub fn select_candidates(connection: &Connection, key: &str) -> Result<Vec<String>, SearchError> {
    query(
        &mut connection.prepare("SELECT path FROM wiki_documents WHERE title_key=?1")?,
        key,
    )
}

fn query(statement: &mut Statement<'_>, key: &str) -> Result<Vec<String>, SearchError> {
    Ok(statement
        .query_map([key], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?)
}
