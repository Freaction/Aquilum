use rusqlite::Connection;
use std::path::Path;

const PRAGMAS: &str = "PRAGMA journal_mode = WAL;
     PRAGMA synchronous = NORMAL;
     PRAGMA foreign_keys = ON;
     PRAGMA busy_timeout = 3000;
     PRAGMA journal_size_limit = 4194304;";

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    if connection.query_row("PRAGMA auto_vacuum", [], |row| row.get::<_, i64>(0))? != 2 {
        connection.execute_batch("PRAGMA auto_vacuum = INCREMENTAL; VACUUM;")?;
    }
    connection.execute_batch(PRAGMAS)?;
    Ok(connection)
}

#[cfg(test)]
pub fn memory() -> Connection {
    open(Path::new(":memory:")).expect("база в памяти")
}

pub fn maintain(connection: &Connection) -> rusqlite::Result<()> {
    connection.pragma(None, "incremental_vacuum", 0, |_| Ok(()))?;
    connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
}

#[cfg(test)]
mod tests {
    use super::{maintain, open};

    fn pragma(connection: &rusqlite::Connection, name: &str) -> i64 {
        connection.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn an_old_database_gains_incremental_vacuum_and_gives_back_every_free_page() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("old.sqlite3");
        let old = rusqlite::Connection::open(&path).unwrap();
        old.execute_batch("PRAGMA journal_mode = WAL; CREATE TABLE notes (text TEXT); INSERT INTO notes VALUES ('x');").unwrap();
        drop(old);

        let connection = open(&path).unwrap();
        assert_eq!(pragma(&connection, "auto_vacuum"), 2);
        connection.execute_batch("WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 2000) INSERT INTO notes SELECT zeroblob(1000) FROM n; DELETE FROM notes;").unwrap();
        assert!(pragma(&connection, "freelist_count") > 400);

        maintain(&connection).unwrap();
        assert_eq!(pragma(&connection, "freelist_count"), 0);
    }
}
