use super::remote::RemoteFile;
use super::scan::key;
use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::HashMap;
use std::path::Path;

const SELECT: &str = "SELECT path, hash, size, modified, remote_id, remote_rev, checksum, remote_size, remote_modified FROM records";

const RECORDS: &str = "CREATE TABLE IF NOT EXISTS records (
         key TEXT PRIMARY KEY,
         path TEXT NOT NULL,
         hash TEXT NOT NULL,
         size INTEGER NOT NULL,
         modified INTEGER NOT NULL,
         remote_id TEXT NOT NULL,
         remote_rev TEXT NOT NULL,
         checksum TEXT NOT NULL,
         remote_size INTEGER NOT NULL,
         remote_modified INTEGER NOT NULL
     ) WITHOUT ROWID";

pub struct Record {
    pub hash: String,
    pub size: u64,
    pub modified: u64,
    pub remote: RemoteFile,
}

pub struct Records {
    connection: Connection,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Memory {
    pub done: Option<u64>,
    pub large: Vec<Large>,
    pub size: Size,
    pub allowed: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Large {
    pub path: String,
    pub size: u64,
    pub kept: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub synced: u64,
    pub files: u64,
}

impl Records {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let connection = crate::sqlite::open(path)?;
        connection.execute_batch(&format!("{RECORDS}; CREATE TABLE IF NOT EXISTS state (key TEXT PRIMARY KEY, value TEXT NOT NULL);"))?;
        if connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))? < 2 {
            keyed(&connection)?;
        }
        Ok(Self { connection })
    }

    #[cfg(test)]
    pub fn memory() -> rusqlite::Result<Self> {
        Self::open(Path::new(":memory:"))
    }

    pub fn all(&self) -> rusqlite::Result<HashMap<String, Record>> {
        let mut statement = self.connection.prepare(SELECT)?;
        let mut found = HashMap::new();
        for record in statement.query_map([], record)? {
            let record = record?;
            found.insert(key(&record.remote.path), record);
        }
        Ok(found)
    }

    pub fn get(&self, path: &str) -> rusqlite::Result<Option<Record>> {
        self.connection.prepare_cached(&format!("{SELECT} WHERE key = ?1"))?.query_row([key(path)], record).optional()
    }

    pub fn holds_under(&self, folder: &str) -> rusqlite::Result<bool> {
        let folder = key(folder);
        self.connection.query_row("SELECT EXISTS(SELECT 1 FROM records WHERE key > ?1 AND key < ?2)", [format!("{folder}/"), format!("{folder}0")], |row| row.get(0))
    }

    pub fn put(&self, record: &Record) -> rusqlite::Result<()> {
        insert(&self.connection, record)
    }

    pub fn remove(&self, path: &str) -> rusqlite::Result<()> {
        self.connection.prepare_cached("DELETE FROM records WHERE key = ?1")?.execute([key(path)]).map(|_| ())
    }

    pub fn forget_stat(&self, path: &str) -> rusqlite::Result<()> {
        self.connection.prepare_cached("UPDATE records SET size = 0, modified = 0 WHERE key = ?1")?.execute([key(path)]).map(|_| ())
    }

    pub fn forget_cursor(&self) -> rusqlite::Result<()> {
        self.connection.execute("DELETE FROM state WHERE key = 'cursor'", []).map(|_| ())
    }

    pub fn state(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.connection.query_row("SELECT value FROM state WHERE key = ?1", [key], |row| row.get(0)).optional()
    }

    pub fn recall(&self) -> Memory {
        self.state("memory").ok().flatten().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default()
    }

    pub fn remember(&self, memory: &Memory) -> rusqlite::Result<()> {
        let json = serde_json::to_string(memory).unwrap_or_default();
        self.connection.execute("INSERT OR REPLACE INTO state VALUES ('memory', ?1)", [json]).map(|_| ())
    }

    pub fn commit(&self, cursor: &str, scope: &str) -> rusqlite::Result<()> {
        self.connection.execute("INSERT OR REPLACE INTO state VALUES ('cursor', ?1), ('scope', ?2)", [cursor, scope])?;
        crate::sqlite::maintain(&self.connection)
    }
}

fn insert(connection: &Connection, record: &Record) -> rusqlite::Result<()> {
    let remote = &record.remote;
    let insert = "INSERT OR REPLACE INTO records (key, path, hash, size, modified, remote_id, remote_rev, checksum, remote_size, remote_modified) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)";
    connection.prepare_cached(insert)?.execute(params![
        key(&remote.path),
        remote.path,
        record.hash,
        record.size as i64,
        record.modified as i64,
        remote.id,
        remote.rev,
        remote.checksum,
        remote.size as i64,
        remote.modified as i64
    ])?;
    Ok(())
}

fn keyed(connection: &Connection) -> rusqlite::Result<()> {
    let transaction = connection.unchecked_transaction()?;
    let old = transaction.prepare(SELECT)?.query_map([], record)?.collect::<rusqlite::Result<Vec<_>>>()?;
    transaction.execute_batch(&format!("DROP TABLE records; {RECORDS}; PRAGMA user_version = 2;"))?;
    for record in &old {
        insert(&transaction, record)?;
    }
    transaction.commit()
}

fn record(row: &Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        hash: row.get(1)?,
        size: row.get::<_, i64>(2)? as u64,
        modified: row.get::<_, i64>(3)? as u64,
        remote: RemoteFile {
            path: row.get(0)?,
            id: row.get(4)?,
            rev: row.get(5)?,
            checksum: row.get(6)?,
            size: row.get::<_, i64>(7)? as u64,
            modified: row.get::<_, i64>(8)? as u64,
        },
    })
}
