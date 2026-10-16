use super::workspace::{indexed_vault, requested_vault};
use super::{offset, 
    limit, open_after_write, optional_names, optional_text, require_write, tool,
    version_name_property, workspace_property, write_retrying,
};
use crate::history::Source;
use crate::search::analyzer::frontmatter_set;
use crate::search::fields::{as_object, head_fields, note_fields, Field};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use crate::app_core::Core;

const FIND_LIMIT: usize = 200;

pub fn definitions() -> Vec<Value> {
    vec![
        tool(
            "set_frontmatter",
            "Изменить поля frontmatter заметки: перечисленные ключи создаются или переписываются, остальные поля, их порядок и текст заметки остаются нетронутыми. Пустая строка очищает значение, но оставляет ключ. Если у заметки нет frontmatter, блок создаётся в начале. В ответе поля-списки приходят массивом.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "fields": { "type": "object", "description": "Пары «ключ: значение», например {\"status\": \"read\", \"rating\": 5}" },
                "open": { "type": "boolean", "description": "Открыть заметку вкладкой в приложении, по умолчанию да; в другой базе вкладка не открывается" },
                "versionName": version_name_property(),
                "workspace": workspace_property(),
            }),
            &["note", "fields"],
        ),
        tool(
            "find_notes",
            "Выборка заметок по полям frontmatter: точное совпадение (match), заполненность (has) и незаполненность (missing). Это не полнотекстовый поиск: по содержимому заметок ищет search_notes, а здесь ищутся метаданные — «все заметки с type: book», «книги без автора». Значения полей возвращаются только для ключей, перечисленных в fields. Поля-списки (`tags: [a, b]` или список с дефисами) приходят массивом, пустой список считается незаполненным полем, а match сравнивает значение целиком, а не отдельные элементы списка.",
            json!({
                "match": { "type": "object", "description": "Поля, совпадающие точно (регистр не учитывается), например {\"type\": \"book\"}" },
                "has": { "type": "array", "items": { "type": "string" }, "description": "Эти поля должны быть заполнены" },
                "missing": { "type": "array", "items": { "type": "string" }, "description": "Эти поля должны отсутствовать или быть пустыми" },
                "fields": { "type": "array", "items": { "type": "string" }, "description": "Какие поля вернуть в ответе; без него возвращаются только пути" },
                "folder": { "type": "string", "description": "Искать только в этой папке, пусто — вся база" },
                "sort": { "type": "string", "description": "Поле для сортировки, по умолчанию сортировка по пути" },
                "offset": { "type": "integer", "description": "Сколько заметок пропустить, чтобы получить следующую страницу" },
                "limit": { "type": "integer", "description": "Сколько заметок вернуть, по умолчанию 100, максимум 200" },
                "workspace": workspace_property(),
            }),
            &[],
        ),
    ]
}

pub fn call(core: &Core, name: &str, arguments: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "set_frontmatter" => set_frontmatter(core, arguments),
        "find_notes" => find_notes(core, arguments),
        _ => return None,
    })
}

pub fn map(text: &str) -> Value {
    Value::Object(as_object(&note_fields(text)))
}

fn set_frontmatter(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let updates = field_updates(arguments)?;
    let version_name = optional_text(arguments, "versionName");
    let (changed, next) = write_retrying(core, &note, Source::Agent, version_name.as_deref(), |current| {
        frontmatter_set(current, &updates)
    })?;
    if changed {
        open_after_write(core, &vault, &note, arguments);
    }
    Ok(json!({
        "path": vault.relative(&note),
        "changed": changed,
        "frontmatter": map(&next),
    }))
}

