mod fields;

use aquilum_core::search::analyzer::frontmatter::bounds;
use aquilum_core::text::content::Content;

pub use fields::{Fields, parse, set};

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
    names(key).any(|name| aquilum_core::search::analyzer::frontmatter::same_key(found, &name))
}

pub use aquilum_core::search::analyzer::frontmatter::{Bounds, fence};

pub fn bounds_of(text: &dyn Content) -> Option<Bounds> {
    bounds(&aquilum_core::files::document::head(text).ok()?)
}

pub fn head(text: &dyn Content) -> String {
    let mut head = aquilum_core::files::document::head(text).unwrap_or_default();
    head.truncate(bounds(&head).map_or(0, |found| found.body));
    head
}

pub fn end(text: &str) -> Option<usize> {
    bounds(text).map(|found| found.end)
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
        let hidden = |text: &str| bounds_of(&text).map_or(0, |found| found.body);
        assert_eq!(hidden("---\nkey: 1\n---\nтекст"), 15);
        assert_eq!(hidden("---\nkey: 1\n---"), 14);
        assert_eq!(hidden("текст"), 0);
        assert_eq!(bounds_of(&"---\nkey: 1\n---\nтекст").map(|found| found.end), Some(14));
        assert!(bounds_of(&"---\nkey: 1").is_none());
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
    }

    #[test]
    fn keys_ignore_case_in_lookup_and_suggestions() {
        assert!(is_key(" PAGE_COVER_URL ", PAGE_COVER));
        assert!(!is_key("page_cover_position", PAGE_COVER));
        assert_eq!(suggest_keys("PAGE_c"), ["Page_cover", "page_cover_position"]);
        assert_eq!(suggest_keys("путь"), ["Путь к файлу"]);
    }
}
