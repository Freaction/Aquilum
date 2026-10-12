use std::collections::HashMap;

use super::{BOOK_TYPE, COVER, TYPE, block, is_key, names};

fn split_line(line: &str) -> (String, Option<String>) {
    if let Some((key, value)) = line.split_once(':') {
        return (key.trim().to_owned(), Some(value.trim().to_owned()));
    }
    let trimmed = line.trim();
    match trimmed.split_once(' ') {
        Some((key, value)) => (key.trim().to_owned(), Some(value.trim().to_owned())),
        None => (trimmed.to_owned(), None),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Text(String),
    List(Vec<String>),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fields(HashMap<String, Value>);

impl Fields {
    pub fn text(&self, key: &str) -> Option<&str> {
        names(key).find_map(|k| match self.0.get(&k) {
            Some(Value::Text(v)) if !v.trim().is_empty() => Some(v.trim()),
            _ => None,
        })
    }

    pub fn has_page_cover(&self) -> bool {
        matches!(self.0.get(COVER), Some(Value::Text(v)) if v.trim().eq_ignore_ascii_case("true"))
    }

    pub fn is_book(&self) -> bool {
        matches!(self.0.get(TYPE), Some(Value::Text(v)) if v.trim() == BOOK_TYPE)
    }
}

pub fn parse(text: &str) -> Option<Fields> {
    let block = block(text)?;
    let mut fields = HashMap::new();
    for line in text[block.body].split('\n') {
        let (key, value) = split_line(line);
        let Some(value) = value else { continue };
        if key.is_empty() {
            continue;
        }
        let value = if value.starts_with('[') && value.ends_with(']') && value.len() >= 2 {
            Value::List(value[1..value.len() - 1].split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect())
        } else {
            let quoted = value.len() >= 2 && ((value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\'')));
            Value::Text(if quoted { value[1..value.len() - 1].to_owned() } else { value })
        };
        fields.insert(key.to_lowercase(), value);
    }
    Some(Fields(fields))
}

fn set_field(text: &str, key: &str, value: &str) -> Option<String> {
    let block = block(text)?;
    let body = &text[block.body];
    let mut found = false;
    let mut lines: Vec<String> = Vec::new();
    if !body.is_empty() {
        for line in body.split('\n') {
            if !is_key(&split_line(line).0, key) {
                lines.push(line.to_owned());
                continue;
            }
            found = true;
            if !value.is_empty() {
                lines.push(format!("{key}: {value}"));
            }
        }
    }
    if !found && !value.is_empty() {
        lines.push(format!("{key}: {value}"));
    }
    Some(format!("---\n{}\n---{}", lines.join("\n"), &text[block.end..]))
}

pub fn set_fields(text: &str, updates: &[(&str, String)]) -> Option<String> {
    updates.iter().try_fold(text.to_owned(), |text, (key, value)| set_field(&text, key, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::{BOOK_COVER, BOOK_FILE, PAGE_COVER, PAGE_COVER_POSITION};

    #[test]
    fn parses_and_sets_fields_with_aliases() {
        let doc = "---\ncover: True\ntype: book\npage_cover_url: \"pattern:polka\"\ntags: [a, b]\n---\nтекст";
        let fields = parse(doc).unwrap();
        assert!(fields.has_page_cover() && fields.is_book());
        assert_eq!(fields.text(PAGE_COVER), Some("pattern:polka"));
        assert_eq!(fields.0.get("tags"), Some(&Value::List(vec!["a".into(), "b".into()])));
        let next = set_field(doc, PAGE_COVER, "pattern:weave").unwrap();
        assert_eq!(next, "---\ncover: True\ntype: book\nPage_cover: pattern:weave\ntags: [a, b]\n---\nтекст");
        let next = set_field(&next, PAGE_COVER_POSITION, "30%").unwrap();
        assert!(next.contains("\npage_cover_position: 30%\n---"));
        let next = set_field(&next, COVER, "").unwrap();
        assert!(!next.contains("cover: True"));
        assert_eq!(set_field("текст", COVER, "true"), None);
    }

    #[test]
    fn keys_ignore_case_in_lookup_and_rewrite() {
        let doc = "---\nCover: true\npage_cover: pattern:polka\nBOOK_COVER: Files/x.png\n---\nтекст";
        let fields = parse(doc).unwrap();
        assert!(fields.has_page_cover());
        assert_eq!(fields.text(PAGE_COVER), Some("pattern:polka"));
        assert_eq!(fields.text(BOOK_COVER), Some("Files/x.png"));
        let next = set_field(doc, PAGE_COVER, "pattern:weave").unwrap();
        assert_eq!(next, "---\nCover: true\nPage_cover: pattern:weave\nBOOK_COVER: Files/x.png\n---\nтекст");
    }

    #[test]
    fn several_fields_are_applied_in_order_or_not_at_all() {
        let doc = "---\ntype: book\n---\nтекст";
        let next = set_fields(doc, &[(BOOK_FILE, "Files/a.epub".into()), (COVER, "true".into())]).unwrap();
        assert_eq!(next, "---\ntype: book\nПуть к файлу: Files/a.epub\ncover: true\n---\nтекст");
        assert_eq!(set_fields("текст", &[(COVER, "true".into())]), None);
        assert_eq!(set_fields(doc, &[]).as_deref(), Some(doc));
    }
}
