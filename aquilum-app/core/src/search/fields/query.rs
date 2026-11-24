use super::value::{lower, Field, FieldKind};
use crate::search::error::SearchError;
use crate::search::metadata::{all_paths, for_paths};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

const FIELDS: &str = "note_fields field JOIN note_pages page ON page.id = field.page";

pub fn read(connection: &Connection, paths: &[String]) -> Result<HashMap<String, Vec<Field>>, SearchError> {
    let mut found: HashMap<String, Vec<Field>> = HashMap::new();
    let sql = |list: &str| {
        format!(
            "SELECT page.path, field.key, field.kind, field.text, field.items FROM {FIELDS}
             WHERE page.path IN ({list}) ORDER BY page.path, page.ordinal, field.ordinal"
        )
    };
    let row = |row: &rusqlite::Row<'_>| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, String>(3)?, row.get::<_, Option<String>>(4)?));
    for_paths(connection, paths, sql, row, |(path, key, kind, text, items)| {
        let fields = found.entry(path).or_default();
        if !fields.iter().any(|field| field.key.eq_ignore_ascii_case(&key)) {
            fields.push(Field::stored(key, FieldKind::from_stored(kind), text, items));
        }
    })?;
    Ok(found)
}

pub fn candidates(
    connection: &Connection,
    equals: &[(String, String)],
    has: &[String],
) -> Result<Vec<String>, SearchError> {
    let mut narrowed: Option<HashSet<String>> = None;
    for (key, value) in equals {
        let found = lookup(
            connection,
            &format!("SELECT DISTINCT page.path FROM {FIELDS} WHERE field.key_lower=?1 AND field.text_lower=?2"),
            &[lower(key), lower(value)],
        )?;
        narrowed = Some(intersect(narrowed, found));
    }
    for key in has {
        let found = lookup(
            connection,
            &format!("SELECT DISTINCT page.path FROM {FIELDS} WHERE field.key_lower=?1 AND field.text_lower<>''"),
            &[lower(key)],
        )?;
        narrowed = Some(intersect(narrowed, found));
    }
    match narrowed {
        Some(paths) => {
            let mut paths = paths.into_iter().collect::<Vec<_>>();
            paths.sort();
            Ok(paths)
        }
        None => all_paths(connection),
    }
}

pub fn paths_with_tag(connection: &Connection, tag: &str) -> Result<HashSet<String>, SearchError> {
    let wanted = lower(tag.trim_start_matches('#'));
    let nested = format!("{wanted}/");
    let mut statement =
        connection.prepare(&format!("SELECT page.path, field.text_lower, field.items FROM {FIELDS} WHERE field.key_lower='tags'"))?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut found = HashSet::new();
    for row in rows {
        let (path, text, items) = row?;
        let carries = |value: &str| {
            let value = lower(value);
            value == wanted || value.starts_with(&nested)
        };
        let matched = match items {
            Some(raw) => serde_json::from_str::<Vec<String>>(&raw)
                .map(|items| items.iter().any(|item| carries(item)))
                .unwrap_or(false),
            None => carries(&text),
        };
        if matched {
            found.insert(path);
        }
    }
    Ok(found)
}

fn lookup(
    connection: &Connection,
    sql: &str,
    arguments: &[String],
) -> Result<HashSet<String>, SearchError> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(arguments), |row| {
            row.get::<_, String>(0)
        })?
        .collect::<Result<HashSet<_>, _>>()?;
    Ok(rows)
}

fn intersect(narrowed: Option<HashSet<String>>, found: HashSet<String>) -> HashSet<String> {
    match narrowed {
        Some(current) => current.intersection(&found).cloned().collect(),
        None => found,
    }
}