fn field_updates(arguments: &Value) -> Result<Vec<(String, String)>, String> {
    let fields = arguments
        .get("fields")
        .and_then(Value::as_object)
        .ok_or_else(|| "Не заполнен аргумент «fields»: нужен объект вида {«ключ»: «значение»}".to_owned())?;
    if fields.is_empty() {
        return Err("Список полей пуст: нечего менять".to_owned());
    }
    fields
        .iter()
        .map(|(key, value)| {
            if key.trim().is_empty() || key.contains(':') || key.contains('\n') {
                return Err(format!(
                    "Недопустимое имя поля «{key}»: двоеточия и переводы строк в ключе не разрешены"
                ));
            }
            let value = match value {
                Value::String(value) => value.clone(),
                Value::Number(value) => value.to_string(),
                Value::Bool(value) => value.to_string(),
                Value::Null => String::new(),
                _ => {
                    return Err(format!(
                        "Поле «{key}»: значением может быть строка, число или логическое значение"
                    ))
                }
            };
            if value.contains('\n') {
                return Err(format!("Поле «{key}»: значение не может быть многострочным"));
            }
            Ok((key.trim().to_owned(), value))
        })
        .collect()
}

struct Filter {
    equals: Vec<(String, String)>,
    has: Vec<String>,
    missing: Vec<String>,
}

fn find_notes(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = indexed_vault(core, arguments)?;
    let folder = vault.folder(&optional_text(arguments, "folder").unwrap_or_default())?;
    if !folder.is_dir() {
        return Err(format!("Папка не найдена: {}", vault.relative(&folder)));
    }
    let filter = Filter {
        equals: equals_from(arguments)?,
        has: optional_names(arguments, "has")?.unwrap_or_default(),
        missing: optional_names(arguments, "missing")?.unwrap_or_default(),
    };
    let wanted = optional_names(arguments, "fields")?.unwrap_or_default();
    let sort = optional_text(arguments, "sort");
    let size = limit(arguments, 100, FIND_LIMIT);
    let offset = offset(arguments);

    let indexed = core
        .search
        .note_fields_for_filter(&vault.root.to_string_lossy(), &filter.equals, &filter.has)
        .map_err(|error| error.to_string())?;
    let candidates: Vec<(PathBuf, Vec<(String, String)>)> = match indexed {
        Some(rows) => rows
            .into_iter()
            .map(|(path, fields)| (PathBuf::from(path), pairs_of(&fields)))
            .filter(|(path, _)| path.starts_with(&folder))
            .collect(),
        None => vault
            .markdown_files()
            .filter(|path| path.starts_with(&folder))
            .map(|path| {
                let pairs = pairs_of(&head_fields(&path));
                (path, pairs)
            })
            .collect(),
    };

    let mut found = Vec::new();
    for (path, pairs) in candidates {
        if !matches(&pairs, &filter) {
            continue;
        }
        let key = sort
            .as_deref()
            .and_then(|name| value_of(&pairs, name))
            .map(|value| value.to_lowercase());
        found.push((vault.relative(&path), selected(&pairs, &wanted), key));
    }

    match sort.as_deref() {
        Some(_) => found.sort_by(|left, right| {
            left.2
                .is_none()
                .cmp(&right.2.is_none())
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.0.cmp(&right.0))
        }),
        None => found.sort_by(|left, right| left.0.cmp(&right.0)),
    }

    let total = found.len();
    let page = found
        .into_iter()
        .skip(offset)
        .take(size)
        .map(|(path, fields, _)| json!({ "path": path, "fields": fields }))
        .collect::<Vec<_>>();
    let end = offset.saturating_add(page.len());

    Ok(json!({
        "total": total,
        "notes": page,
        "nextOffset": (end < total).then_some(end),
    }))
}

fn pairs_of(fields: &[Field]) -> Vec<(String, String)> {
    fields
        .iter()
        .map(|field| (field.key.clone(), field.text.clone()))
        .collect()
}

fn matches(pairs: &[(String, String)], filter: &Filter) -> bool {
    filter
        .equals
        .iter()
        .all(|(key, value)| value_of(pairs, key).is_some_and(|found| same(found, value)))
        && filter
            .has
            .iter()
            .all(|key| value_of(pairs, key).is_some_and(|found| !found.trim().is_empty()))
        && filter
            .missing
            .iter()
            .all(|key| value_of(pairs, key).is_none_or(|found| found.trim().is_empty()))
}

