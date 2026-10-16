use super::super::vault::Vault;
use super::workspace::{indexed_vault, requested_vault, workspace_vault};
use super::{offset, 
    content, content_or_empty, flag, limit, open_after_write, optional_content, optional_names, optional_text,
    require_write, shorten, text, tool, version_name_property, workspace_property, write_retrying,
};
use crate::files::document::{ensure_directory_impl, read_text};
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
            "Переместить заметку в другую папку или в другую базу знаний.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "folder": { "type": "string", "description": "Папка назначения, пусто — корень базы" },
                "workspace": workspace_property(),
                "toWorkspace": { "type": "string", "description": "База назначения для переноса между базами: путь или имя из list_workspaces" },
            }),
            &["note"],
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

fn read_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = requested_vault(core, arguments)?;
    let Some(many) = optional_names(arguments, "notes")? else {
        let size = limit(arguments, READ_LIMIT, 40_000);
        return read_one(&vault, &text(arguments, "note")?, arguments, size);
    };
    if many.len() > BATCH_LIMIT {
        return Err(format!(
            "За один вызов читается не больше {BATCH_LIMIT} заметок, запрошено {}",
            many.len()
        ));
    }
    let size = limit(arguments, MANY_READ_LIMIT, 40_000);
    let notes = many
        .iter()
        .map(|name| match read_one(&vault, name, arguments, size) {
            Ok(note) => note,
            Err(error) => json!({ "note": name, "error": error }),
        })
        .collect::<Vec<_>>();
    Ok(json!({ "notes": notes }))
}

fn read_one(vault: &Vault, name: &str, arguments: &Value, size: usize) -> Result<Value, String> {
    let note = vault.note(name)?;
    let content = read_text(&note).map_err(|error| error.to_string())?;
    let meta = super::frontmatter::map(&content);
    let (content, section) = match optional_text(arguments, "heading").as_deref() {
        Some(wanted) => {
            let outline = headings::headings(&content);
            let heading = single_heading(&outline, wanted)?;
            (
                content[heading.start..heading.content.end].to_owned(),
                Some(heading.title.clone()),
            )
        }
        None => (content, None),
    };

    let offset = offset(arguments);
    let (body, total) = window(&content, offset, size)?;
    let shown = size.min(total.saturating_sub(offset));
    let end = offset.saturating_add(shown);

    Ok(json!({
        "path": vault.relative(&note),
        "frontmatter": meta,
        "heading": section,
        "content": body,
        "totalChars": total,
        "truncated": end < total,
        "nextOffset": (end < total).then_some(end),
    }))
}

pub fn window(text: &str, offset: usize, size: usize) -> Result<(&str, usize), String> {
    let stop = offset.saturating_add(size);
    let mut start = (offset == 0).then_some(0);
    let mut end = None;
    let mut total = 0;
    for (index, (byte, _)) in text.char_indices().enumerate() {
        total = index + 1;
        if index == offset {
            start = Some(byte);
        }
        if index == stop {
            end = Some(byte);
        }
    }
    let start = start
        .or_else(|| (offset == total).then_some(text.len()))
        .ok_or_else(|| format!("Смещение {offset} за концом текста: всего символов {total}"))?;
    Ok((&text[start..end.unwrap_or(text.len())], total))
}

fn note_outline(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let content = read_text(&note).map_err(|error| error.to_string())?;
    let items = headings::headings(&content)
        .iter()
        .map(|heading| {
            json!({
                "level": heading.level,
                "heading": heading.title,
                "chars": content[heading.content.clone()].chars().count(),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "note": vault.relative(&note),
        "totalChars": content.chars().count(),
        "outline": items,
    }))
}

fn single_heading<'a>(outline: &'a [Heading], wanted: &str) -> Result<&'a Heading, String> {
    let found = headings::matching(outline, wanted);
    match found.len() {
        0 => Err(format!("Заголовок не найден: {wanted}")),
        1 => Ok(found[0]),
        count => Err(format!(
            "Заголовок «{wanted}» встречается в заметке больше одного раза (всего: {count}) — правьте через findReplace или переименуйте один из разделов"
        )),
    }
}

