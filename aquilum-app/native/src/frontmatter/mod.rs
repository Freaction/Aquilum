mod fields;

use std::ops::Range;

pub use fields::{parse, set_fields};

pub const BOOK_COVER: &str = "Book_cover";
pub const PAGE_COVER: &str = "Page_cover";
pub const BOOK_FILE: &str = "Путь к файлу";
pub const PAGE_COVER_POSITION: &str = "page_cover_position";
pub const COVER: &str = "cover";
pub const TYPE: &str = "type";
pub const BOOK_TYPE: &str = "book";
pub const PAGES: &str = "pages";
pub const AUTHOR: &str = "author";
pub const CREATED: &str = "created";
pub const TAGS: &str = "tags";
pub const STATUS: &str = "status";
pub const RATING: &str = "rating";
pub const READER_POSITION: &str = "reader_position";
pub const READ_PERCENT: &str = "read_percent";

pub const KEYS: &[&str] = &[
    COVER,
    TYPE,
    PAGE_COVER,
    PAGE_COVER_POSITION,
    BOOK_COVER,
    BOOK_FILE,
    TAGS,
    "aliases",
    CREATED,
    AUTHOR,
    STATUS,
    PAGES,
    RATING,
    READER_POSITION,
    READ_PERCENT,
];

fn aliases(key: &str) -> &'static [&'static str] {
    match key {
        BOOK_COVER => &["cover_url"],
        PAGE_COVER => &["page_cover_url"],
        BOOK_FILE => &["book_file"],
        _ => &[],
    }
}

fn names(key: &str) -> impl Iterator<Item = String> {
    std::iter::once(key).chain(aliases(key).iter().copied()).map(str::to_lowercase)
}

pub fn suggest_keys(query: &str) -> Vec<String> {
    crate::rank::rank(KEYS, query).into_iter().map(str::to_owned).collect()
}

pub fn is_key(found: &str, key: &str) -> bool {
    let found = found.trim().to_lowercase();
    names(key).any(|name| name == found)
}

pub fn fence(line: &str) -> bool {
    line.trim_end_matches([' ', '\t']) == "---"
}

pub fn end_of(text: &crop::Rope) -> Option<usize> {
    let (mut at, mut end, mut first) = (0, None, true);
    crate::rope::each_line(text.byte_slice(..), |line| {
        let start = at;
        at += line.len() + 1;
        if std::mem::take(&mut first) {
            return fence(line);
        }
        if fence(line) {
            end = Some(start + line.len());
            return false;
        }
        true
    });
    end
}

pub fn hidden_of(text: &crop::Rope) -> usize {
    end_of(text).map_or(0, |end| end + usize::from(end < text.byte_len() && text.byte(end) == b'\n'))
}

pub fn head(text: &crop::Rope) -> String {
    text.byte_slice(..hidden_of(text)).to_string()
}

struct Block {
    body: Range<usize>,
    end: usize,
}

fn block(text: &str) -> Option<Block> {
    let first = text.find('\n')?;
    if !fence(&text[..first]) {
        return None;
    }
    let start = first + 1;
    let mut line = start;
    loop {
        let stop = text[line..].find('\n').map_or(text.len(), |at| line + at);
        if fence(&text[line..stop]) {
            return Some(Block { body: start..line.saturating_sub(1).max(start), end: stop });
        }
        if stop == text.len() {
            return None;
        }
        line = stop + 1;
    }
}

pub fn end(text: &str) -> Option<usize> {
    block(text).map(|block| block.end)
}

pub fn template(fields: &[(&str, &str)]) -> String {
    let lines: String = fields.iter().map(|(key, value)| format!("{key}: {value}\n")).collect();
    format!("---\n{lines}---\n")
}

pub fn parse_pages(value: &str) -> Option<(u32, u32)> {
    let (current, total) = value.split_once('/')?;
    let (current, total) = (current.trim().parse().ok()?, total.trim().parse::<u32>().ok()?);
    (total > 0).then_some((current, total))
}

pub fn format_pages(current: u32, total: u32) -> String {
    format!("{current}/{total}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_block_between_two_fence_lines() {
        assert_eq!(end("---\nkey: 1\n---\nтекст"), Some(14));
        assert_eq!(end("---\n\n---"), Some(8));
        assert_eq!(end("---\n---"), Some(7));
        assert_eq!(end("текст\n---\nkey: 1\n---"), None);
        assert_eq!(end("---\r\nkey: 1\r\n---"), None);
        assert_eq!(end("---\nkey: 1"), None);
        let hidden = |text: &str| hidden_of(&crop::Rope::from(text));
        assert_eq!(hidden("---\nkey: 1\n---\nтекст"), 15);
        assert_eq!(hidden("---\nkey: 1\n---"), 14);
        assert_eq!(hidden("текст"), 0);
        assert_eq!(end_of(&crop::Rope::from("---\nkey: 1\n---\nтекст")), Some(14));
        assert_eq!(end_of(&crop::Rope::from("---\nkey: 1")), None);
    }

    #[test]
    fn only_a_line_of_exactly_three_dashes_closes_the_block() {
        let text = "---\nkey: 1\n----\nstill: yes\n---abc\n---  \nтекст";
        assert_eq!(end(text), Some(text.find("\nтекст").unwrap()));
        assert_eq!(end("---\nkey: 1\n----\nтекст"), None);
        assert_eq!(end("---\nkey: 1\n---abc\nтекст"), None);
        assert_eq!(end("----\nkey: 1\n---"), None);
        assert_eq!(end("---abc\nkey: 1\n---"), None);
        let fields = parse(text).unwrap();
        assert_eq!(fields.text("still"), Some("yes"));
    }

    #[test]
    fn template_and_pages() {
        assert_eq!(template(&[(TAGS, "[]"), (AUTHOR, "")]), "---\ntags: []\nauthor: \n---\n");
        assert_eq!(end(&template(&[(TAGS, "[]")])), Some(16));
        assert_eq!(parse_pages("12 / 300"), Some((12, 300)));
        assert_eq!(parse_pages("1/0"), None);
        assert_eq!(parse_pages("abc"), None);
        assert_eq!(parse_pages("1.5/10"), None);
        assert_eq!(format_pages(3, 10), "3/10");
    }

    #[test]
    fn keys_ignore_case_in_lookup_and_suggestions() {
        assert!(is_key(" PAGE_COVER_URL ", PAGE_COVER));
        assert!(!is_key("page_cover_position", PAGE_COVER));
        assert_eq!(suggest_keys("PAGE_c"), ["Page_cover", "page_cover_position"]);
        assert_eq!(suggest_keys("путь"), ["Путь к файлу"]);
    }
}
