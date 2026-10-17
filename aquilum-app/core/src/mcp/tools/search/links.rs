use crate::app_core::Core;
use crate::markdown::line::bounds;
use crate::mcp::tools::workspace::indexed_vault;
use crate::mcp::tools::shorten;
use crate::search::headings::find_spot;
use crate::search::pages::Pages;
use crate::text::cancel::Cancel;
use crate::text::utf16::utf16_byte;
use serde_json::{json, Value};
use std::path::Path;

const CONTEXT_LIMIT: usize = 200;

pub fn backlinks(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = indexed_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let links = core
        .search
        .backlinks(&vault.root.to_string_lossy(), &note.to_string_lossy())
        .map_err(|error| error.to_string())?;
    let items = mentions(&links, |left, right| left.path == right.path)
        .into_iter()
        .map(|(first, count)| {
            let link = &links[first];
            let (heading, context) = link_context(Path::new(&link.path), link.offset);
            json!({
                "path": vault.relative(Path::new(&link.path)),
                "mentions": count,
                "heading": heading,
                "context": context,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "note": vault.relative(&note), "backlinks": items }))
}

pub fn outgoing_links(core: &Core, arguments: &Value) -> Result<Value, String> {
    let vault = indexed_vault(core, arguments)?;
    let note = vault.note_argument(arguments)?;
    let links = core
        .search
        .outgoing_links(&vault.root.to_string_lossy(), &note.to_string_lossy())
        .map_err(|error| error.to_string())?;
    let items = mentions(&links, |left, right| left.target == right.target)
        .into_iter()
        .map(|(first, count)| {
            let link = &links[first];
            json!({
                "target": link.target,
                "path": link.path.as_deref().map(|path| vault.relative(Path::new(path))),
                "exists": link.path.is_some(),
                "mentions": count,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "note": vault.relative(&note), "links": items }))
}

fn mentions<T>(items: &[T], same: impl Fn(&T, &T) -> bool) -> Vec<(usize, usize)> {
    let mut groups = Vec::<(usize, usize)>::new();
    for (position, item) in items.iter().enumerate() {
        match groups.iter_mut().find(|(first, _)| same(&items[*first], item)) {
            Some((_, count)) => *count += 1,
            None => groups.push((position, 1)),
        }
    }
    groups
}

fn link_context(path: &Path, offset_utf16: usize) -> (Option<String>, Option<String>) {
    let spot = Pages::open(path).and_then(|mut pages| {
        find_spot(&mut pages, &Cancel::never(), |page| {
            let local = offset_utf16.checked_sub(page.start_utf16)?;
            let at = utf16_byte(&page.text, local);
            (at < page.text.len()).then_some(at)
        })
    });
    let Ok(Some(spot)) = spot else {
        return (None, None);
    };
    let line = shorten(spot.page.text[bounds(&spot.page.text, spot.at)].trim(), CONTEXT_LIMIT);
    (spot.heading, Some(line))
}

#[cfg(test)]
mod tests {
    use super::link_context;
    use std::fs;

    #[test]
    fn the_context_is_the_line_and_section_of_the_link() {
        let directory = tempfile::tempdir().unwrap();
        let note = directory.path().join("Источник.md");
        fs::write(&note, "# Раздел\n\nтекст [[Цель]] хвост\nдальше\n").unwrap();
        let (heading, context) = link_context(&note, 16);
        assert_eq!(heading.as_deref(), Some("Раздел"));
        assert_eq!(context.as_deref(), Some("текст [[Цель]] хвост"));
    }
}
