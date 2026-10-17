use crate::search::analyzer::frontmatter_body;
use crate::markdown::{fence, list};
use super::parse::{note_fields, scalar_or_list};
use super::value::Field;

pub fn page_fields(text: &str, first: bool) -> Vec<Field> {
    let (mut found, body) = match first {
        true => (note_fields(text), frontmatter_body(text)),
        false => (Vec::new(), text),
    };
    for (key, value) in inline_pairs(body) {
        if !found.iter().any(|field| field.key.eq_ignore_ascii_case(&key)) {
            found.push(scalar_or_list(key, &value));
        }
    }
    found
}

const MAX_KEY: usize = 64;

const SEPARATOR: &str = "::";

fn inline_pairs(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (_, line) in fence::outside(text).filter(|(_, line)| line.contains(SEPARATOR)) {
        let line = strip_code_spans(line);
        let content = strip_markers(&line);
        match split_pair(content) {
            Some(pair) => found.push(pair),
            None => found.extend(bracketed(content)),
        }
    }
    found
}

fn strip_code_spans(line: &str) -> String {
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
    kept
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

fn bracketed(content: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (open, close) in [('[', ']'), ('(', ')')] {
        let mut rest = content;
        while let Some(start) = rest.find(open) {
            let after = &rest[start + open.len_utf8()..];
            let Some(end) = after.find(close) else { break };
            found.extend(split_pair(&after[..end]));
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
    Some((key.to_owned(), value.trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{inline_pairs, page_fields};
    use crate::search::fields::value::FieldKind;

    fn pairs(text: &str) -> Vec<(String, String)> {
        inline_pairs(crate::search::analyzer::frontmatter_body(text))
    }

    #[test]
    fn only_a_literal_separator_outside_code_makes_a_field() {
        assert!(pairs("Обычный текст заметки\n> - пункт списка\nЕщё один абзац").is_empty());
        assert!(pairs("`Оценка:: 9` — код, а не поле").is_empty());
        assert!(pairs("Статус:`код`: готово").is_empty());
        assert_eq!(pairs("Автор:: Лем и `Value:: Text`"), vec![("Автор".to_owned(), "Лем и".to_owned())]);
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
        let found = page_fields(text, true);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].key, "Оценка");
        assert_eq!(found[0].text, "5", "шапка важнее строки в тексте");
        assert_eq!(found[1].key, "Автор");
    }

    #[test]
    fn an_inline_field_keeps_its_type() {
        let found = page_fields("Оценка:: 8\nТеги:: [книги, фантастика]", true);
        assert_eq!(found[0].kind, FieldKind::Number);
        assert_eq!(found[1].kind, FieldKind::List);
        assert_eq!(found[1].items, vec!["книги", "фантастика"]);
    }

    #[test]
    fn an_empty_key_or_a_giant_one_is_refused() {
        assert!(pairs(":: значение").is_empty());
        let giant = "к".repeat(70);
        assert!(pairs(&format!("{giant}:: значение")).is_empty());
    }
}