fn create_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = requested_vault(core, arguments)?;
    let requested = requested_notes(arguments)?;
    let batch = requested.len() > 1;
    let version_name = optional_text(arguments, "versionName");

    let mut created = Vec::new();
    let mut failed = Vec::new();
    let mut paths = Vec::new();
    for (path, initial) in &requested {
        match create_one(core, &vault, path, initial, version_name.as_deref()) {
            Ok(path) => {
                created.push(vault.relative(&path));
                paths.push(path);
            }
            Err(error) => failed.push(json!({ "path": path, "error": error })),
        }
    }

    if vault.visible && flag(arguments, "open", !batch) {
        if let Some(last) = paths.last() {
            super::super::bridge::open_note_after_write(core, last);
        }
    }

    if !batch {
        let path = created.into_iter().next().ok_or_else(|| {
            failed
                .first()
                .and_then(|entry| entry["error"].as_str())
                .unwrap_or("Заметка не создана")
                .to_owned()
        })?;
        return Ok(json!({
            "path": path,
            "workspace": vault.root.to_string_lossy(),
            "created": true,
        }));
    }
    Ok(json!({
        "workspace": vault.root.to_string_lossy(),
        "created": created,
        "failed": failed,
    }))
}

fn create_one(
    core: &Core,
    vault: &Vault,
    requested: &str,
    initial: &str,
    version_name: Option<&str>,
) -> Result<PathBuf, String> {
    let path = vault.note_path(requested)?;
    if path.exists() {
        return Err(format!("Заметка уже существует: {}", vault.relative(&path)));
    }
    if let Some(parent) = path.parent() {
        ensure_directory_impl(parent).map_err(|error| error.to_string())?;
    }
    gate::create(core, &path, initial, Source::Agent, version_name).map_err(|error| error.to_string())?;
    Ok(path)
}

fn requested_notes(arguments: &Value) -> Result<Vec<(String, String)>, String> {
    let Some(list) = arguments.get("notes") else {
        return Ok(vec![(
            text(arguments, "path")?,
            optional_content(arguments, "content").unwrap_or_default(),
        )]);
    };
    let list = list
        .as_array()
        .ok_or_else(|| "Аргумент «notes» — список объектов вида {path, content}".to_owned())?;
    if list.is_empty() {
        return Err("Список «notes» пуст: нечего создавать".to_owned());
    }
    if list.len() > BATCH_LIMIT {
        return Err(format!(
            "За один вызов создаётся не больше {BATCH_LIMIT} заметок, запрошено {}",
            list.len()
        ));
    }
    list.iter()
        .map(|item| {
            let path = item
                .get("path")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "У каждой заметки в «notes» должен быть непустой path".to_owned())?;
            let initial = item
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            Ok((path.to_owned(), initial))
        })
        .collect()
}

fn update_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let edit = edit_from(arguments)?;
    let version_name = optional_text(arguments, "versionName");
    let (changed, next) = write_retrying(core, &note, Source::Agent, version_name.as_deref(), |current| {
        compose(current, &edit)
    })?;
    if !changed {
        return Ok(json!({ "path": vault.relative(&note), "changed": false }));
    }
    open_after_write(core, &vault, &note, arguments);
    Ok(json!({
        "path": vault.relative(&note),
        "changed": true,
        "chars": next.chars().count(),
    }))
}

struct Edit {
    mode: String,
    content: String,
    find: Option<String>,
    heading: Option<String>,
    all: bool,
}

fn edit_from(arguments: &Value) -> Result<Edit, String> {
    let mode = optional_text(arguments, "mode").unwrap_or_else(|| "append".to_owned());
    let content = match mode.as_str() {
        "findReplace" | "replaceSection" => content_or_empty(arguments, "content")?,
        _ => content(arguments, "content")?,
    };
    Ok(Edit {
        mode,
        content,
        find: optional_content(arguments, "find"),
        heading: optional_text(arguments, "heading"),
        all: flag(arguments, "all", false),
    })
}

impl Edit {
    fn heading(&self) -> Result<&str, String> {
        self.heading
            .as_deref()
            .ok_or_else(|| format!("Для режима {} нужен аргумент «heading»", self.mode))
    }
}

