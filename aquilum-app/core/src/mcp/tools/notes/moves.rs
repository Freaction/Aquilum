use super::*;

pub(super) fn rename_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = indexed_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let title = text(arguments, "title")?;
    if title.contains(['/', '\\']) {
        return Err("Название не должно содержать разделители пути — используйте move_note".to_owned());
    }
    let target = note.with_file_name(format!("{}.md", title.trim_end_matches(".md")));
    relocate(core, &vault, &note, target, "renamed")
}

pub(super) fn move_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = indexed_vault(core, arguments)?;
    let notes = match optional_names(arguments, "notes")? {
        Some(many) => many.iter().map(|name| vault.note(name)).collect::<Result<Vec<_>, _>>()?,
        None => vec![vault.note_argument(arguments)?],
    };
    let destination = match optional_text(arguments, "toWorkspace") {
        Some(input) => workspace_vault(core, &input)?,
        None => vault.clone(),
    };
    let into = destination.folder(&optional_text(arguments, "folder").unwrap_or_default())?;
    let targets = notes.iter().map(|note| note.file_name().map(|name| into.join(name)).ok_or("Не удалось определить имя файла")).collect::<Result<Vec<_>, _>>()?;
    if destination.root != vault.root {
        let mut moved = transfer::across(core, &vault, &destination, &notes, &targets)?;
        if let [target] = targets.as_slice() {
            moved["path"] = json!(destination.relative(target));
        }
        return Ok(moved);
    }
    let mut moved = notes.iter().zip(targets).map(|(note, target)| relocate(core, &vault, note, target, "moved")).collect::<Result<Vec<_>, _>>()?;
    Ok(if moved.len() == 1 { moved.remove(0) } else { json!({ "moved": moved }) })
}

fn relocate(core: &Core, vault: &Vault, note: &Path, target: PathBuf, done: &str) -> Result<Value, String> {
    if target == note {
        return Ok(json!({ "path": vault.relative(note), done: false }));
    }
    free(vault, &target)?;
    let result = gate::rename(core, note, &target).map_err(|error| error.to_string())?;
    Ok(json!({ "path": vault.relative(&target), done: true, "updatedLinks": result.updated_paths.len() }))
}

pub(super) fn free(vault: &Vault, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Err(format!("Заметка уже существует: {}", vault.relative(target)));
    }
    target.parent().map_or(Ok(()), |parent| std::fs::create_dir_all(parent).map_err(|error| error.to_string()))
}

pub(super) fn delete_note(core: &Core, arguments: &Value) -> Result<Value, String> {
    require_write(core)?;
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let target = gate::trash(core, &vault.root, &note).map_err(|error| error.to_string())?;
    Ok(json!({
        "deleted": vault.relative(&note),
        "trash": vault.relative(&target),
    }))
}
