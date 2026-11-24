use super::target::pick_candidate;
use crate::search::error::SearchError;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;

pub struct WikiDocument {
    pub path: String,
    pub relative_key: String,
    pub title_key: String,
    pub created_ns: i64,
    pub modified_ns: i64,
}

pub fn read_documents(connection: &Connection) -> Result<Vec<WikiDocument>, SearchError> {
    let mut statement =
        connection.prepare("SELECT path, relative_key, title_key, created_ns, modified_ns FROM documents ORDER BY path")?;
    let rows = statement.query_map([], |row| {
        Ok(WikiDocument {
            path: row.get(0)?,
            relative_key: row.get(1)?,
            title_key: row.get(2)?,
            created_ns: row.get(3)?,
            modified_ns: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn for_each_link(
    connection: &Connection,
    root: &Path,
    documents: &[WikiDocument],
    mut visit: impl FnMut(&str, &str),
) -> Result<(), SearchError> {
    let mut by_title = HashMap::<&str, Vec<&str>>::new();
    let mut by_relative = HashMap::<&str, Vec<&str>>::new();
    for document in documents {
        by_relative.entry(&document.relative_key).or_default().push(&document.path);
        by_title.entry(&document.title_key).or_default().push(&document.path);
    }

    let mut statement =
        connection.prepare("SELECT page.path, link.target_key, link.target_kind FROM wiki_links link JOIN note_pages page ON page.id = link.page")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (source, key, kind) = row?;
        let candidates = if kind == "path" {
            by_relative.get(key.as_str())
        } else {
            by_title.get(key.as_str())
        };
        let Some(target) =
            candidates.and_then(|paths| pick_candidate(root, Path::new(&source), paths))
        else {
            continue;
        };
        visit(&source, target);
    }
    Ok(())
}
