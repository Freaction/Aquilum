use super::super::error::SearchError;
use super::target::{select_candidate, target_key};
use crate::search::paths::is_markdown;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub fn resolve(connection: &Connection, root: &Path, source: &Path, target: &str) -> Result<Option<PathBuf>, SearchError> {
    let (key, kind) = target_key(target);
    if key.is_empty() {
        return Ok(is_markdown(source).then(|| source.to_path_buf()));
    }
    let column = if kind == "path" { "relative_key" } else { "title_key" };
    Ok(select_candidate(root, source, &candidates(connection, column, &key)?))
}

pub fn resolve_many(connection: &Connection, root: &Path, source: &Path, targets: &[String]) -> Result<Vec<Option<PathBuf>>, SearchError> {
    targets.iter().map(|target| resolve(connection, root, source, target)).collect()
}

pub fn candidates(connection: &Connection, column: &str, key: &str) -> Result<Vec<String>, SearchError> {
    let mut statement = connection.prepare_cached(&format!("SELECT path FROM documents WHERE {column}=?1"))?;
    let rows = statement.query_map([key], |row| row.get(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
