use super::Page;
use rusqlite::{params, Connection, Transaction};
use std::collections::HashMap;

pub const PAGES_DDL: &str = "CREATE TABLE IF NOT EXISTS note_pages (
           id INTEGER PRIMARY KEY,
           path TEXT NOT NULL,
           key TEXT NOT NULL,
           ordinal INTEGER NOT NULL,
           start INTEGER NOT NULL,
           start_utf16 INTEGER NOT NULL,
           start_line INTEGER NOT NULL
          );
         CREATE INDEX IF NOT EXISTS note_pages_path ON note_pages(path)";

pub fn key(path: &str, page: &Page, repeat: usize) -> String {
    format!("{path}\u{1f}{}\u{1f}{repeat}{}", page.hash.to_hex(), if page.first { "^" } else { "" })
}

pub fn stored(transaction: &Transaction<'_>, path: &str) -> rusqlite::Result<HashMap<String, i64>> {
    let mut statement = transaction.prepare_cached("SELECT key, id FROM note_pages WHERE path=?1")?;
    let keys = statement.query_map([path], |row| Ok((row.get(0)?, row.get(1)?)))?.collect();
    keys
}

pub fn insert(transaction: &Transaction<'_>, path: &str, key: &str, ordinal: usize, page: &Page) -> rusqlite::Result<i64> {
    let mut statement = transaction.prepare_cached("INSERT INTO note_pages(path, key, ordinal, start, start_utf16, start_line) VALUES(?1, ?2, ?3, ?4, ?5, ?6)")?;
    statement.execute(params![path, key, ordinal as i64, page.start as i64, page.start_utf16 as i64, page.start_line as i64])?;
    Ok(transaction.last_insert_rowid())
}

pub fn place(transaction: &Transaction<'_>, id: i64, ordinal: usize, page: &Page) -> rusqlite::Result<()> {
    let mut statement = transaction.prepare_cached("UPDATE note_pages SET ordinal=?2, start=?3, start_utf16=?4, start_line=?5 WHERE id=?1 AND (ordinal, start, start_utf16, start_line) <> (?2, ?3, ?4, ?5)")?;
    statement.execute(params![id, ordinal as i64, page.start as i64, page.start_utf16 as i64, page.start_line as i64]).map(|_| ())
}

pub fn delete(transaction: &Transaction<'_>, id: i64) -> rusqlite::Result<()> {
    transaction.execute("DELETE FROM note_pages WHERE id=?1", [id]).map(|_| ())
}

pub fn remove(transaction: &Transaction<'_>, path: &str) -> rusqlite::Result<()> {
    transaction.execute("DELETE FROM note_pages WHERE path=?1", [path]).map(|_| ())
}

pub fn count(connection: &Connection) -> rusqlite::Result<u64> {
    connection.query_row("SELECT COUNT(*) FROM note_pages", [], |row| row.get::<_, i64>(0)).map(|count| count as u64)
}
