use super::super::error::SearchError;
use rusqlite::{params, Connection};

pub struct Candidate {
    pub path: String,
    pub title_key: String,
}

pub fn starting_with(
    connection: &Connection,
    key: &str,
    limit: usize,
) -> Result<Vec<Candidate>, SearchError> {
    read(
        connection,
        "SELECT path, title_key FROM documents
         WHERE title_key >= ?1 AND title_key < ?2
         ORDER BY length(title_key), title_key
         LIMIT ?3",
        params![key, upper_bound(key), limit as i64],
    )
}

pub fn containing(
    connection: &Connection,
    key: &str,
    limit: usize,
) -> Result<Vec<Candidate>, SearchError> {
    read(
        connection,
        "SELECT path, title_key FROM documents
         WHERE title_key LIKE ?1 ESCAPE '\\'
           AND NOT (title_key >= ?2 AND title_key < ?3)
         ORDER BY length(title_key), title_key
         LIMIT ?4",
        params![
            format!("%{}%", escape_like(key)),
            key,
            upper_bound(key),
            limit as i64
        ],
    )
}

fn read(
    connection: &Connection,
    sql: &str,
    parameters: impl rusqlite::Params,
) -> Result<Vec<Candidate>, SearchError> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map(parameters, |row| {
            Ok(Candidate {
                path: row.get(0)?,
                title_key: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn upper_bound(key: &str) -> String {
    format!("{key}\u{10FFFF}")
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
