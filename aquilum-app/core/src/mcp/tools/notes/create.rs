use super::*;

pub(super) fn create_note(core: &Core, arguments: &Value) -> Result<Value, String> {
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
            super::super::super::bridge::open_note_after_write(core, last);
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

pub(super) fn create_one(
    core: &Core,
    vault: &Vault,
    requested: &str,
    initial: &str,
    version_name: Option<&str>,
) -> Result<PathBuf, String> {
    let path = vault.note_path(requested)?;
    super::moves::free(vault, &path)?;
    gate::create(core, &path, &initial, Source::Agent, version_name).map_err(|error| error.to_string())?;
    Ok(path)
}

pub(super) fn requested_notes(arguments: &Value) -> Result<Vec<(String, String)>, String> {
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
