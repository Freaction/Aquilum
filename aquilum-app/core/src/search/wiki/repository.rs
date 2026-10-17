use super::super::error::SearchError;
use super::parser::extract;
use super::target::{relative_key, target_key, title_key};
use rusqlite::{params, Connection, Transaction};
use std::path::Path;

pub const WIKI_DDL: &str = "CREATE TABLE IF NOT EXISTS wiki_documents (
           path TEXT PRIMARY KEY,
           relative_key TEXT NOT NULL,
           title_key TEXT NOT NULL
         ) WITHOUT ROWID;
         CREATE INDEX IF NOT EXISTS wiki_documents_title ON wiki_documents(title_key);
         CREATE TABLE IF NOT EXISTS wiki_links (
           page INTEGER NOT NULL REFERENCES note_pages(id) ON DELETE CASCADE,
           source_path TEXT NOT NULL REFERENCES wiki_documents(path) ON DELETE CASCADE,
           target_key TEXT NOT NULL,
           target_kind TEXT NOT NULL,
           byte_offset INTEGER NOT NULL,
           offset_utf16 INTEGER NOT NULL,
           PRIMARY KEY(page, byte_offset)
         ) WITHOUT ROWID;
         CREATE INDEX IF NOT EXISTS wiki_links_source ON wiki_links(source_path);
         CREATE INDEX IF NOT EXISTS wiki_links_target ON wiki_links(target_kind, target_key)";

pub fn register(transaction: &Transaction<'_>, root: &Path, path: &Path) -> Result<(), SearchError> {
    transaction.execute(
        "INSERT INTO wiki_documents(path, relative_key, title_key) VALUES(?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET relative_key=excluded.relative_key, title_key=excluded.title_key",
        params![path.to_string_lossy(), relative_key(root, path), title_key(path)],
    )?;
    Ok(())
}

pub fn insert_links(transaction: &Transaction<'_>, page: i64, path: &Path, text: &str) -> Result<(), SearchError> {
    let path_text = path.to_string_lossy();
    let mut statement = transaction.prepare_cached(
        "INSERT INTO wiki_links(page, source_path, target_key, target_kind, byte_offset, offset_utf16)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for link in extract(text) {
        let (target_key, target_kind) = target_key(&link.target);
        statement.execute(params![page, path_text, target_key, target_kind, link.note_range.start as i64, link.offset_utf16 as i64])?;
    }
    Ok(())
}

pub fn remove_document(transaction: &Transaction<'_>, path: &Path) -> Result<(), SearchError> {
    transaction.execute(
        "DELETE FROM wiki_documents WHERE path=?1",
        [path.to_string_lossy().as_ref()],
    )?;
    Ok(())
}

pub fn candidate_sources(
    connection: &Connection,
    root: &Path,
    document: &Path,
) -> Result<Vec<String>, SearchError> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT source_path FROM wiki_links
         WHERE (target_kind='name' AND target_key=?1)
            OR (target_kind='path' AND target_key=?2)",
    )?;
    let paths = statement
        .query_map(
            params![title_key(document), relative_key(root, document)],
            |row| row.get(0),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::remove_document;
    use rusqlite::Connection;
    use std::path::Path;

    #[test]
    fn removes_replaced_and_deleted_document_links() {
        let mut connection = crate::search::metadata::memory();
        let root = Path::new("C:/vault");
        let path = root.join("Source.md");
        let transaction = connection.transaction().unwrap();
        crate::search::index_document::index_text(&transaction, root, &path, "[[Target]]").unwrap();
        transaction.commit().unwrap();
        let count = |connection: &Connection| {
            connection
                .query_row("SELECT COUNT(*) FROM wiki_links", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap()
        };
        assert_eq!(count(&connection), 1);

        let transaction = connection.transaction().unwrap();
        crate::search::index_document::index_text(&transaction, root, &path, "plain text").unwrap();
        transaction.commit().unwrap();
        assert_eq!(count(&connection), 0);

        let transaction = connection.transaction().unwrap();
        crate::search::index_document::index_text(&transaction, root, &path, "[[Target]]").unwrap();
        remove_document(&transaction, &path).unwrap();
        transaction.commit().unwrap();
        assert_eq!(count(&connection), 0);
    }
}
