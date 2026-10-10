use super::value::{Field, FieldKind};
use crate::search::analyzer::{frontmatter_unquote, frontmatter_yaml};
use std::iter::Peekable;
use std::str::Lines;

pub fn note_fields(text: &str) -> Vec<Field> {
    frontmatter_yaml(text)
        .map(note_fields_from_yaml)
        .unwrap_or_default()
}

fn note_fields_from_yaml(yaml: &str) -> Vec<Field> {
    let mut found: Vec<Field> = Vec::new();
    let mut lines = yaml.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(item) = list_item(trimmed) {
            if let Some(field) = found.last_mut() {
                field.push_item(item);
            }
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim();
        found.push(if is_block_marker(value) {
            Field::stored(
                key.to_owned(),
                FieldKind::Text,
                block_scalar(&mut lines),
                None,
            )
        } else if let Some(items) = inline_list(value) {
            Field::list(key.to_owned(), items)
        } else {
            yaml_scalar(key.to_owned(), value)
        });
    }
    found
}

pub fn scalar_or_list(key: String, value: &str) -> Field {
    match inline_list(value) {
        Some(items) => Field::list(key, items),
        None => yaml_scalar(key, value),
    }
}

fn yaml_scalar(key: String, raw: &str) -> Field {
    let mut field = Field::scalar(key, frontmatter_unquote(raw));
    if let Ok(value) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw) {
        field.kind = match value {
            serde_yaml_ng::Value::Bool(_) => FieldKind::Bool,
            serde_yaml_ng::Value::Number(_) => FieldKind::Number,
            _ => FieldKind::Text,
        };
    }
    field
}

fn list_item(trimmed: &str) -> Option<String> {
    if trimmed == "-" {
        return Some(String::new());
    }
    trimmed
        .strip_prefix("- ")
        .map(|value| frontmatter_unquote(value.trim()))
}

fn is_block_marker(value: &str) -> bool {
    matches!(value, "|" | ">" | "|-" | ">-" | "|+" | ">+")
}

fn block_scalar(lines: &mut Peekable<Lines<'_>>) -> String {
    let mut collected = Vec::new();
    while let Some(line) = lines.peek() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !line.starts_with([' ', '\t']) {
            break;
        }
        if !trimmed.is_empty() {
            collected.push(trimmed.to_owned());
        }
        lines.next();
    }
    collected.join("\n")
}

