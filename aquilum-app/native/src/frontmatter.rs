use std::collections::HashMap;

pub const BOOK_COVER: &str = "Book_cover";
pub const PAGE_COVER: &str = "Page_cover";
pub const BOOK_FILE: &str = "Путь к файлу";
pub const PAGE_COVER_POSITION: &str = "page_cover_position";

fn aliases(key: &str) -> &'static [&'static str] {
    match key {
        BOOK_COVER => &["cover_url"],
        PAGE_COVER => &["page_cover_url"],
        BOOK_FILE => &["book_file"],
        _ => &[],
    }
}

pub fn end(text: &str) -> Option<usize> {
    let rest = text.strip_prefix("---\n")?;
    let close = rest.find("\n---")?;
    Some(4 + close + 4)
}

pub fn hidden_len(text: &str) -> usize {
    match end(text) {
        Some(end) if text[end..].starts_with('\n') => end + 1,
        Some(end) => end,
        None => 0,
    }
}

fn body(text: &str) -> Option<&str> {
    let end = end(text)?;
    Some(&text[4..end - 4])
}

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
        std::iter::once(key).chain(aliases(key).iter().copied()).find_map(|k| match self.0.get(k) {
            Some(Value::Text(v)) if !v.trim().is_empty() => Some(v.trim()),
            _ => None,
        })
    }

    pub fn has_page_cover(&self) -> bool {
        matches!(self.0.get("cover"), Some(Value::Text(v)) if v.trim().eq_ignore_ascii_case("true"))
    }

    pub fn is_book(&self) -> bool {
        matches!(self.0.get("type"), Some(Value::Text(v)) if v.trim() == "book")
    }
}

pub fn parse(text: &str) -> Option<Fields> {
    let mut fields = HashMap::new();
    for line in body(text)?.split('\n') {
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
        fields.insert(key, value);
    }
    Some(Fields(fields))
}

pub fn set_field(text: &str, key: &str, value: &str) -> Option<String> {
    let end = end(text)?;
    let body = &text[4..end - 4];
    let drop: Vec<&str> = std::iter::once(key).chain(aliases(key).iter().copied()).collect();
    let mut found = false;
    let mut lines: Vec<String> = Vec::new();
    if !body.is_empty() {
        for line in body.split('\n') {
            if !drop.contains(&split_line(line).0.as_str()) {
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
    Some(format!("---\n{}\n---{}", lines.join("\n"), &text[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_block_like_the_frontend_regex() {
        assert_eq!(end("---\nkey: 1\n---\nтекст"), Some(14));
        assert_eq!(end("---\n\n---"), Some(8));
        assert_eq!(end("---\n---"), None);
        assert_eq!(end("текст\n---\nkey: 1\n---"), None);
        assert_eq!(end("---\r\nkey: 1\r\n---"), None);
        assert_eq!(hidden_len("---\nkey: 1\n---\nтекст"), 15);
        assert_eq!(hidden_len("---\nkey: 1\n---"), 14);
        assert_eq!(hidden_len("текст"), 0);
    }

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
        let next = set_field(&next, "cover", "").unwrap();
        assert!(!next.contains("cover: True"));
        assert_eq!(set_field("текст", "cover", "true"), None);
    }
}
