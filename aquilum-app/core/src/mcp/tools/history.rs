use super::notes::window;
use super::workspace::requested_vault;
use super::{offset, 
    limit, open_after_write, optional_text, require_write, text, tool, version_name_property,
    workspace_property, write_retrying,
};
use crate::files::gate;
use crate::history::{version_at, Source};
use serde_json::{json, Value};
use crate::app_core::Core;

const LIST_LIMIT: usize = 200;
const READ_LIMIT: usize = 8_000;

pub fn definitions() -> Vec<Value> {
    vec![
        tool(
            "note_history",
            "Версии заметки из её истории, от новых к старым. id — для read_version, restore_version и name_version; atMs — время в миллисекундах Unix; source — кто правил: me — пользователь, agent — агент (каждый вызов агента — отдельная версия), external — правка снаружи, links — ссылки после переименования другой заметки, start — исходный текст, restore и revert — восстановление версии и отмена её изменений (fromMs — время той версии); name — название версии.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "offset": { "type": "integer", "description": "Сколько версий пропустить, чтобы получить следующую страницу" },
                "limit": { "type": "integer", "description": "Сколько версий вернуть, по умолчанию 50, максимум 200" },
                "workspace": workspace_property(),
            }),
            &["note"],
        ),
        tool(
            "read_version",
            "Текст одной версии заметки по id из note_history. Длинный текст отдаётся частями: смотрите поля truncated и nextOffset.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "version": { "type": "string", "description": "id версии из note_history" },
                "offset": { "type": "integer", "description": "С какого символа читать" },
                "limit": { "type": "integer", "description": "Сколько символов вернуть, по умолчанию 8000" },
                "workspace": workspace_property(),
            }),
            &["note", "version"],
        ),
        tool(
            "restore_version",
            "Восстановить версию заметки: её текст становится текущим. Текст до восстановления сам остаётся в истории, поэтому восстановление обратимо. Вызывайте по просьбе пользователя.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "version": { "type": "string", "description": "id версии из note_history" },
                "versionName": version_name_property(),
                "open": { "type": "boolean", "description": "Открыть заметку вкладкой в приложении, по умолчанию да; в другой базе вкладка не открывается" },
                "workspace": workspace_property(),
            }),
            &["note", "version"],
        ),
        tool(
            "name_version",
            "Назвать версию заметки, переименовать её или снять название пустым name. Без version называется последняя версия заметки. Названные версии не удаляются по сроку хранения истории.",
            json!({
                "note": { "type": "string", "description": "Путь или название заметки" },
                "version": { "type": "string", "description": "id версии из note_history; без него — последняя версия заметки" },
                "name": { "type": "string", "description": "Название; пустая строка снимает его" },
                "workspace": workspace_property(),
            }),
            &["note", "name"],
        ),
    ]
}

pub fn call(core: &Core, name: &str, arguments: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "note_history" => note_history(core, arguments),
        "read_version" => read_version(core, arguments),
        "restore_version" => restore_version(core, arguments),
        "name_version" => name_version(core, arguments),
        _ => return None,
    })
}

fn missing_version(id: &str) -> String {
    format!("У заметки нет версии {id}: возьмите id из note_history")
}

fn note_history(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let offset = offset(arguments);
    let history = &core.history;
    let page = history.page(&|| core.known_roots(), &note, offset, limit(arguments, 50, LIST_LIMIT));
    let end = offset.saturating_add(page.versions.len());
    Ok(json!({
        "path": vault.relative(&note),
        "total": page.total,
        "versions": page.versions,
        "nextOffset": (end < page.total).then_some(end),
    }))
}

fn read_version(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let id = text(arguments, "version")?;
    let history = &core.history;
    let texts = history
        .version(&|| core.known_roots(), &note, Some(&id))
        .ok_or_else(|| missing_version(&id))?;
    let offset = offset(arguments);
    let size = limit(arguments, READ_LIMIT, 40_000);
    let (body, total) = window(&texts.text, offset, size)?;
    let end = offset.saturating_add(size.min(total.saturating_sub(offset)));
    Ok(json!({
        "path": vault.relative(&note),
        "version": id,
        "content": body,
        "totalChars": total,
        "truncated": end < total,
        "nextOffset": (end < total).then_some(end),
    }))
}

fn restore_version(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let id = text(arguments, "version")?;
    let from_ms = version_at(&id).ok_or_else(|| missing_version(&id))?;
    let history = &core.history;
    let texts = history
        .version(&|| core.known_roots(), &note, Some(&id))
        .ok_or_else(|| missing_version(&id))?;
    let version_name = optional_text(arguments, "versionName");
    let (changed, _) = write_retrying(core, &note, Source::Restore(from_ms), version_name.as_deref(), |_| {
        Ok(texts.text.clone())
    })?;
    if changed {
        open_after_write(core, &vault, &note, arguments);
    }
    Ok(json!({ "path": vault.relative(&note), "version": id, "changed": changed }))
}

fn name_version(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let version = optional_text(arguments, "version");
    let name = optional_text(arguments, "name").unwrap_or_default();
    let named = gate::name_version(core, &note, version.as_deref(), &name)
        .ok_or_else(|| match &version {
            Some(id) => missing_version(id),
            None => "У заметки ещё нет версий: называть нечего".to_owned(),
        })?;
    Ok(json!({ "path": vault.relative(&note), "version": named, "name": name.trim() }))
}
