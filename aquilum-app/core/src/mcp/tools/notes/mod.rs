use super::super::vault::Vault;
use super::workspace::{indexed_vault, requested_vault, workspace_vault};
use super::{listing, offset, text_window,
    content, content_or_empty, flag, limit, open_after_write, optional_content, optional_names, optional_text,
    require_write, shorten, text, tool, version_name_property, workspace_property,
};
use crate::files::document::read_head;
use crate::search::pages::{read_chars, Pages};
use crate::files::gate;
use crate::history::Source;
use crate::search::headings::{self, Heading};
use crate::text::matcher::{Matcher, IGNORE_CASE};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use crate::app_core::Core;

const READ_LIMIT: usize = 8_000;
const MANY_READ_LIMIT: usize = 2_000;
const LIST_LIMIT: usize = 200;
const BATCH_LIMIT: usize = 100;

pub fn definitions() -> Vec<Value> {
    vec![
        tool(
            "read_note",
            "Прочитать заметку целиком или один её раздел (аргумент heading). Несколько заметок за раз читаются через notes — ответ придёт списком, и по умолчанию каждая обрезается до 2000 символов. Поля frontmatter возвращаются отдельным объектом frontmatter, значения-списки в нём приходят массивом, менять их надо через set_frontmatter. Длинный текст отдаётся частями: смотрите поля truncated и nextOffset.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "notes": { "type": "array", "items": { "type": "string" }, "description": "Несколько заметок за один вызов, не больше 100; вместо note" },
                "heading": { "type": "string", "description": "Читать только этот раздел: заголовок и текст до следующего заголовка того же или более высокого уровня" },
                "offset": { "type": "integer", "description": "С какого символа читать" },
                "limit": { "type": "integer", "description": "Сколько символов вернуть, по умолчанию 8000" },
                "workspace": workspace_property(),
            }),
            &["note"],
        ),
        tool(
            "create_note",
            "Создать заметку. Папки создаются автоматически. Если файл по этому пути уже есть, вызов вернёт ошибку и ничего не изменит, поэтому проверять существование заранее не нужно. Для импорта передайте notes — список заметок создаётся за один вызов, ошибка на одной не отменяет остальные, а вкладки при этом не открываются.",
            json!({
                "path": { "type": "string", "description": "Путь от корня базы, например «Проекты/Идея.md»" },
                "content": { "type": "string", "description": "Начальное содержимое" },
                "notes": { "type": "array", "items": { "type": "object" }, "description": "Пачка заметок: список объектов {path, content}, не больше 100; вместо path и content" },
                "workspace": workspace_property(),
                "open": { "type": "boolean", "description": "Открыть заметку вкладкой в приложении: по умолчанию да для одной заметки и нет для пачки" },
                "versionName": version_name_property(),
            }),
            &[],
        ),
        tool(
            "update_note",
            "Изменить текст заметки. Правьте разделом (underHeading, replaceSection) или точной заменой (findReplace); mode=replace перезаписывает всю заметку целиком вместе с frontmatter, поэтому метаданные меняйте через set_frontmatter, а не режимом replace.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "mode": {
                    "type": "string",
                    "enum": ["append", "prepend", "replace", "findReplace", "underHeading", "replaceSection"],
                    "description": "Способ правки, по умолчанию append. underHeading вставляет текст первым абзацем раздела, replaceSection заменяет текст раздела, сохраняя заголовок",
                },
                "content": { "type": "string", "description": "Новый текст или замена; в режимах findReplace и replaceSection может быть пустым — тогда найденный текст или текст раздела удаляется" },
                "find": { "type": "string", "description": "Что заменить в режиме findReplace" },
                "heading": { "type": "string", "description": "Заголовок раздела для режимов underHeading и replaceSection" },
                "all": { "type": "boolean", "description": "В режиме findReplace заменить все вхождения; по умолчанию нет, и на нескольких вхождениях вернётся ошибка с их числом" },
                "open": { "type": "boolean", "description": "Открыть заметку вкладкой в приложении, по умолчанию да; в другой базе вкладка не открывается" },
                "versionName": version_name_property(),
                "workspace": workspace_property(),
            }),
            &["note", "content"],
        ),
        tool(
            "rename_note",
            "Переименовать заметку с обновлением всех входящих ссылок [[...]].",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "title": { "type": "string", "description": "Новое название без расширения" },
                "workspace": workspace_property(),
            }),
            &["note", "title"],
        ),
        tool(
            "move_note",
            "Переместить заметку или пачку заметок (notes) в другую папку или в другую базу знаний. При переносе между базами вставленные картинки и файлы едут вместе с заметками: свои переносятся, общие с другими заметками копируются. brokenLinks — заметки исходной базы, чьи ссылки на перенесённые оборвутся (ссылки внутри пачки не считаются).",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "notes": { "type": "array", "items": { "type": "string" }, "description": "Несколько заметок за один вызов; вместо note" },
                "folder": { "type": "string", "description": "Папка назначения, пусто — корень базы" },
                "workspace": workspace_property(),
                "toWorkspace": { "type": "string", "description": "База назначения для переноса между базами: путь или имя из list_workspaces" },
            }),
            &[],
        ),
        tool(
            "delete_note",
            "Удалить заметку: файл переносится в корзину базы на то же место внутри .trash и хранится там по сроку из настроек. Вернуть можно через restore_note, посмотреть — через list_trash.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "workspace": workspace_property(),
            }),
            &["note"],
        ),
        tool(
            "list_notes",
            "Все заметки базы знаний постранично: total — сколько всего, nextOffset — с чего продолжить. Дешёвый способ увидеть базу целиком. С sort=links заметки идут объектами {path, links} от самых цитируемых: это карта хабов незнакомой базы.",
            json!({
                "folder": { "type": "string", "description": "Папка, пусто — вся база" },
                "query": { "type": "string", "description": "Подстрока в названии" },
                "offset": { "type": "integer", "description": "Сколько путей пропустить, чтобы получить следующую страницу" },
                "limit": { "type": "integer", "description": "Сколько путей вернуть, по умолчанию 100, максимум 200" },
                "sort": { "type": "string", "enum": ["path", "links"], "description": "path — по пути (по умолчанию), links — по числу заметок, которые ссылаются на эту" },
                "workspace": workspace_property(),
            }),
            &[],
        ),
        tool(
            "note_outline",
            "Заголовки заметки с уровнем и размером раздела. Нужный раздел дальше читается через read_note с аргументом heading.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "workspace": workspace_property(),
            }),
            &["note"],
        ),
        tool(
            "list_folders",
            "Структура папок базы знаний.",
            json!({ "workspace": workspace_property() }),
            &[],
        ),
    ]
}

pub fn call(
    core: &Core,
    name: &str,
    arguments: &Value,
) -> Option<Result<Value, String>> {
    Some(match name {
        "read_note" => read_note(core, arguments),
        "create_note" => create_note(core, arguments),
        "update_note" => update_note(core, arguments),
        "rename_note" => rename_note(core, arguments),
        "move_note" => move_note(core, arguments),
        "delete_note" => delete_note(core, arguments),
        "list_notes" => list_notes(core, arguments),
        "note_outline" => note_outline(core, arguments),
        "list_folders" => list_folders(core, arguments),
        _ => return None,
    })
}


mod create;
mod edit;
mod list;
mod moves;
mod read;
#[cfg(test)]
mod tests;
mod transfer;
#[cfg(test)]
mod transfer_tests;

use create::*;
use edit::*;
use list::*;
use moves::*;
use read::*;
