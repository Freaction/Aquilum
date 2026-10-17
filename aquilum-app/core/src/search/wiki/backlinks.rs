use super::super::error::SearchError;
use super::super::models::Backlink;
use super::resolver::select_candidates;
use super::target::{relative_key, select_candidate, title_key};
use super::LINK_LIST_LIMIT;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

const QUERY_BATCH_SIZE: usize = 256;

type LinkRow = (String, String, i64, i64);

pub fn backlinks(
    connection: &Connection,
    root: &Path,
    document: &Path,
) -> Result<Vec<Backlink>, SearchError> {
    let title = title_key(document);
    let relative = relative_key(root, document);
    let candidates = select_candidates(connection, &title)?;
    let mut statement = connection.prepare(
        "SELECT * FROM (
           SELECT link.source_path, link.target_kind, page.start + link.byte_offset AS at, page.start_utf16 + link.offset_utf16
           FROM wiki_links link JOIN note_pages page ON page.id = link.page
           WHERE (link.target_kind='name' AND link.target_key=?1) OR (link.target_kind='path' AND link.target_key=?2))
         WHERE source_path > ?3 OR (source_path = ?3 AND at > ?4)
         ORDER BY source_path, at LIMIT ?5",
    )?;
    let mut cursor_path = String::new();
    let mut cursor_offset = -1;
    let mut items = Vec::new();
    while items.len() < LINK_LIST_LIMIT {
        let rows = query_batch(
            &mut statement,
            &title,
            &relative,
            &cursor_path,
            cursor_offset,
        )?;
        let Some(last) = rows.last() else { break };
        cursor_path = last.0.clone();
        cursor_offset = last.2;
        let exhausted = rows.len() < QUERY_BATCH_SIZE;
        for (source, target_kind, _byte_offset, offset_utf16) in rows {
            let source_path = PathBuf::from(&source);
            if target_kind == "name"
                && select_candidate(root, &source_path, &candidates).as_deref() != Some(document)
            {
                continue;
            }
            items.push(Backlink {
                path: source,
                title: crate::search::paths::title(&source_path),
                offset: offset_utf16.max(0) as usize,
            });
            if items.len() == LINK_LIST_LIMIT {
                break;
            }
        }
        if exhausted {
            break;
        }
    }
    Ok(items)
}

fn query_batch(
    statement: &mut rusqlite::Statement<'_>,
    title: &str,
    relative: &str,
    cursor_path: &str,
    cursor_offset: i64,
) -> Result<Vec<LinkRow>, SearchError> {
    Ok(statement
        .query_map(
            params![
                title,
                relative,
                cursor_path,
                cursor_offset,
                QUERY_BATCH_SIZE as i64
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?
        .collect::<Result<Vec<_>, _>>()?)
}
