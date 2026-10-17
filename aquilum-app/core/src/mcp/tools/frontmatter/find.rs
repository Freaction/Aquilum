use super::super::workspace::indexed_vault;
use super::super::{limit, listing, offset, optional_names, optional_text};
use super::scalar;
use crate::app_core::Core;
use crate::search::analyzer::frontmatter::same_key;
use crate::search::fields::{head_fields, Field};
use serde_json::{json, Value};
use std::path::PathBuf;

const FIND_LIMIT: usize = 200;

struct Filter {
    equals: Vec<(String, String)>,
    has: Vec<String>,
    missing: Vec<String>,
}

pub(super) fn find_notes(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = indexed_vault(core, arguments)?;
    let folder = vault.folder_argument(arguments)?;
    let filter = Filter {
        equals: equals_from(arguments)?,
        has: optional_names(arguments, "has")?.unwrap_or_default(),
        missing: optional_names(arguments, "missing")?.unwrap_or_default(),
    };
    let wanted = optional_names(arguments, "fields")?.unwrap_or_default();
    let sort = optional_text(arguments, "sort");
    let indexed = core
        .search
        .note_fields_for_filter(&vault.root.to_string_lossy(), &filter.equals, &filter.has)
        .map_err(|error| error.to_string())?;
    let candidates: Vec<(PathBuf, Vec<Field>)> = match indexed {
        Some(rows) => rows.into_iter().map(|(path, fields)| (PathBuf::from(path), fields)).collect(),
        None => vault.markdown_files().filter(|path| path.starts_with(&folder)).map(|path| (path.clone(), head_fields(&path))).collect(),
    };
    let mut found = candidates
        .into_iter()
        .filter(|(path, fields)| path.starts_with(&folder) && matches(fields, &filter))
        .map(|(path, fields)| {
            let key = sort.as_deref().and_then(|name| value_of(&fields, name)).map(str::to_lowercase);
            (key.is_none(), key, vault.relative(&path), selected(&fields, &wanted))
        })
        .collect::<Vec<_>>();
    found.sort_by(|left, right| (left.0, &left.1, &left.2).cmp(&(right.0, &right.1, &right.2)));
    let (offset, total) = (offset(arguments), found.len());
    let size = limit(arguments, 100, FIND_LIMIT);
    let page = found.into_iter().skip(offset).take(size).map(|(_, _, path, fields)| json!({ "path": path, "fields": fields })).collect();
    Ok(listing("notes", total, offset, page))
}

fn matches(fields: &[Field], filter: &Filter) -> bool {
    filter.equals.iter().all(|(key, value)| value_of(fields, key).is_some_and(|found| same_key(found, value)))
        && filter.has.iter().all(|key| value_of(fields, key).is_some_and(|found| !found.trim().is_empty()))
        && filter.missing.iter().all(|key| value_of(fields, key).is_none_or(|found| found.trim().is_empty()))
}

fn value_of<'a>(fields: &'a [Field], key: &str) -> Option<&'a str> {
    fields.iter().find(|field| same_key(&field.key, key)).map(|field| field.text.as_str())
}

fn selected(fields: &[Field], wanted: &[String]) -> Value {
    Value::Object(wanted.iter().filter_map(|key| Some((key.clone(), Value::String(value_of(fields, key)?.to_owned())))).collect())
}

fn equals_from(arguments: &Value) -> Result<Vec<(String, String)>, String> {
    let Some(fields) = arguments.get("match") else {
        return Ok(Vec::new());
    };
    let fields = fields.as_object().ok_or("Аргумент «match» должен быть объектом вида {«ключ»: «значение»}")?;
    fields.iter().map(|(key, value)| Ok((key.trim().to_owned(), scalar(key, value)?))).collect()
}

#[cfg(test)]
#[path = "find_tests.rs"]
mod tests;
