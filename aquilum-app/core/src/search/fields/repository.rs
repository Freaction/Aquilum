use super::inline::indexed_fields;
use super::value::{key_lower, value_lower, FieldKind};
use crate::search::error::SearchError;
use rusqlite::{params, Connection, Transaction};
use std::path::Path;

pub fn open_schema(connection: &Connection) -> Result<(), SearchError> {
    super::bases::open_schema(connection)?;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS note_fields (
           path TEXT NOT NULL,
           ordinal INTEGER NOT NULL,
           key TEXT NOT NULL,
           key_lower TEXT NOT NULL,
           kind INTEGER NOT NULL,
           text TEXT NOT NULL,
           text_lower TEXT NOT NULL,
           items TEXT,
           PRIMARY KEY(path, ordinal)
         ) WITHOUT ROWID;
         CREATE INDEX IF NOT EXISTS note_fields_lookup ON note_fields(key_lower, text_lower);",
    )?;
    Ok(())
}

pub fn index_document(
    transaction: &Transaction<'_>,
    path: &Path,
    body: &str,
) -> Result<(), SearchError> {
    remove_document(transaction, path)?;
    super::bases::index(transaction, path, body)?;
    let found = indexed_fields(body);
    if found.is_empty() {
        return Ok(());
    }
    let path_text = path.to_string_lossy();
    let mut statement = transaction.prepare(
        "INSERT INTO note_fields(path, ordinal, key, key_lower, kind, text, text_lower, items)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    for (ordinal, field) in found.iter().enumerate() {
        let items = (field.kind == FieldKind::List)
            .then(|| serde_json::to_string(&field.items).unwrap_or_else(|_| "[]".to_owned()));
        statement.execute(params![
            path_text,
            ordinal as i64,
            field.key,
            key_lower(&field.key),
            field.kind.stored(),
            field.text,
            value_lower(&field.text),
            items,
        ])?;
    }
    Ok(())
}

pub fn remove_document(transaction: &Transaction<'_>, path: &Path) -> Result<(), SearchError> {
    super::bases::remove(transaction, path)?;
    transaction.execute(
        "DELETE FROM note_fields WHERE path=?1",
        [path.to_string_lossy().as_ref()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{index_document, open_schema, remove_document};
    use crate::search::fields::query;
    use rusqlite::Connection;
    use std::path::Path;

    const BOOK: &str = "---\ntype: book\nauthor: Иван Ефремов\ntags: [fiction, classic]\n---\nтело";

    fn count(connection: &Connection) -> i64 {
        connection
            .query_row("SELECT COUNT(*) FROM note_fields", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn reindexing_replaces_fields_and_removal_clears_them() {
        let mut connection = Connection::open_in_memory().unwrap();
        open_schema(&connection).unwrap();
        let path = Path::new("C:/vault/Книга.md");

        let transaction = connection.transaction().unwrap();
        index_document(&transaction, path, BOOK).unwrap();
        transaction.commit().unwrap();
        assert_eq!(count(&connection), 3);

        let transaction = connection.transaction().unwrap();
        index_document(&transaction, path, "---\ntype: note\n---\n").unwrap();
        transaction.commit().unwrap();
        assert_eq!(count(&connection), 1, "поля прежней версии заметки удалены");

        let transaction = connection.transaction().unwrap();
        remove_document(&transaction, path).unwrap();
        transaction.commit().unwrap();
        assert_eq!(count(&connection), 0);
    }

    #[test]
    fn a_note_without_frontmatter_leaves_no_rows() {
        let mut connection = Connection::open_in_memory().unwrap();
        open_schema(&connection).unwrap();
        let transaction = connection.transaction().unwrap();
        index_document(
            &transaction,
            Path::new("C:/vault/Заметка.md"),
            "просто текст",
        )
        .unwrap();
        transaction.commit().unwrap();
        assert_eq!(count(&connection), 0);
    }

    #[test]
    fn stored_fields_come_back_with_their_type_and_list_items() {
        let mut connection = Connection::open_in_memory().unwrap();
        open_schema(&connection).unwrap();
        let path = "C:/vault/Книга.md".to_owned();
        let transaction = connection.transaction().unwrap();
        index_document(&transaction, Path::new(&path), BOOK).unwrap();
        transaction.commit().unwrap();

        let read = query::read(&connection, std::slice::from_ref(&path)).unwrap();
        let fields = read.get(&path).expect("поля заметки прочитаны");
        assert_eq!(fields.len(), 3, "порядок и состав полей сохранены");
        assert_eq!(fields[0].key, "type");
        assert_eq!(fields[1].text, "Иван Ефремов");
        assert_eq!(fields[2].items, vec!["fiction", "classic"]);
    }

    #[test]
    fn lookup_narrows_by_key_and_value_ignoring_case() {
        let mut connection = Connection::open_in_memory().unwrap();
        open_schema(&connection).unwrap();
        let transaction = connection.transaction().unwrap();
        index_document(&transaction, Path::new("C:/vault/Книга.md"), BOOK).unwrap();
        index_document(
            &transaction,
            Path::new("C:/vault/Статья.md"),
            "---\ntype: article\nauthor: \n---\n",
        )
        .unwrap();
        transaction.commit().unwrap();

        let equals = vec![("TYPE".to_owned(), "Book".to_owned())];
        let found = query::candidates(&connection, &equals, &[]).unwrap();
        assert_eq!(found, vec!["C:/vault/Книга.md".to_owned()]);

        let has = vec!["author".to_owned()];
        let filled = query::candidates(&connection, &[], &has).unwrap();
        assert_eq!(
            filled,
            vec!["C:/vault/Книга.md".to_owned()],
            "пустое значение не считается заполненным"
        );
    }
}
