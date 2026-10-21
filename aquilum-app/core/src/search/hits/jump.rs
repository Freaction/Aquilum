use super::file::Located;
use crate::files::document::text_from_bytes;
use crate::search::headings::heading_near;
use crate::search::pages::{key_hash, Placed};
use crate::text::matcher::Matcher;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub fn locate(path: &Path, pages: &[Placed], hits: &HashSet<String>, matcher: &Matcher) -> Option<Located> {
    let mut file = File::open(path).ok()?;
    for (index, page) in pages.iter().enumerate().filter(|(_, page)| hits.contains(&page.key)) {
        let end = pages.get(index + 1).map(|next| next.start);
        let text = read(&mut file, page, end)?;
        let Some(found) = matcher.first(&text) else { continue };
        let heading = heading_near(&text, found.start, || index.checked_sub(1).and_then(|before| read(&mut file, &pages[before], Some(page.start))));
        return Some(Located::new(&text, page.start_utf16, found.start, end.is_none(), heading));
    }
    None
}

fn read(file: &mut File, page: &Placed, end: Option<usize>) -> Option<String> {
    file.seek(SeekFrom::Start(page.start as u64)).ok()?;
    let mut bytes = Vec::new();
    match end {
        Some(end) => file.by_ref().take(end.checked_sub(page.start)? as u64).read_to_end(&mut bytes).ok()?,
        None => file.read_to_end(&mut bytes).ok()?,
    };
    let text = text_from_bytes(bytes).ok()?;
    (Some(blake3::hash(text.as_bytes())) == key_hash(&page.key)).then_some(text)
}

#[cfg(test)]
#[path = "jump_tests.rs"]
mod tests;
