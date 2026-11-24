use super::parse::Task;
use crate::search::error::SearchError;
use rusqlite::{params, Connection, Transaction};
use std::collections::HashMap;

pub const TASKS_DDL: &str = "CREATE TABLE IF NOT EXISTS note_tasks (
           page INTEGER NOT NULL REFERENCES note_pages(id) ON DELETE CASCADE,
           ordinal INTEGER NOT NULL,
           line INTEGER NOT NULL,
           done INTEGER NOT NULL,
           text TEXT NOT NULL,
           PRIMARY KEY(page, ordinal)
         ) WITHOUT ROWID";

pub const TASKS: &str = "note_tasks task JOIN note_pages page ON page.id = task.page";

pub fn insert(transaction: &Transaction<'_>, page: i64, tasks: &[Task]) -> Result<(), SearchError> {
    let mut statement = transaction.prepare_cached("INSERT INTO note_tasks VALUES(?1, ?2, ?3, ?4, ?5)")?;
    for (ordinal, task) in tasks.iter().enumerate() {
        statement.execute(params![page, ordinal as i64, task.line as i64, i64::from(task.done), task.text])?;
    }
    Ok(())
}

pub fn read(connection: &Connection, paths: &[String]) -> Result<HashMap<String, Vec<Task>>, SearchError> {
    let mut found: HashMap<String, Vec<Task>> = HashMap::new();
    let sql = |list: &str| {
        format!(
            "SELECT page.path, page.start_line + task.line, task.done, task.text FROM {TASKS}
             WHERE page.path IN ({list}) ORDER BY page.path, page.ordinal, task.ordinal"
        )
    };
    let row = |row: &rusqlite::Row<'_>| {
        let task = Task { line: row.get::<_, i64>(1)? as usize, done: row.get::<_, i64>(2)? != 0, text: row.get(3)? };
        Ok((row.get::<_, String>(0)?, task))
    };
    crate::search::metadata::for_paths(connection, paths, sql, row, |(path, task)| found.entry(path).or_default().push(task))?;
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::read;
    use rusqlite::Connection;
    use std::path::Path;

    fn store(body: &str) -> Connection {
        let mut connection = crate::search::metadata::memory();
        let transaction = connection.transaction().expect("транзакция");
        crate::search::index_document::index_text(&transaction, std::path::Path::new("C:/vault"), Path::new("C:/vault/Дела.md"), body).expect("индексация");
        transaction.commit().expect("фиксация");
        connection
    }

    #[test]
    fn tasks_survive_a_round_trip_through_the_index() {
        let connection = store("- [ ] написать\n- [x] отправить");
        let found = read(&connection, &["C:/vault/Дела.md".to_owned()]).expect("чтение");
        let tasks = &found["C:/vault/Дела.md"];
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].text, "написать");
        assert!(!tasks[0].done);
        assert!(tasks[1].done);
    }

    #[test]
    fn reindexing_replaces_the_previous_tasks() {
        let mut connection = store("- [ ] старая");
        let transaction = connection.transaction().expect("транзакция");
        crate::search::index_document::index_text(&transaction, std::path::Path::new("C:/vault"), Path::new("C:/vault/Дела.md"), "- [ ] новая").expect("снова");
        transaction.commit().expect("фиксация");

        let found = read(&connection, &["C:/vault/Дела.md".to_owned()]).expect("чтение");
        assert_eq!(found["C:/vault/Дела.md"].len(), 1);
        assert_eq!(found["C:/vault/Дела.md"][0].text, "новая");
    }

    #[test]
    fn removing_a_document_takes_its_tasks_with_it() {
        let mut connection = store("- [ ] написать");
        let transaction = connection.transaction().expect("транзакция");
        crate::search::pages::remove(&transaction, "C:/vault/Дела.md").expect("удаление");
        transaction.commit().expect("фиксация");

        let found = read(&connection, &["C:/vault/Дела.md".to_owned()]).expect("чтение");
        assert!(found.is_empty());
    }
}
