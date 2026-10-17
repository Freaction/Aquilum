use crate::search::error::SearchError;
use crate::search::fields::{read_fields, Field};
use crate::search::tasks::{read_tasks, Task};
use rusqlite::Connection;
use crate::search::paths::{relative_slash_path, strip_markdown_extension};
use std::path::Path;

pub struct Row {
    pub relative: String,
    pub source: String,
    pub name: String,
    pub folder: String,
    pub created: i64,
    pub modified: i64,
    pub size: i64,
    pub fields: Vec<Field>,
    pub tasks: Vec<Task>,
}

impl Row {
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|field| crate::search::analyzer::frontmatter::same_key(&field.key, name))
    }
}

use crate::search::metadata::for_paths;

pub fn load(
    connection: &Connection,
    root: &Path,
    paths: &[String],
) -> Result<Vec<Row>, SearchError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let mut fields = read_fields(connection, paths)?;
    let mut rows = Vec::with_capacity(paths.len());

    let sql = |list: &str| format!("SELECT path, created_ns, modified_ns, size FROM documents WHERE path IN ({list})");
    let row = |row: &rusqlite::Row<'_>| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?));
    for_paths(connection, paths, sql, row, |(path, created, modified, size)| {
        let relative = relative_slash_path(root, Path::new(&path));
        rows.push(Row {
            source: path.clone(),
            name: note_name(&relative),
            folder: folder_of(&relative),
            fields: fields.remove(&path).unwrap_or_default(),
            tasks: Vec::new(),
            relative,
            created,
            modified,
            size,
        });
    })?;
    Ok(rows)
}

pub fn attach_tasks(connection: &Connection, rows: &mut [Row], paths: &[String]) -> Result<(), SearchError> {
    if paths.is_empty() || rows.is_empty() {
        return Ok(());
    }
    let mut found = read_tasks(connection, paths)?;
    for row in rows.iter_mut() {
        if let Some(tasks) = found.remove(&row.source) {
            row.tasks = tasks;
        }
    }
    Ok(())
}

fn note_name(relative: &str) -> String {
    let name = relative.rsplit('/').next().unwrap_or(relative);
    strip_markdown_extension(name).to_owned()
}

fn folder_of(relative: &str) -> String {
    match relative.rfind('/') {
        Some(position) => relative[..position].to_owned(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{folder_of, note_name};

    #[test]
    fn a_note_name_drops_the_folder_and_the_extension() {
        assert_eq!(note_name("Книги/Пороки.md"), "Пороки");
        assert_eq!(note_name("Пороки.md"), "Пороки");
    }

    #[test]
    fn a_note_in_the_root_has_an_empty_folder() {
        assert_eq!(folder_of("Книги/Пороки.md"), "Книги");
        assert_eq!(folder_of("Пороки.md"), "");
    }
}
