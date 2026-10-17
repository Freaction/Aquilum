use super::strip_frontmatter;

fn set(text: &str, updates: &[(String, String)]) -> Result<String, String> {
    let updates = updates.iter().map(|(key, value)| (key.as_str(), Some(value.as_str()))).collect::<Vec<_>>();
    Ok(match super::set(text, &updates, super::same_key)? {
        Some((range, insert)) => format!("{}{insert}{}", &text[..range.start], &text[range.end..]),
        None => text.to_owned(),
    })
}
use crate::search::fields::note_fields;

fn keys(text: &str) -> Vec<String> {
    note_fields(text)
        .into_iter()
        .map(|field| field.key)
        .collect()
}

fn value_of(text: &str, key: &str) -> String {
    note_fields(text)
        .into_iter()
        .find(|field| field.key == key)
        .map(|field| field.text)
        .unwrap_or_default()
}

const BOOK: &str = "---\ncreated: 31-08-2026 | (19:31)\ncover: true\ntype: book\nauthor: Иван Ефремов\nПуть к файлу: \n---\nТело заметки.\n";

fn update(key: &str, value: &str) -> Vec<(String, String)> {
    vec![(key.to_owned(), value.to_owned())]
}

#[test]
fn rewrites_a_field_in_place_and_leaves_the_body_alone() {
    let updated = set(BOOK, &update("author", "Рэй Брэдбери")).unwrap();
    assert!(updated.contains("author: Рэй Брэдбери"));
    assert!(!updated.contains("Иван Ефремов"));
    assert!(
        updated.ends_with("---\nТело заметки.\n"),
        "тело заметки не тронуто: {updated}"
    );
    assert_eq!(keys(&updated).len(), 5, "новых ключей не появилось");
}

#[test]
fn appends_an_unknown_field_after_the_existing_ones() {
    let updated = set(BOOK, &update("rating", "5")).unwrap();
    let found = keys(&updated);
    assert_eq!(found.first().unwrap(), "created", "порядок прежних ключей сохранён");
    assert_eq!(found.last().unwrap(), "rating");
}

#[test]
fn finds_the_key_regardless_of_case_and_keeps_its_original_spelling() {
    let updated = set(BOOK, &update("ПУТЬ К ФАЙЛУ", "Files/книга.epub")).unwrap();
    assert!(
        updated.contains("Путь к файлу: Files/книга.epub"),
        "написание ключа осталось прежним: {updated}"
    );
    assert_eq!(keys(&updated).len(), 5, "дубликат ключа не создан");
}

#[test]
fn creates_the_block_for_a_note_that_had_none() {
    let updated = set("Только текст.\n", &update("type", "book")).unwrap();
    assert_eq!(updated, "---\ntype: book\n---\nТолько текст.\n");
}

#[test]
fn refuses_to_touch_an_unclosed_block() {
    let broken = "---\ntype: book\nтело без закрывающего разделителя\n";
    assert!(
        set(broken, &update("type", "note")).is_err(),
        "второй блок поверх сломанного не создаётся"
    );
}

#[test]
fn keeps_neighbouring_values_with_colons_untouched() {
    let note = "---\nsource: https://example.com/page\n---\nтекст";
    assert_eq!(value_of(note, "source"), "https://example.com/page");
    let updated = set(note, &update("author", "О. Генри")).unwrap();
    assert!(updated.contains("source: https://example.com/page"));
    assert!(updated.contains("author: О. Генри"));
}

const SAMPLE: &str = r#"---
type: book
author: Isaac Asimov
status: to-read
pages: 0/0
tags: []
rating: 0
cover_url: https://cdn.example.com/cover.jpg
page_cover_url:
---
# Foundation

Actual note body."#;

#[test]
fn keeps_only_author_from_frontmatter() {
    let cleaned = strip_frontmatter(SAMPLE);
    assert!(cleaned.contains("Isaac Asimov"));
    assert!(cleaned.contains("Actual note body"));
    assert!(!cleaned.contains("type:"));
    assert!(!cleaned.contains("to-read"));
    assert!(!cleaned.contains("cover_url"));
    assert!(!cleaned.contains("tags:"));
}

#[test]
fn notes_without_frontmatter_pass_through() {
    let text = "plain note without yaml";
    assert_eq!(strip_frontmatter(text), text);
}

#[test]
fn empty_author_yields_body_only() {
    let text = "---\ntype: book\nauthor:\nstatus: x\n---\nbody";
    let cleaned = strip_frontmatter(text);
    assert_eq!(cleaned, "body");
    assert!(!cleaned.contains("type"));
}
