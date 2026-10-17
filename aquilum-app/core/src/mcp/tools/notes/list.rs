use super::*;

pub(super) fn list_notes(core: &Core, arguments: &Value) -> Result<Value, String> {
    let by_links = optional_text(arguments, "sort").as_deref() == Some("links");
    let vault = if by_links {
        indexed_vault(core, arguments)?
    } else {
        requested_vault(core, arguments)?
    };
    let folder = vault.folder_argument(arguments)?;
    let query = optional_text(arguments, "query").map(|value| Matcher::new(&value, IGNORE_CASE));
    let size = limit(arguments, 100, LIST_LIMIT);
    let offset = offset(arguments);

    let mut paths = vault
        .markdown_files()
        .filter(|path| path.starts_with(&folder))
        .filter(|path| {
            query
                .as_ref()
                .is_none_or(|matcher| matcher.first(&crate::search::paths::title(path)).is_some())
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
    Ok(listing("notes", total, offset, notes))
}

pub(super) fn list_folders(core: &Core, arguments: &Value) -> Result<Value, String> {
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
