use super::target::{is_attachment, parts};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WikiLink {
    pub target: String,
    pub note_range: Range<usize>,
    pub offset_utf16: usize,
}

struct Found {
    start: usize,
    note_range: Range<usize>,
    target: String,
}

pub fn extract(source: &str) -> Vec<WikiLink> {
    let excluded = excluded_ranges(source);
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    let mut cursor = 0;
    let mut excluded_at = 0;
    while cursor + 3 < bytes.len() {
        if bytes[cursor] != b'[' || bytes[cursor + 1] != b'[' || escaped(bytes, cursor) {
            cursor += 1;
            continue;
        }
        let Some(end) = find_close(bytes, cursor + 2) else {
            cursor += 2;
            continue;
        };
        let full = cursor..end + 2;
        while excluded_at < excluded.len() && excluded[excluded_at].end <= full.start {
            excluded_at += 1;
        }
        let inside_code = excluded
            .get(excluded_at)
            .is_some_and(|range| overlaps(range, &full));
        let embed = cursor > 0 && bytes[cursor - 1] == b'!' && !escaped(bytes, cursor - 1);
        let inner = &source[cursor + 2..end];
        let split = parts(inner);
        let note = &inner[split.note.clone()];
        let skipped = inside_code || note.is_empty() || (embed && is_attachment(note));
        if !skipped {
            let target_end = split.anchor.as_ref().map_or(split.note.end, |anchor| anchor.end);
            found.push(Found {
                start: cursor,
                note_range: cursor + 2 + split.note.start..cursor + 2 + split.note.end,
                target: inner[split.note.start..target_end].to_owned(),
            });
        }
        cursor = end + 2;
    }
    let mut prior = 0;
    let mut offset_utf16 = 0;
    found
        .into_iter()
        .map(|link| {
            offset_utf16 += source[prior..link.start].encode_utf16().count();
            prior = link.start;
            WikiLink {
                target: link.target,
                note_range: link.note_range,
                offset_utf16,
            }
        })
        .collect()
}
fn excluded_ranges(source: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut block_start = None;
    for (event, range) in Parser::new_ext(source, Options::all()).into_offset_iter() {
        match event {
            Event::Code(_) | Event::Html(_) | Event::InlineHtml(_) => ranges.push(range),
            Event::Start(Tag::CodeBlock(_)) => block_start = Some(range.start),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = block_start.take() {
                    ranges.push(start..range.end);
                }
            }
            _ => {}
        }
    }
    ranges.sort_by_key(|range| range.start);
    ranges
}

fn find_close(bytes: &[u8], mut cursor: usize) -> Option<usize> {
    while cursor + 1 < bytes.len() {
        if bytes[cursor] == b'\n' || bytes[cursor] == b'\r' {
            return None;
        }
        if bytes[cursor] == b']' && bytes[cursor + 1] == b']' && !escaped(bytes, cursor) {
            return Some(cursor);
        }
        cursor += 1;
    }
    None
}

fn escaped(bytes: &[u8], at: usize) -> bool {
    let mut count = 0;
    let mut cursor = at;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        count += 1;
        cursor -= 1;
    }
    count % 2 == 1
}

fn overlaps(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

#[cfg(test)]
mod tests {
    use super::extract;

    fn targets(source: &str) -> Vec<String> {
        extract(source).into_iter().map(|link| link.target).collect()
    }

    #[test]
    fn extracts_targets_and_ignores_code() {
        let source = "[[Note]] [[Folder/Name|alias]] `[[Code]]`\n```\n[[Block]]\n```";
        assert_eq!(targets(source), ["Note", "Folder/Name"]);
    }

    #[test]
    fn continues_after_an_unclosed_link() {
        let links = extract("[[broken\n[[Note]]");
        assert_eq!(links[0].target, "Note");
    }

    #[test]
    fn keeps_the_anchor_in_the_target_but_not_in_the_note_range() {
        let source = "см. [[Заметка#Заголовок|подпись]]";
        let link = &extract(source)[0];
        assert_eq!(link.target, "Заметка#Заголовок");
        assert_eq!(&source[link.note_range.clone()], "Заметка");
        assert_eq!(link.offset_utf16, "см. ".encode_utf16().count());
    }

    #[test]
    fn links_to_the_current_note_are_not_edges() {
        assert!(extract("[[#Заголовок]] и [[ #^abc ]]").is_empty());
    }

    #[test]
    fn table_cell_alias_does_not_leak_a_backslash_into_the_target() {
        let source = r"| [[Note\|alias]] | [[Folder/Note#H\|a]] |";
        let links = extract(source);
        assert_eq!(links.iter().map(|link| link.target.as_str()).collect::<Vec<_>>(), ["Note", "Folder/Note#H"]);
        assert_eq!(&source[links[0].note_range.clone()], "Note");
    }

    #[test]
    fn embedded_files_are_not_links_but_embedded_notes_are() {
        let source = "![[image.png|610]] ![[document.pdf#page=3]] ![[Заметка]] ![[Заметка#Раздел]] [[image.png]]";
        assert_eq!(targets(source), ["Заметка", "Заметка#Раздел", "image.png"]);
    }

    #[test]
    fn escaped_bang_keeps_a_plain_link() {
        assert_eq!(targets(r"\![[image.png]]"), ["image.png"]);
    }
}