fn compose(current: &str, edit: &Edit) -> Result<String, String> {
    let content = edit.content.as_str();
    match edit.mode.as_str() {
        "replace" => Ok(content.to_owned()),
        "append" => Ok(join_blocks(current, content)),
        "prepend" => Ok(join_blocks(content, current)),
        "findReplace" => {
            let needle = edit
                .find
                .as_deref()
                .ok_or("Для режима findReplace нужен аргумент «find»")?;
            match current.matches(needle).count() {
                0 => Err(format!("Текст не найден в заметке: {}", shorten(needle, 80))),
                count if count > 1 && !edit.all => Err(format!(
                    "Текст встречается в заметке больше одного раза (всего: {count}) — уточните «find» или передайте all: true"
                )),
                _ => Ok(current.replace(needle, content)),
            }
        }
        "underHeading" => write_section(current, edit.heading()?, content, false),
        "replaceSection" => write_section(current, edit.heading()?, content, true),
        other => Err(format!("Неизвестный режим правки: {other}")),
    }
}

fn join_blocks(before: &str, after: &str) -> String {
    if before.trim().is_empty() {
        return after.to_owned();
    }
    format!("{}\n\n{}", before.trim_end_matches('\n'), after.trim_start_matches('\n'))
}

fn write_section(
    current: &str,
    heading: &str,
    content: &str,
    replace: bool,
) -> Result<String, String> {
    let outline = headings::headings(current);
    let section = single_heading(&outline, heading)?.content.clone();
    let kept = if replace { section.end } else { section.start };
    let tail = current[kept..].trim_start_matches('\n');

    let mut next = String::with_capacity(current.len() + content.len() + 3);
    next.push_str(&current[..section.start]);
    let body = content.trim_matches('\n');
    if !body.is_empty() {
        next.push('\n');
        next.push_str(body);
        next.push('\n');
    }
    if !tail.is_empty() {
        next.push('\n');
        next.push_str(tail);
    }
    Ok(next)
}

fn rename_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = indexed_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let title = text(arguments, "title")?;
    if title.contains(['/', '\\']) {
        return Err("Название не должно содержать разделители пути — используйте move_note".to_owned());
    }
    let target = note.with_file_name(format!("{}.md", title.trim_end_matches(".md")));
    if target == note {
        return Ok(json!({ "path": vault.relative(&note), "renamed": false }));
    }
    if target.exists() {
        return Err(format!("Заметка уже существует: {}", vault.relative(&target)));
    }

    let result = gate::rename(core, &note, &target).map_err(|error| error.to_string())?;
    Ok(json!({
        "path": vault.relative(&target),
        "renamed": true,
        "updatedLinks": result.updated_paths.len(),
    }))
}