fn value_of<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(name, _)| same(name, key))
        .map(|(_, value)| value.as_str())
}

fn same(left: &str, right: &str) -> bool {
    left.trim().to_lowercase() == right.trim().to_lowercase()
}

fn selected(pairs: &[(String, String)], wanted: &[String]) -> Value {
    let mut fields = Map::new();
    for key in wanted {
        if let Some(value) = value_of(pairs, key) {
            fields.insert(key.clone(), Value::String(value.to_owned()));
        }
    }
    Value::Object(fields)
}

fn equals_from(arguments: &Value) -> Result<Vec<(String, String)>, String> {
    let Some(fields) = arguments.get("match") else {
        return Ok(Vec::new());
    };
    let fields = fields
        .as_object()
        .ok_or_else(|| "Аргумент «match» должен быть объектом вида {«ключ»: «значение»}".to_owned())?;
    fields
        .iter()
        .map(|(key, value)| {
            let value = match value {
                Value::String(value) => value.clone(),
                Value::Number(value) => value.to_string(),
                Value::Bool(value) => value.to_string(),
                _ => {
                    return Err(format!(
                        "Поле «{key}»: сравнивать можно со строкой, числом или логическим значением"
                    ))
                }
            };
            Ok((key.trim().to_owned(), value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{matches, selected, value_of, Filter};

    fn book() -> Vec<(String, String)> {
        vec![
            ("type".to_owned(), "book".to_owned()),
            ("author".to_owned(), "Иван Ефремов".to_owned()),
            ("status".to_owned(), "read".to_owned()),
            ("Путь к файлу".to_owned(), String::new()),
        ]
    }

    fn filter(equals: &[(&str, &str)], has: &[&str], missing: &[&str]) -> Filter {
        Filter {
            equals: equals
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
            has: has.iter().map(|key| (*key).to_owned()).collect(),
            missing: missing.iter().map(|key| (*key).to_owned()).collect(),
        }
    }

    #[test]
    fn equality_ignores_case_of_both_key_and_value() {
        assert!(matches(&book(), &filter(&[("TYPE", "Book")], &[], &[])));
        assert!(!matches(&book(), &filter(&[("type", "article")], &[], &[])));
    }

    #[test]
    fn a_missing_key_never_matches_equality() {
        assert!(!matches(&book(), &filter(&[("rating", "5")], &[], &[])));
    }

    #[test]
    fn empty_value_counts_as_missing_not_as_present() {
        assert!(matches(&book(), &filter(&[], &[], &["Путь к файлу"])));
        assert!(!matches(&book(), &filter(&[], &["Путь к файлу"], &[])));
        assert!(matches(&book(), &filter(&[], &["author"], &["rating"])));
    }

    #[test]
    fn all_conditions_must_hold_together() {
        assert!(matches(
            &book(),
            &filter(&[("type", "book")], &["author"], &["Путь к файлу"])
        ));
        assert!(!matches(
            &book(),
            &filter(&[("type", "book")], &["rating"], &[])
        ));
    }

    #[test]
    fn an_empty_filter_matches_everything() {
        assert!(matches(&book(), &filter(&[], &[], &[])));
        assert!(matches(&[], &filter(&[], &[], &[])));
    }

    #[test]
    fn only_requested_fields_come_back() {
        let fields = selected(&book(), &["author".to_owned(), "rating".to_owned()]);
        let object = fields.as_object().unwrap();
        assert_eq!(object.len(), 1, "несуществующее поле не превращается в null");
        assert_eq!(object["author"], "Иван Ефремов");
        assert!(selected(&book(), &[]).as_object().unwrap().is_empty());
    }

    #[test]
    fn lookup_finds_the_cyrillic_key_written_in_another_case() {
        assert_eq!(value_of(&book(), "ПУТЬ К ФАЙЛУ"), Some(""));
        assert_eq!(value_of(&book(), "AUTHOR"), Some("Иван Ефремов"));
    }
}
