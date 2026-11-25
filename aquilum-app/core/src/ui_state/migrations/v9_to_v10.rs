use super::super::error::UiStateError;
use crate::search::paths::{identity, normalize_extended_prefix};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn migrate_v9_to_v10(connection: &Connection) -> Result<(), UiStateError> {
    let transaction = connection.unchecked_transaction()?;
    let rows = transaction
        .prepare("SELECT id, path FROM workspaces ORDER BY last_seen_ms DESC")?
        .query_map([], |row| <(String, String)>::try_from(row))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = HashSet::new();
    let (kept, doubles): (Vec<_>, Vec<_>) = rows
        .into_iter()
        .map(|(id, path)| (id, normalize_extended_prefix(PathBuf::from(path)).to_string_lossy().into_owned()))
        .partition(|(_, path)| seen.insert(identity(Path::new(path))));
    for (id, _) in doubles {
        transaction.execute("DELETE FROM workspaces WHERE id = ?1", [id])?;
    }
    for (id, path) in kept {
        transaction.execute("UPDATE workspaces SET path = ?2 WHERE id = ?1", params![id, path])?;
    }
    transaction.execute_batch("PRAGMA user_version = 10;")?;
    Ok(transaction.commit()?)
}

#[cfg(test)]
mod tests {
    use super::super::{migrate, schema::SCHEMA};

    #[test]
    fn verbatim_doubles_collapse_into_the_newest_row_without_the_prefix() {
        let connection = crate::sqlite::memory();
        connection.execute_batch(SCHEMA).unwrap();
        connection
            .execute_batch(
                r"PRAGMA user_version = 9;
                  INSERT INTO workspaces(id, path, last_seen_ms) VALUES
                      ('new', 'D:\Notes', 5), ('old', '\\?\D:\Notes', 1), ('only', '\\?\E:\Archive', 3), ('share', '\\?\UNC\server\share', 2);
                  INSERT INTO documents(id, workspace_id, relative_path) VALUES ('kept', 'new', 'a.md'), ('dropped', 'old', 'a.md');",
            )
            .unwrap();

        migrate(&connection).unwrap();

        let rows = connection
            .prepare("SELECT id, path FROM workspaces ORDER BY last_seen_ms DESC")
            .unwrap()
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let expected = [("new", r"D:\Notes"), ("only", r"E:\Archive"), ("share", r"\\server\share")].map(|(id, path)| (id.to_owned(), path.to_owned()));
        assert_eq!(rows, expected);
        let documents: String = connection.query_row("SELECT group_concat(id) FROM documents", [], |row| row.get(0)).unwrap();
        assert_eq!(documents, "kept", "состояние двойника ушло вместе с ним");
    }
}
