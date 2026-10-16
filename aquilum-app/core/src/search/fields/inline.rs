use crate::search::analyzer::frontmatter_body;
use crate::markdown::{fence, list};
use super::parse::{note_fields, scalar_or_list};
use super::value::Field;
use std::borrow::Cow;

pub fn indexed_fields(text: &str) -> Vec<Field> {
    let mut found = note_fields(text);
    for (key, value) in inline_pairs(text) {
        if found.iter().any(|field| field.key.eq_ignore_ascii_case(&key)) {
            continue;
        }
        found.push(scalar_or_list(key, &value));
    }
    found
}

const MAX_KEY: usize = 64;

const SEPARATOR: &str = "::";

fn inline_pairs(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (_, line) in fence::outside(frontmatter_body(text)) {
        // Backtick removal can join two ':' characters into a separator.
        // Keep parsing such lines even when the raw text has no "::".
        if !line.contains(SEPARATOR) && !line.contains('`') {
            continue;
        }
        let line = strip_code_spans(line);
        if !line.contains(SEPARATOR) {
            continue;
        }
        let content = strip_markers(&line);
        if let Some(pair) = whole_line(content) {
            found.push(pair);
            continue;
        }
        found.extend(bracketed(content));
    }
    found
}

fn strip_code_spans(line: &str) -> Cow<'_, str> {
    if !line.contains('`') {
        return Cow::Borrowed(line);
    }
    let mut kept = String::with_capacity(line.len());
    let mut inside = false;
    for symbol in line.chars() {
        if symbol == '`' {
            inside = !inside;
            continue;
        }
        if !inside {
            kept.push(symbol);
        }
    }
    Cow::Owned(kept)
}

fn strip_markers(line: &str) -> &str {
    let mut rest = line.trim_start();
    loop {
        let before = rest;
        rest = rest.trim_start_matches(['>', ' ', '\t']);
        if let Some(item) = list::item(rest) {
            rest = rest[item.content..].trim_start();
        }
        if rest == before {
            return rest;
        }
    }
}

fn whole_line(content: &str) -> Option<(String, String)> {
    let (key, value) = split_pair(content)?;
    Some((key, value.trim().to_owned()))
}

fn bracketed(content: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (open, close) in [('[', ']'), ('(', ')')] {
        let mut rest = content;
        while let Some(start) = rest.find(open) {
            let after = &rest[start + open.len_utf8()..];
            let Some(end) = after.find(close) else { break };
            if let Some((key, value)) = split_pair(&after[..end]) {
                found.push((key, value.trim().to_owned()));
            }
            rest = &after[end + close.len_utf8()..];
        }
    }
    found
}

fn split_pair(content: &str) -> Option<(String, String)> {
    let position = content.find(SEPARATOR)?;
    let key = content[..position].trim();
    let value = &content[position + SEPARATOR.len()..];
    if !value.is_empty() && !value.starts_with([' ', '\t']) {
        return None;
    }
    if key.is_empty() || key.chars().count() > MAX_KEY {
        return None;
    }
    if key.contains(SEPARATOR) || key.contains(['[', ']', '(', ')', '|', '#', ':']) {
        return None;
    }
    Some((key.to_owned(), value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{indexed_fields, inline_pairs};
    use crate::search::fields::value::FieldKind;

    fn pairs(text: &str) -> Vec<(String, String)> {
        inline_pairs(text)
    }

    #[test]
    fn a_whole_line_becomes_a_field() {
        assert_eq!(
            pairs("Оценка:: 8\nпростой текст"),
            vec![("Оценка".to_owned(), "8".to_owned())]
        );
    }

    #[test]
    fn a_field_works_inside_a_list_item_and_a_quote() {
        assert_eq!(
            pairs("- Автор:: Лем\n> Год:: 1961\n2. Жанр:: фантастика"),
            vec![
                ("Автор".to_owned(), "Лем".to_owned()),
                ("Год".to_owned(), "1961".to_owned()),
                ("Жанр".to_owned(), "фантастика".to_owned()),
            ]
        );
    }

    #[test]
    fn a_field_in_any_kind_of_list_item_is_read() {
        assert_eq!(
            pairs("- [x] Статус:: готово\n1) Автор:: Лем\n* Год:: 1961\n+ [ ] Жанр:: фантастика"),
            vec![
                ("Статус".to_owned(), "готово".to_owned()),
                ("Автор".to_owned(), "Лем".to_owned()),
                ("Год".to_owned(), "1961".to_owned()),
                ("Жанр".to_owned(), "фантастика".to_owned()),
            ]
        );
    }

    #[test]
    fn a_bracketed_field_is_read_from_the_middle_of_a_sentence() {
        assert_eq!(
            pairs("Прочитал за вечер [Оценка:: 9], понравилось"),
            vec![("Оценка".to_owned(), "9".to_owned())]
        );
    }

    #[test]
    fn a_rust_path_is_not_a_field() {
        assert!(pairs("note_date::parse читает обе формы").is_empty());
        assert!(pairs("`Value::Text` — это значение").is_empty());
    }

    #[test]
    fn a_code_block_holds_no_fields() {
        let text = "```rust\nОценка:: 8\nlet x = Value::Text;\n```\nОбычный текст";
        assert!(pairs(text).is_empty());
    }

    #[test]
    fn frontmatter_is_not_read_twice_and_wins_over_the_body() {
        let text = "---\nОценка: 5\n---\nОценка:: 9\nАвтор:: Лем";
        let found = indexed_fields(text);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].key, "Оценка");
        assert_eq!(found[0].text, "5", "шапка важнее строки в тексте");
        assert_eq!(found[1].key, "Автор");
    }

    #[test]
    fn an_inline_field_keeps_its_type() {
        let found = indexed_fields("Оценка:: 8\nТеги:: [книги, фантастика]");
        assert_eq!(found[0].kind, FieldKind::Number);
        assert_eq!(found[1].kind, FieldKind::List);
        assert_eq!(found[1].items, vec!["книги", "фантастика"]);
    }

    #[test]
    fn prose_without_inline_delimiters_is_ignored() {
        assert!(pairs("Обычный текст заметки\\n> - пункт списка\\nЕще один абзац").is_empty());
        assert!(pairs("A note with `inline code` but no metadata").is_empty());
    }

    #[test]
    fn delimiters_inside_code_spans_are_not_indexed() {
        assert!(pairs("`Оценка:: 9` is source code, not metadata").is_empty());
        assert_eq!(
            pairs("Автор:: Лем and `Value:: Text`"),
            vec![("Автор".to_owned(), "Лем and ".to_owned())]
        );
    }

    #[test]
    fn removing_a_code_span_can_join_separated_colons() {
        // Preserve the previous parser's behavior even if "::" is not
        // present verbatim in the Markdown input.
        assert_eq!(
            pairs("Статус:`ignored`: готово"),
            vec![("Статус".to_owned(), "готово".to_owned())]
        );
    }

    #[test]
    fn code_span_stripping_borrows_unmodified_lines() {
        use std::borrow::Cow;
        assert!(matches!(super::strip_code_spans("Автор:: Лем"), Cow::Borrowed(_)));
        assert!(matches!(super::strip_code_spans("Автор:: `code`Лем"), Cow::Owned(_)));
    }

    #[test]
    fn an_empty_key_or_a_giant_one_is_refused() {
        assert!(pairs(":: значение").is_empty());
        let giant = "к".repeat(70);
        assert!(pairs(&format!("{giant}:: значение")).is_empty());
    }
}