fn move_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = indexed_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let file_name = note
        .file_name()
        .ok_or("Не удалось определить имя файла")?
        .to_owned();
    let folder = optional_text(arguments, "folder").unwrap_or_default();

    let destination = match optional_text(arguments, "toWorkspace") {
        Some(input) => workspace_vault(core, &input)?,
        None => vault.clone(),
    };
    let target = destination.folder(&folder)?.join(&file_name);
    if destination.root == vault.root {
        return move_inside_vault(core, &vault, &note, target);
    }
    if target.exists() {
        return Err(format!("В целевой базе уже есть {}", destination.relative(&target)));
    }
    if let Some(parent) = target.parent() {
        ensure_directory_impl(parent).map_err(|error| error.to_string())?;
    }

    let broken = core
        .search
        .backlinks(&vault.root.to_string_lossy(), &note.to_string_lossy())
        .map(|links| {
            links
                .iter()
                .map(|link| vault.relative(Path::new(&link.path)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    gate::move_across(core, &note, &target).map_err(|error| error.to_string())?;
    Ok(json!({
        "path": destination.relative(&target),
        "workspace": destination.root.to_string_lossy(),
        "brokenLinks": broken,
    }))
}

fn move_inside_vault(
    core: &Core,
    vault: &Vault,
    note: &Path,
    target: PathBuf,
) -> Result<Value, String> {
    if target == note {
        return Ok(json!({ "path": vault.relative(note), "moved": false }));
    }
    if target.exists() {
        return Err(format!("Заметка уже существует: {}", vault.relative(&target)));
    }
    if let Some(parent) = target.parent() {
        ensure_directory_impl(parent).map_err(|error| error.to_string())?;
    }
    let result = gate::rename(core, note, &target).map_err(|error| error.to_string())?;
    Ok(json!({
        "path": vault.relative(&target),
        "moved": true,
        "updatedLinks": result.updated_paths.len(),
    }))
}

fn delete_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let target = gate::trash(core, &vault.root, &note).map_err(|error| error.to_string())?;
    Ok(json!({
        "deleted": vault.relative(&note),
        "trash": vault.relative(&target),
    }))
}

fn list_notes(core: &Core, arguments: &Value) -> Result<Value, String> {
    let by_links = optional_text(arguments, "sort").as_deref() == Some("links");
    let vault = if by_links {
        indexed_vault(core, arguments)?
    } else {
        requested_vault(core, arguments)?
    };
    let folder = vault.folder(&optional_text(arguments, "folder").unwrap_or_default())?;
    if !folder.is_dir() {
        return Err(format!("Папка не найдена: {}", vault.relative(&folder)));
    }
    let query = optional_text(arguments, "query").map(|value| Matcher::new(&value, IGNORE_CASE));
    let size = limit(arguments, 100, LIST_LIMIT);
    let offset = offset(arguments);

    let mut paths = vault
        .markdown_files()
        .filter(|path| path.starts_with(&folder))
        .filter(|path| {
            query
                .as_ref()
                .is_none_or(|matcher| matcher.first(&Vault::title(path)).is_some())
        })
        .collect::<Vec<_>>();
    paths.sort();
    let total = paths.len();
    let notes = if by_links {
        let counts = core
            .search
            .incoming_links(&vault.root.to_string_lossy(), &paths)
            .map_err(|error| error.to_string())?;
        let mut ranked = paths.iter().zip(counts).collect::<Vec<_>>();
        ranked.sort_by_key(|(_, links)| std::cmp::Reverse(*links));
        ranked
            .into_iter()
            .skip(offset)
            .take(size)
            .map(|(path, links)| json!({ "path": vault.relative(path), "links": links }))
            .collect::<Vec<_>>()
    } else {
        paths
            .iter()
            .skip(offset)
            .take(size)
            .map(|path| json!(vault.relative(path)))
            .collect::<Vec<_>>()
    };
    let end = offset.saturating_add(notes.len());

    Ok(json!({
        "total": total,
        "notes": notes,
        "nextOffset": (end < total).then_some(end),
    }))
}

fn list_folders(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = requested_vault(core, arguments)?;
    let mut folders = walkdir::WalkDir::new(&vault.root)
        .into_iter()
        .filter_entry(crate::search::paths::is_visible_entry)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir() && entry.depth() > 0)
        .map(|entry| vault.relative(entry.path()))
        .collect::<Vec<_>>();
    folders.sort();
    Ok(json!({ "root": vault.root.to_string_lossy(), "folders": folders }))
}

#[cfg(test)]
mod tests {
    use super::{compose, edit_from, Edit};
    use serde_json::json;

    const NOTE: &str = "# Заголовок\n\nПервый абзац.\n";
    const SECTIONS: &str = "# Один\n\nтекст\n\n## Два\n\nещё\n\n### Три\n\nглубже\n";

    fn edit(mode: &str, content: &str) -> Edit {
        Edit {
            mode: mode.to_owned(),
            content: content.to_owned(),
            find: None,
            heading: None,
            all: false,
        }
    }

    fn under(mode: &str, heading: &str, content: &str) -> Edit {
        Edit {
            heading: Some(heading.to_owned()),
            ..edit(mode, content)
        }
    }

    fn replacing(find: &str, content: &str, all: bool) -> Edit {
        Edit {
            find: Some(find.to_owned()),
            all,
            ..edit("findReplace", content)
        }
    }

    #[test]
    fn appends_and_prepends_with_a_blank_line() {
        assert_eq!(
            compose(NOTE, &edit("append", "Новый абзац.")).unwrap(),
            "# Заголовок\n\nПервый абзац.\n\nНовый абзац."
        );
        assert_eq!(
            compose(NOTE, &edit("prepend", "Сверху.")).unwrap(),
            "Сверху.\n\n# Заголовок\n\nПервый абзац.\n"
        );
    }

    #[test]
    fn appends_to_an_empty_note_without_leading_blank_lines() {
        assert_eq!(compose("", &edit("append", "Первый текст")).unwrap(), "Первый текст");
    }

    #[test]
    fn reports_missing_find_target_instead_of_silently_doing_nothing() {
        assert!(compose(NOTE, &replacing("нет такого", "Замена", false)).is_err());
        assert_eq!(
            compose(NOTE, &replacing("Первый", "Второй", false)).unwrap(),
            "# Заголовок\n\nВторой абзац.\n"
        );
    }

    #[test]
    fn refuses_to_replace_several_occurrences_unless_asked() {
        let note = "маркер и ещё маркер\n";
        let error = compose(note, &replacing("маркер", "замена", false)).unwrap_err();
        assert!(error.contains("2"), "в ошибке названо число вхождений: {error}");
        assert_eq!(
            compose(note, &replacing("маркер", "замена", true)).unwrap(),
            "замена и ещё замена\n"
        );
    }

    #[test]
    fn inserts_under_the_requested_heading_and_keeps_the_rest() {
        let updated = compose(SECTIONS, &under("underHeading", "## Два", "вставка")).unwrap();
        assert!(updated.contains("## Два\n\nвставка\n\nещё"));
        assert!(updated.contains("### Три\n\nглубже"));
        assert!(updated.ends_with('\n'), "финальный перевод строки не теряется");
        assert!(compose(SECTIONS, &under("underHeading", "Нет такого", "вставка")).is_err());
    }

    #[test]
    fn replaces_a_section_without_touching_its_neighbours() {
        let updated = compose(SECTIONS, &under("replaceSection", "### Три", "новое")).unwrap();
        assert!(updated.contains("### Три\n\nновое"));
        assert!(!updated.contains("глубже"));
        assert!(updated.contains("## Два\n\nещё"));
        assert!(updated.ends_with('\n'));
    }

    #[test]
    fn replacing_a_section_stops_at_the_next_heading_of_the_same_level() {
        let updated = compose(SECTIONS, &under("replaceSection", "## Два", "новое")).unwrap();
        assert!(updated.contains("## Два\n\nновое"));
        assert!(!updated.contains("глубже"), "вложенный раздел входит в заменяемый");
        assert!(updated.starts_with("# Один\n\nтекст\n"));
    }

    #[test]
    fn refuses_an_ambiguous_heading() {
        let note = "## Два\n\nпервый\n\n## Два\n\nвторой\n";
        let error = compose(note, &under("replaceSection", "Два", "новое")).unwrap_err();
        assert!(error.contains("2"), "в ошибке названо число заголовков: {error}");
    }

    #[test]
    fn an_empty_replacement_deletes_the_text_or_empties_the_section() {
        let deleting = edit_from(&json!({ "mode": "findReplace", "find": " абзац", "content": "" })).unwrap();
        assert_eq!(compose(NOTE, &deleting).unwrap(), "# Заголовок\n\nПервый.\n");
        let emptying = edit_from(&json!({ "mode": "replaceSection", "heading": "## Два", "content": "" })).unwrap();
        assert_eq!(compose(SECTIONS, &emptying).unwrap(), "# Один\n\nтекст\n\n## Два\n");
        assert!(edit_from(&json!({ "mode": "replace", "content": "" })).is_err(), "пустой replace стёр бы заметку");
        assert!(edit_from(&json!({ "mode": "findReplace", "find": "x" })).is_err(), "content обязателен");
    }

    #[test]
    fn rejects_an_unknown_mode_and_a_section_edit_without_a_heading() {
        assert!(compose(NOTE, &edit("patch", "текст")).is_err());
        assert!(compose(NOTE, &edit("replaceSection", "текст")).is_err());
    }
}
