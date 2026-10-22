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

#[derive(Default)]
pub struct Keys(HashMap<blake3::Hash, usize>);

impl Keys {
    pub fn next(&mut self, path: &str, page: &Page) -> String {
        let hash = blake3::hash(page.text.as_bytes());
        let repeat = self.0.entry(hash).or_insert(0);
        *repeat += 1;
        format!("{path}\u{1f}{}\u{1f}{}{}", hash.to_hex(), *repeat - 1, if page.first { "^" } else { "" })
    }
}

pub fn prefix(path: &str) -> String {
    format!("{path}\u{1f}")
}

pub fn key_hash(key: &str) -> Option<blake3::Hash> {
    key.rsplit('\u{1f}').nth(1).and_then(|hex| blake3::Hash::from_hex(hex).ok())
}

pub struct Placed {
    pub key: String,
    pub start: usize,
    pub start_utf16: usize,
}

pub fn placed(connection: &Connection, path: &str) -> rusqlite::Result<Vec<Placed>> {
    let mut statement = connection.prepare_cached("SELECT key, start, start_utf16 FROM note_pages WHERE path=?1 ORDER BY ordinal")?;
    let rows = statement.query_map([path], |row| Ok(Placed { key: row.get(0)?, start: row.get::<_, i64>(1)? as usize, start_utf16: row.get::<_, i64>(2)? as usize }))?;
    rows.collect()
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
