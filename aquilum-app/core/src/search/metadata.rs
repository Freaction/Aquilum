use super::created::{CreatedAt, CreatedSource};
use super::error::SearchError;
use rusqlite::{Connection, OptionalExtension, Row, Transaction};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileState {
    pub fingerprint: Fingerprint,
    pub analyzer_version: u32,
    pub created: CreatedAt,
}

pub type Fingerprint = (i64, i64);
pub type PathKey = [u8; 16];

const SCHEMA_VERSION: i64 = 13;

const DOCUMENTS_DDL: &str = "CREATE TABLE IF NOT EXISTS documents (
           path TEXT PRIMARY KEY,
           modified_ns INTEGER NOT NULL,
           size INTEGER NOT NULL,
           analyzer_version INTEGER NOT NULL DEFAULT 0,
           created_ns INTEGER NOT NULL DEFAULT 0,
           created_source INTEGER NOT NULL DEFAULT 0
          ) WITHOUT ROWID";

pub fn open(path: &Path) -> Result<Connection, SearchError> {
    let connection = Connection::open(path)?;
    let version = connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))?;
    let auto_vacuum = connection.query_row("PRAGMA auto_vacuum", [], |row| row.get::<_, i64>(0))?;
    if auto_vacuum != 2 {
        connection.execute_batch("PRAGMA auto_vacuum=INCREMENTAL; VACUUM;")?;
    }
    connection.execute_batch(crate::SQLITE_PRAGMAS)?;
    if version < SCHEMA_VERSION {
        connection.execute_batch(&format!(
            "DROP TABLE IF EXISTS wiki_links;
             DROP TABLE IF EXISTS wiki_documents;
             DROP TABLE IF EXISTS note_fields;
             DROP TABLE IF EXISTS note_tasks;
             DROP TABLE IF EXISTS note_pages;
             DROP TABLE IF EXISTS documents;
             PRAGMA user_version={SCHEMA_VERSION};
             VACUUM;"
        ))?;
    }
    connection.execute_batch(&format!("{DOCUMENTS_DDL}; {}; {}; {}; {};", super::pages::PAGES_DDL, super::wiki::WIKI_DDL, super::fields::FIELDS_DDL, super::tasks::TASKS_DDL))?;
    Ok(connection)
}

#[cfg(test)]
pub fn memory() -> Connection {
    open(Path::new(":memory:")).unwrap()
}

pub fn for_paths<T>(connection: &Connection, paths: &[String], sql: impl Fn(&str) -> String, row: impl Fn(&Row<'_>) -> rusqlite::Result<T>, mut visit: impl FnMut(T)) -> Result<(), SearchError> {
    for chunk in paths.chunks(400) {
        let mut statement = connection.prepare(&sql(&vec!["?"; chunk.len()].join(",")))?;
        for item in statement.query_map(rusqlite::params_from_iter(chunk), &row)? {
            visit(item?);
        }
    }
    Ok(())
}

pub fn maintain(connection: &Connection) -> Result<(), SearchError> {
    let free = connection.query_row("PRAGMA freelist_count", [], |row| row.get::<_, i64>(0))?;
    if free > 0 {
        connection.execute_batch("PRAGMA incremental_vacuum(256);")?;
    }
    connection.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")?;
    Ok(())
}

const FILE_STATE_COLUMNS: &str = "modified_ns, size, analyzer_version, created_ns, created_source";

fn file_state(row: &Row<'_>, first: usize) -> rusqlite::Result<FileState> {
    Ok(FileState {
        fingerprint: (row.get(first)?, row.get(first + 1)?),
        analyzer_version: row.get::<_, i64>(first + 2)? as u32,
        created: CreatedAt {
            nanos: row.get(first + 3)?,
            source: CreatedSource::from_stored(row.get(first + 4)?),
        },
    })
}

pub fn load(connection: &Connection) -> Result<HashMap<PathKey, FileState>, SearchError> {
    let mut statement = connection.prepare(&format!("SELECT path, {FILE_STATE_COLUMNS} FROM documents"))?;
    let rows = statement.query_map([], |row| Ok((key(&row.get::<_, String>(0)?), file_state(row, 1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn fingerprint(metadata: &fs::Metadata) -> Fingerprint {
    let modified_ns = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or_default();
    (modified_ns, metadata.len().min(i64::MAX as u64) as i64)
}

pub fn current(transaction: &Transaction<'_>, path: &str) -> Result<Option<FileState>, SearchError> {
    let sql = format!("SELECT {FILE_STATE_COLUMNS} FROM documents WHERE path=?1");
    Ok(transaction.query_row(&sql, [path], |row| file_state(row, 0)).optional()?)
}

pub fn paths_after(transaction: &Transaction<'_>, after: &str, limit: usize) -> Result<Vec<String>, SearchError> {
    let mut statement = transaction.prepare("SELECT path FROM documents WHERE path > ?1 ORDER BY path LIMIT ?2")?;
    let rows = statement.query_map((after, limit as i64), |row| row.get(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn paths_under(transaction: &Transaction<'_>, path: &str) -> Result<Vec<String>, SearchError> {
    let separator = std::path::MAIN_SEPARATOR;
    let (from, to) = (format!("{path}{separator}"), format!("{path}{}", char::from(separator as u8 + 1)));
    let mut statement = transaction.prepare("SELECT path FROM documents WHERE path = ?1 OR (path > ?2 AND path < ?3)")?;
    let rows = statement.query_map((path, from, to), |row| row.get(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn key(path: &str) -> PathKey {
    let hash = blake3::hash(path.as_bytes());
    std::array::from_fn(|index| hash.as_bytes()[index])
}

#[cfg(test)]
mod tests {
    use super::{load, open, SCHEMA_VERSION};
    use rusqlite::Connection;

    #[test]
    fn an_older_schema_is_rebuilt_and_the_vault_reindexed() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("documents.sqlite3");
        let legacy = Connection::open(&path).expect("legacy database");
        legacy
            .execute_batch("CREATE TABLE documents (path TEXT PRIMARY KEY, content_hash BLOB) WITHOUT ROWID; INSERT INTO documents VALUES('C:/vault/note.md', x'00'); PRAGMA user_version=12;")
            .expect("legacy schema");
        drop(legacy);
        let connection = open(&path).expect("upgraded database");
        let version = connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0)).expect("schema version");
        assert_eq!(version, SCHEMA_VERSION);
        assert!(load(&connection).expect("stored file states").is_empty());
    }
}
