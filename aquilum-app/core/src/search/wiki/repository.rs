use super::super::error::SearchError;
use super::parser::extract;
use super::target::{relative_key, target_key, title_key};
use rusqlite::{params, Connection, Transaction};
use std::path::Path;

pub const WIKI_DDL: &str = "CREATE TABLE IF NOT EXISTS wiki_links (
           page INTEGER NOT NULL REFERENCES note_pages(id) ON DELETE CASCADE,
           target_key TEXT NOT NULL,
           target_kind TEXT NOT NULL,
           byte_offset INTEGER NOT NULL,
           offset_utf16 INTEGER NOT NULL,
           PRIMARY KEY(page, byte_offset)
         ) WITHOUT ROWID;
         CREATE INDEX IF NOT EXISTS wiki_links_target ON wiki_links(target_kind, target_key)";

pub fn insert_links(transaction: &Transaction<'_>, page: i64, text: &str) -> Result<(), SearchError> {
    let mut statement = transaction.prepare_cached("INSERT INTO wiki_links VALUES(?1, ?2, ?3, ?4, ?5)")?;
    for link in extract(text) {
        let (target_key, target_kind) = target_key(&link.target);
        statement.execute(params![page, target_key, target_kind, link.note_range.start as i64, link.offset_utf16 as i64])?;
    }
    Ok(())
}

pub fn candidate_sources(
    connection: &Connection,
    root: &Path,
    document: &Path,
) -> Result<Vec<String>, SearchError> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT page.path FROM wiki_links link JOIN note_pages page ON page.id = link.page
         WHERE (link.target_kind='name' AND link.target_key=?1)
            OR (link.target_kind='path' AND link.target_key=?2)",
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
        crate::search::pages::remove(&transaction, &path.to_string_lossy()).unwrap();
        transaction.commit().unwrap();
        assert_eq!(count(&connection), 0);
    }
}