fn inline_list(value: &str) -> Option<Vec<String>> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        inner
            .split(',')
            .map(|item| frontmatter_unquote(item.trim()))
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::note_fields;
    use crate::search::fields::value::{Field, FieldKind};

    const BOOK: &str = "---\ncreated: 31-08-2026 | (19:31)\ncover: true\ntype: book\nauthor: Иван Ефремов\nПуть к файлу: \n---\nТело заметки.\n";

    fn field(text: &str, key: &str) -> Field {
        note_fields(text)
            .into_iter()
            .find(|field| field.key == key)
            .unwrap_or_else(|| panic!("нет поля «{key}»"))
    }

    #[test]
    fn reads_every_field_in_file_order() {
        let found = note_fields(BOOK);
        assert_eq!(found.len(), 5);
        assert_eq!(found[0].key, "created");
        assert_eq!(
            found[0].text, "31-08-2026 | (19:31)",
            "двоеточие и вертикальная черта внутри значения не обрезают его"
        );
        assert_eq!(found[3].text, "Иван Ефремов");
        assert_eq!(found[4].key, "Путь к файлу");
        assert_eq!(found[4].text, "");
    }

    #[test]
    fn note_without_frontmatter_has_no_fields() {
        assert!(note_fields("просто текст").is_empty());
    }

    #[test]
    fn scalars_carry_their_type() {
        assert_eq!(field(BOOK, "cover").kind, FieldKind::Bool);
        assert_eq!(field(BOOK, "author").kind, FieldKind::Text);
        assert_eq!(field(BOOK, "Путь к файлу").kind, FieldKind::Text);
        let note = "---\nrating: 4.5\npages: 0/0\n---\n";
        assert_eq!(field(note, "rating").kind, FieldKind::Number);
        assert_eq!(
            field(note, "pages").kind,
            FieldKind::Text,
            "«0/0» — не число"
        );
    }

    #[test]
    fn quoted_numeric_boolean_and_block_strings_keep_text_kind() {
        let text = "---\nrank: \"2\"\ndone: 'true'\nnumber: 2\nboolean: true\nblock: |\n  2\n---\n";
        assert_eq!(field(text, "rank").kind, FieldKind::Text);
        assert_eq!(field(text, "done").kind, FieldKind::Text);
        assert_eq!(field(text, "block").kind, FieldKind::Text);
        assert_eq!(field(text, "number").kind, FieldKind::Number);
        assert_eq!(field(text, "boolean").kind, FieldKind::Bool);
    }

    #[test]
    fn a_date_like_value_is_not_mistaken_for_a_number() {
        let note = "---\ncreated: 31-08-2026\n---\n";
        assert_eq!(field(note, "created").kind, FieldKind::Text);
        assert_eq!(field(note, "created").text, "31-08-2026");
    }

    #[test]
    fn inline_list_becomes_a_list_of_items() {
        let note = "---\ntags: [fiction, classic]\n---\n";
        let tags = field(note, "tags");
        assert_eq!(tags.kind, FieldKind::List);
        assert_eq!(tags.items, vec!["fiction", "classic"]);
    }

    #[test]
    fn an_empty_inline_list_is_an_empty_list_not_the_text_brackets() {
        let tags = field("---\ntags: []\n---\n", "tags");
        assert_eq!(tags.kind, FieldKind::List);
        assert!(tags.items.is_empty());
        assert_eq!(
            tags.text, "",
            "пустой список не считается заполненным полем"
        );
    }

    #[test]
    fn block_list_items_attach_to_the_key_above_them() {
        let note = "---\ntags:\n  - fiction\n  - \"classic sci-fi\"\nauthor: Ефремов\n---\n";
        let tags = field(note, "tags");
        assert_eq!(tags.kind, FieldKind::List);
        assert_eq!(tags.items, vec!["fiction", "classic sci-fi"]);
        assert_eq!(
            field(note, "author").text,
            "Ефремов",
            "ключ после списка читается как обычно"
        );
    }

    #[test]
    fn a_list_item_never_overwrites_a_filled_scalar() {
        let note = "---\nauthor: Ефремов\n- лишняя строка\n---\n";
        let author = field(note, "author");
        assert_eq!(author.kind, FieldKind::Text);
        assert_eq!(author.text, "Ефремов");
    }

    #[test]
    fn block_scalar_lines_do_not_leak_into_new_keys() {
        let note = "---\ndescription: |\n  первая строка\n  key: не поле\nauthor: Ефремов\n---\n";
        let found = note_fields(note);
        assert_eq!(
            found.len(),
            2,
            "внутри блочного значения ключей нет: {found:?}"
        );
        assert_eq!(
            field(note, "description").text,
            "первая строка\nkey: не поле"
        );
        assert_eq!(field(note, "author").text, "Ефремов");
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let note = "---\n# комментарий\n\ntype: book\n---\n";
        let found = note_fields(note);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "type");
    }

    #[test]
    fn quotes_are_stripped_from_values() {
        assert_eq!(
            field("---\nauthor: \"О. Генри\"\n---\n", "author").text,
            "О. Генри"
        );
        assert_eq!(
            field("---\nauthor: 'О. Генри'\n---\n", "author").text,
            "О. Генри"
        );
    }

    #[test]
    fn a_url_value_keeps_its_colon() {
        let note = "---\nsource: https://example.com/page\n---\n";
        assert_eq!(field(note, "source").text, "https://example.com/page");
    }
}
