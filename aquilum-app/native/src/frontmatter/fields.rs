use std::ops::Range;

use aquilum_core::search::analyzer::frontmatter::{bounds, set as set_head};
use aquilum_core::search::fields::{Field, FieldKind, note_fields};

use super::{BOOK_TYPE, COVER, TYPE, is_key, names};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fields(Vec<Field>);

impl Fields {
    pub fn text(&self, key: &str) -> Option<&str> {
        names(key).find_map(|name| {
            let field = self.0.iter().find(|field| field.key.to_lowercase() == name && field.kind != FieldKind::List)?;
            Some(field.text.trim()).filter(|text| !text.is_empty())
        })
    }

    pub fn has_page_cover(&self) -> bool {
        self.text(COVER).is_some_and(|value| value.eq_ignore_ascii_case("true"))
    }

    pub fn is_book(&self) -> bool {
        self.text(TYPE) == Some(BOOK_TYPE)
    }
}

pub fn parse(text: &str) -> Option<Fields> {
    bounds(text).map(|_| Fields(note_fields(text)))
}

pub fn set(head: &str, fields: &[(&str, String)]) -> Option<(Range<usize>, String)> {
    let updates = fields.iter().map(|(key, value)| (*key, Some(value.as_str()).filter(|value| !value.is_empty()))).collect::<Vec<_>>();
    set_head(head, &updates, is_key).ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::{BOOK_COVER, BOOK_FILE, PAGE_COVER, PAGE_COVER_POSITION};

    fn apply(head: &str, fields: &[(&str, String)]) -> String {
        set(head, fields).map_or(head.to_owned(), |(range, insert)| format!("{}{insert}{}", &head[..range.start], &head[range.end..]))
    }

    #[test]
    fn parses_and_sets_fields_with_aliases() {
        let doc = "---\ncover: True\ntype: book\npage_cover_url: \"pattern:polka\"\ntags: [a, b]\n---\nтекст";
        let fields = parse(doc).unwrap();
        assert!(fields.has_page_cover() && fields.is_book());
        assert_eq!(fields.text(PAGE_COVER), Some("pattern:polka"));
        assert_eq!(fields.text("tags"), None);
        let next = apply(doc, &[(PAGE_COVER, "pattern:weave".into())]);
        assert_eq!(next, "---\ncover: True\ntype: book\npage_cover_url: pattern:weave\ntags: [a, b]\n---\nтекст");
        let next = apply(&next, &[(PAGE_COVER_POSITION, "30%".into())]);
        assert!(next.contains("\npage_cover_position: 30%\n---"));
        let next = apply(&next, &[(COVER, String::new()), (PAGE_COVER, String::new())]);
        assert_eq!(next, "---\ntype: book\ntags: [a, b]\npage_cover_position: 30%\n---\nтекст");
        assert_eq!(parse("текст"), None);
    }

    #[test]
    fn keys_ignore_case_in_lookup_and_rewrite() {
        let doc = "---\nCover: true\npage_cover: pattern:polka\nBOOK_COVER: Files/x.png\n---\nтекст";
        let fields = parse(doc).unwrap();
        assert!(fields.has_page_cover());
        assert_eq!(fields.text(PAGE_COVER), Some("pattern:polka"));
        assert_eq!(fields.text(BOOK_COVER), Some("Files/x.png"));
        let next = apply(doc, &[(PAGE_COVER, "pattern:weave".into())]);
        assert_eq!(next, "---\nCover: true\npage_cover: pattern:weave\nBOOK_COVER: Files/x.png\n---\nтекст");
    }

    #[test]
    fn several_fields_are_applied_in_order_and_a_note_without_fields_gets_a_block() {
        let doc = "---\ntype: book\n---\nтекст";
        let next = apply(doc, &[(BOOK_FILE, "Files/a.epub".into()), (COVER, "true".into())]);
        assert_eq!(next, "---\ntype: book\nПуть к файлу: Files/a.epub\ncover: true\n---\nтекст");
        assert_eq!(apply("текст", &[(COVER, "true".into())]), "---\ncover: true\n---\nтекст");
        assert_eq!(set(doc, &[]), None);
        assert_eq!(set(doc, &[(COVER, String::new())]), None);
    }
}
