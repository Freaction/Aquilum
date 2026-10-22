use super::locate;
use crate::search::hits::query_matcher;
use crate::search::pages::{Keys, Pages, Placed};
use crate::search::query::stems;
use crate::text::utf16::utf16_len;
use std::collections::HashSet;
use std::path::Path;

const WORD: &str = "щукаплывёт";

fn note(newline: &str) -> String {
    let mut text = String::new();
    for paragraph in 0..20_000 {
        text.push_str(&format!("Абзац {paragraph} про обычные вещи и ничего больше.{newline}{newline}"));
    }
    text.push_str(&format!("## Финал{newline}{newline}В самом конце {WORD} и точка.{newline}"));
    text
}

fn placed(path: &Path) -> (Vec<Placed>, HashSet<String>) {
    let (mut pages, note, mut keys) = (Pages::open(path).unwrap(), path.to_string_lossy().into_owned(), Keys::default());
    let (mut placed, mut hits) = (Vec::new(), HashSet::new());
    while let Some(page) = pages.next().unwrap() {
        let key = keys.next(&note, &page);
        if page.text.contains(WORD) {
            hits.insert(key.clone());
        }
        placed.push(Placed { key, start: page.start, start_utf16: page.start_utf16 });
    }
    (placed, hits)
}

#[test]
fn the_matching_page_is_read_directly_with_its_offset_and_section() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("Большая.md");
    let text = note("\n");
    std::fs::write(&path, &text).unwrap();
    let (pages, hits) = placed(&path);
    assert!(pages.len() > 2, "заметка из нескольких страниц, страниц {}", pages.len());

    let found = locate(&path, &pages, &hits, &query_matcher(&stems(WORD))).expect("прыжок к странице");
    assert_eq!(found.utf16, utf16_len(&text[..text.find(WORD).unwrap()]));
    assert_eq!(found.heading.as_deref(), Some("Финал"));
    assert!(found.snippet.contains(WORD));
}

#[test]
fn a_shifted_or_changed_file_is_not_trusted() {
    let directory = tempfile::tempdir().unwrap();
    let matcher = query_matcher(&stems(WORD));
    let crlf = directory.path().join("Windows.md");
    std::fs::write(&crlf, note("\r\n")).unwrap();
    let (pages, hits) = placed(&crlf);
    assert!(locate(&crlf, &pages, &hits, &matcher).is_none(), "сдвиги страниц считаются без \\r — прыгать нельзя");

    let changed = directory.path().join("Изменённая.md");
    std::fs::write(&changed, note("\n")).unwrap();
    let (pages, hits) = placed(&changed);
    std::fs::write(&changed, format!("Новая первая строка\n{}", note("\n"))).unwrap();
    assert!(locate(&changed, &pages, &hits, &matcher).is_none(), "файл изменился после индексации — страница не та");
}

