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
        return relocate(core, &vault, &note, target, "moved");
    }
    free(&destination, &target)?;

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
