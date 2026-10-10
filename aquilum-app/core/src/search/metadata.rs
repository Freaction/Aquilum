use super::created::{CreatedAt, CreatedSource};
use super::error::SearchError;
use rusqlite::{Connection, Row, Transaction};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileState {
    pub fingerprint: Fingerprint,
    pub content_hash: Vec<u8>,
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
           content_hash BLOB NOT NULL,
           analyzer_version INTEGER NOT NULL DEFAULT 0,
           created_ns INTEGER NOT NULL DEFAULT 0,
           created_source INTEGER NOT NULL DEFAULT 0
          ) WITHOUT ROWID";

const ADD_ANALYZER_VERSION: &str =
    "ALTER TABLE documents ADD COLUMN analyzer_version INTEGER NOT NULL DEFAULT 0";

const ADD_CREATED_COLUMNS: &str =
    "ALTER TABLE documents ADD COLUMN created_ns INTEGER NOT NULL DEFAULT 0;
     ALTER TABLE documents ADD COLUMN created_source INTEGER NOT NULL DEFAULT 0";

const REINDEX_FOR_FIELDS: &str = "UPDATE documents SET analyzer_version=0";

pub fn open(path: &Path) -> Result<Connection, SearchError> {
    let connection = Connection::open(path)?;
    let version = connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))?;
    let auto_vacuum = connection.query_row("PRAGMA auto_vacuum", [], |row| row.get::<_, i64>(0))?;
    if auto_vacuum != 2 {
        connection.execute_batch("PRAGMA auto_vacuum=INCREMENTAL; VACUUM;")?;
    }
    connection.execute_batch(&format!(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=3000;
         PRAGMA wal_autocheckpoint=1000;
         PRAGMA journal_size_limit=33554432;
         {DOCUMENTS_DDL};"
    ))?;
    if version < 9 {
        connection.execute_batch(&format!(
            "DROP TABLE IF EXISTS wiki_links;
             DROP TABLE IF EXISTS wiki_documents;
             DROP TABLE IF EXISTS note_fields;
             DROP TABLE IF EXISTS base_note_fields;
             DROP TABLE IF EXISTS documents;
             {DOCUMENTS_DDL};
             PRAGMA user_version={SCHEMA_VERSION};"
        ))?;
        connection.execute_batch("VACUUM;")?;
    } else if version < SCHEMA_VERSION {
        if version < 10 {
            connection.execute_batch(ADD_ANALYZER_VERSION)?;
        }
        if version < 11 {
            connection.execute_batch(ADD_CREATED_COLUMNS)?;
        }
        if version < 13 {
            connection.execute_batch(REINDEX_FOR_FIELDS)?;
        }
        connection.execute_batch(&format!("PRAGMA user_version={SCHEMA_VERSION};"))?;
    }
    super::wiki::open_schema(&connection)?;
    super::fields::open_schema(&connection)?;
    super::tasks::open_schema(&connection)?;
    Ok(connection)
}

pub fn maintain(connection: &Connection) -> Result<(), SearchError> {
    let free = connection.query_row("PRAGMA freelist_count", [], |row| row.get::<_, i64>(0))?;
    if free > 0 {
        connection.execute_batch("PRAGMA incremental_vacuum(256);")?;
    }
    connection.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")?;
    Ok(())
}

const FILE_STATE_COLUMNS: &str =
    "modified_ns, size, content_hash, analyzer_version, created_ns, created_source";

fn file_state(row: &Row<'_>, first: usize) -> rusqlite::Result<FileState> {
    Ok(FileState {
        fingerprint: (row.get(first)?, row.get(first + 1)?),
        content_hash: row.get(first + 2)?,
        analyzer_version: row.get::<_, i64>(first + 3)? as u32,
        created: CreatedAt {
            nanos: row.get(first + 4)?,
            source: CreatedSource::from_stored(row.get(first + 5)?),
        },
    })
}

