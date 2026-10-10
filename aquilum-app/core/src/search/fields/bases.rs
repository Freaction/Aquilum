use crate::bases::{note, BaseRow};
use crate::search::error::SearchError;
use crate::search::paths::relative_slash_path;
use rusqlite::{Connection, Transaction};
use serde_json::{Map, Value};
use std::path::Path;

pub(super) fn open_schema(connection: &Connection) -> Result<(), SearchError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS base_note_fields (
        path TEXT PRIMARY KEY, fields TEXT, tags TEXT, error TEXT
    ) WITHOUT ROWID;",
    )?;
    Ok(())
}

pub(super) fn index(
    transaction: &Transaction<'_>,
    path: &Path,
    text: &str,
) -> Result<(), SearchError> {
    let (fields, tags, error) = match note::fields(text) {
        Ok(fields) => (
            Some(serde_json::to_string(&fields).map_err(SearchError::task)?),
            Some(serde_json::to_string(&note::tags(text, &fields)).map_err(SearchError::task)?),
            None,
        ),
        Err(error) => (None, None, Some(error.to_string())),
    };
    transaction.execute(
        "INSERT INTO base_note_fields(path, fields, tags, error) VALUES(?1, ?2, ?3, ?4)",
        rusqlite::params![path.to_string_lossy(), fields, tags, error],
    )?;
    Ok(())
}

pub(super) fn remove(transaction: &Transaction<'_>, path: &Path) -> Result<(), SearchError> {
    transaction.execute(
        "DELETE FROM base_note_fields WHERE path=?1",
        [path.to_string_lossy().as_ref()],
    )?;
    Ok(())
}

pub(super) fn read(connection: &Connection, root: &Path) -> Result<Vec<BaseRow>, SearchError> {
    let mut statement = connection
        .prepare("SELECT path, fields, tags, error FROM base_note_fields ORDER BY path")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut found = Vec::new();
    for row in rows {
        let (path, fields, tags, error) = row?;
        if let Some(error) = error {
            return Err(SearchError::Query {
                message: format!("{path}: {error}"),
            });
        }
        let fields: Map<String, Value> =
            serde_json::from_str(fields.as_deref().unwrap_or("{}")).map_err(SearchError::task)?;
        let file_tags =
            serde_json::from_str(tags.as_deref().unwrap_or("[]")).map_err(SearchError::task)?;
        found.push(BaseRow {
            path: relative_slash_path(root, Path::new(&path)),
            fields,
            file_tags,
        });
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::fields::repository;
    use serde_json::json;

    #[test]
    fn typed_base_rows_survive_index_storage_and_reindexing() {
        let mut connection = Connection::open_in_memory().unwrap();
        repository::open_schema(&connection).unwrap();
        let path = Path::new("/vault/a.md");
        let transaction = connection.transaction().unwrap();
        repository::index_document(&transaction, path, "---\nquoted: \"2\"\nnumber: 2\nbool_text: 'true'\nbool_value: true\ntags: [project]\n---\n#body/nested\n").unwrap();
        transaction.commit().unwrap();
        let rows = read(&connection, Path::new("/vault")).unwrap();
        assert_eq!(rows[0].path, "a.md");
        assert_eq!(rows[0].fields["quoted"], json!("2"));
        assert_eq!(rows[0].fields["number"], json!(2));
        assert_eq!(rows[0].fields["bool_text"], json!("true"));
        assert_eq!(rows[0].fields["bool_value"], json!(true));
        assert_eq!(rows[0].fields["tags"], json!(["project"]));
        assert_eq!(rows[0].file_tags, vec!["project", "body/nested"]);
        let transaction = connection.transaction().unwrap();
        repository::index_document(&transaction, path, "---\nx: [broken\n---\n").unwrap();
        transaction.commit().unwrap();
        assert!(matches!(
            read(&connection, Path::new("/vault")),
            Err(SearchError::Query { .. })
        ));
        let transaction = connection.transaction().unwrap();
        repository::remove_document(&transaction, path).unwrap();
        transaction.commit().unwrap();
        assert!(read(&connection, Path::new("/vault")).unwrap().is_empty());
    }
}
