use super::*;

pub(super) fn read_note(core: &Core, arguments: &Value) -> Result<Value, String> {
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

pub(super) fn read_one(vault: &Vault, name: &str, arguments: &Value, size: usize) -> Result<Value, String> {
    let note = vault.note(name)?;
    let failed = |error: std::io::Error| error.to_string();
    let meta = super::super::frontmatter::map(&read_head(&note).map_err(failed)?);
    let (section, heading) = match optional_text(arguments, "heading").as_deref() {
        Some(wanted) => {
            let (outline, _) = headings::outline(&mut Pages::open(&note).map_err(failed)?).map_err(failed)?;
            let heading = single_heading(&outline, wanted)?;
            (heading.start_char..heading.chars.end, Some(heading.title.clone()))
        }
        None => (0..usize::MAX, None),
    };
    let mut window = text_window(&mut Pages::open(&note).map_err(failed)?, section, arguments, size)?;
    window["path"] = json!(vault.relative(&note));
    window["frontmatter"] = meta;
    window["heading"] = json!(heading);
    Ok(window)
}

pub(super) fn note_outline(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = requested_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let failed = |error: std::io::Error| error.to_string();
    let (outline, total) = headings::outline(&mut Pages::open(&note).map_err(failed)?).map_err(failed)?;
    let items = outline
        .iter()
        .map(|heading| json!({ "level": heading.level, "heading": heading.title, "chars": heading.chars.len() }))
        .collect::<Vec<_>>();
    Ok(json!({
        "note": vault.relative(&note),
        "totalChars": total,
        "outline": items,
    }))
}

pub(super) fn single_heading<'a>(outline: &'a [Heading], wanted: &str) -> Result<&'a Heading, String> {
    let found = headings::matching(outline, wanted);
    match found.len() {
        0 => Err(format!("Заголовок не найден: {wanted}")),
        1 => Ok(found[0]),
        count => Err(format!(
            "Заголовок «{wanted}» встречается в заметке больше одного раза (всего: {count}) — правьте через findReplace или переименуйте один из разделов"
        )),
    }
}
