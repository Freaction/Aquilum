use super::workspace::requested_vault;
use super::{open_after_write, optional_text, require_write, tool, version_name_property, workspace_property};
use crate::app_core::Core;
use crate::files::document::read_head;
use crate::files::error::FileCommandError;
use crate::files::gate;
use crate::history::Source;
use crate::search::analyzer::frontmatter::{same_key, set as frontmatter_set};
use crate::search::fields::{as_object, note_fields};
use serde_json::{json, Value};

mod find;

use find::find_notes;

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
    let updates = updates.iter().map(|(key, value)| (key.as_str(), Some(value.as_str()))).collect::<Vec<_>>();
    let version_name = optional_text(arguments, "versionName");
    let changed = gate::retrying(&note, |hash| {
        let head = read_head(&note)?;
        let change = frontmatter_set(&head, &updates, same_key).map_err(|message| FileCommandError::Task { message })?;
        match change {
            Some(change) => gate::edit(core, &note, &[change], Some(hash), Source::Agent, version_name.as_deref()).map(|_| true),
            None => Ok(false),
        }
    })
    .map_err(|error| error.to_string())?;
    if changed {
        open_after_write(core, &vault, &note, arguments);
    }
    let head = read_head(&note).map_err(|error| error.to_string())?;
    Ok(json!({
        "path": vault.relative(&note),
        "changed": changed,
        "frontmatter": map(&head),
    }))
}

fn field_updates(arguments: &Value) -> Result<Vec<(String, String)>, String> {
    let fields = arguments
        .get("fields")
        .and_then(Value::as_object)
        .ok_or("Не заполнен аргумент «fields»: нужен объект вида {«ключ»: «значение»}")?;
    if fields.is_empty() {
        return Err("Список полей пуст: нечего менять".to_owned());
    }
    fields
        .iter()
        .map(|(key, value)| {
            if key.trim().is_empty() || key.contains(':') || key.contains('\n') {
                return Err(format!("Недопустимое имя поля «{key}»: двоеточия и переводы строк в ключе не разрешены"));
            }
            let value = scalar(key, value)?;
            if value.contains('\n') {
                return Err(format!("Поле «{key}»: значение не может быть многострочным"));
            }
            Ok((key.trim().to_owned(), value))
        })
        .collect()
}

fn scalar(key: &str, value: &Value) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(_) | Value::Bool(_) => Ok(value.to_string()),
        Value::Null => Ok(String::new()),
        _ => Err(format!("Поле «{key}»: значением может быть строка, число или логическое значение")),
    }
}