pub fn load(connection: &Connection) -> Result<HashMap<PathKey, FileState>, SearchError> {
    let mut statement =
        connection.prepare(&format!("SELECT path, {FILE_STATE_COLUMNS} FROM documents"))?;
    let rows = statement.query_map([], |row| {
        Ok((key(&row.get::<_, String>(0)?), file_state(row, 1)?))
    })?;
    let mut metadata = HashMap::new();
    for row in rows {
        let (path_key, fingerprint) = row?;
        metadata.insert(path_key, fingerprint);
    }
    Ok(metadata)
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

pub fn current(
    transaction: &Transaction<'_>,
    path: &str,
) -> Result<Option<FileState>, SearchError> {
    let mut statement = transaction.prepare(&format!(
        "SELECT {FILE_STATE_COLUMNS} FROM documents WHERE path=?1"
    ))?;
    let mut rows = statement.query([path])?;
    match rows.next()? {
        Some(row) => Ok(Some(file_state(row, 0)?)),
        None => Ok(None),
    }
}

pub fn paths_after(
    transaction: &Transaction<'_>,
    after: &str,
    limit: usize,
) -> Result<Vec<String>, SearchError> {
    let mut statement =
        transaction.prepare("SELECT path FROM documents WHERE path > ?1 ORDER BY path LIMIT ?2")?;
    let rows = statement.query_map((after, limit as i64), |row| row.get(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn key(path: &str) -> PathKey {
    let hash = blake3::hash(path.as_bytes());
    let mut key = [0; 16];
    key.copy_from_slice(&hash.as_bytes()[..16]);
    key
}

#[cfg(test)]
mod tests {
    use super::{key, load, open, SCHEMA_VERSION};
    use crate::search::created::CreatedSource;
    use rusqlite::Connection;

    const SCHEMA_V10: &str = "CREATE TABLE documents (
           path TEXT PRIMARY KEY,
           modified_ns INTEGER NOT NULL,
           size INTEGER NOT NULL,
           content_hash BLOB NOT NULL,
           analyzer_version INTEGER NOT NULL DEFAULT 0
          ) WITHOUT ROWID;
         PRAGMA user_version=10;";

    #[test]
    fn upgrade_keeps_documents_and_queues_them_for_reindexing() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("documents.sqlite3");
        let legacy = Connection::open(&path).expect("legacy database");
        legacy.execute_batch(SCHEMA_V10).expect("legacy schema");
        legacy
            .execute(
                "INSERT INTO documents VALUES('C:/vault/note.md', 42, 7, x'00', 3)",
                [],
            )
            .expect("legacy row");
        drop(legacy);

        let connection = open(&path).expect("upgraded database");
        let version = connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .expect("schema version");
        let known = load(&connection).expect("stored file states");
        let state = known
            .get(&key("C:/vault/note.md"))
            .expect("row survives the upgrade");

        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(state.fingerprint, (42, 7));
        assert_eq!(
            state.analyzer_version, 0,
            "схема 13 сбрасывает версию анализатора, чтобы заполнить типизированные поля Bases"
        );
        assert_eq!(state.created.nanos, 0);
        assert_eq!(state.created.source, CreatedSource::ModifiedTime);
    }

    #[test]
    fn upgrade_from_twelve_reindexes_notes_for_typed_bases_without_losing_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("metadata.sqlite3");
        let connection = open(&path).unwrap();
        connection.execute("INSERT INTO documents(path, modified_ns, size, content_hash, analyzer_version) VALUES('/vault/note.md', 42, 7, x'00', 10)", []).unwrap();
        connection
            .execute_batch("DROP TABLE base_note_fields; PRAGMA user_version=12;")
            .unwrap();
        drop(connection);
        let connection = open(&path).unwrap();
        let state = load(&connection)
            .unwrap()
            .remove(&key("/vault/note.md"))
            .unwrap();
        assert_eq!(state.fingerprint, (42, 7));
        assert_eq!(state.analyzer_version, 0);
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM base_note_fields", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}
