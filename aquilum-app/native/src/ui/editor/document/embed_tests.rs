use std::sync::Arc;

use masonry::parley::{Affinity, Cursor};

use super::super::resolve::{Book, Fetch, Grid, Media, Request, Resolver};
use super::tests::{laid, markdown};
use super::Fold;

struct Files;

impl Resolver for Files {
    fn dataview(&self, _: &str) -> (Fetch<Grid>, u64) {
        (Fetch::Pending, 0)
    }
    fn media(&self, source: &str, _: bool) -> (Fetch<Media>, u64) {
        match source {
            "есть.png" => (Fetch::Pending, 0),
            _ => (Fetch::Missing(source.to_owned()), 0),
        }
    }
    fn book(&self, _: &str) -> (Fetch<Book>, u64) {
        (Fetch::Pending, 0)
    }
    fn request(&self, _: Request) {}
}

#[test]
fn missing_media_stays_visible_as_text() {
    let mut doc = markdown("![](есть.png)\n![](нет.png)\n![[тоже нет.png|300]]");
    doc.resolver = Some(Arc::new(Files));
    let doc = laid(doc);
    let embedded: Vec<bool> = doc.paragraphs().map(|p| p.embed.is_some()).collect();
    assert_eq!(embedded, [true, false, false], "ненайденная картинка спряталась вместо того, чтобы показать строку");
    let shown: Vec<(f64, Fold)> = doc.paragraphs().enumerate().map(|(i, p)| (doc.bottom(i) - doc.top(i), p.fold)).collect();
    assert!(shown[1..].iter().all(|(h, f)| *h > 0.0 && *f == Fold::None), "{shown:?}");
}

#[test]
fn table_cells_use_editor_markdown() {
    let doc = laid(markdown("| **жирный** | [[Заметка\\|алиас]] |\n| --- | --- |\n| `код` | #тег и [сайт](https://e.com) |"));
    let (_, view) = doc.table_at(0).expect("таблица свёрстана");
    let cell = |r, c| view.cell(r, c).expect("ячейка");
    let link = &cell(0, 1).decor.links[0];
    assert_eq!((link.target.as_str(), link.wiki, &cell(0, 1).text[link.label.clone()]), ("Заметка", true, "алиас"));
    assert_eq!(cell(1, 0).decor.codes.len(), 1, "код в ячейке не оформлен");
    assert_eq!(cell(1, 1).decor.tags.len(), 1, "тег в ячейке не оформлен");
    assert_eq!(cell(1, 1).decor.links[0].target, "https://e.com");
    let layout = &cell(0, 1).layout;
    let b = Cursor::from_byte_index(layout, link.label.start + 2, Affinity::Downstream).geometry(layout, 0.0);
    assert_eq!(cell(0, 1).decor.link_at(layout, b.x0 + 1.0, (b.y0 + b.y1) / 2.0).map(|l| l.target.as_str()), Some("Заметка"), "щелчок по псевдониму не находит ссылку");
}

#[test]
fn clicks_in_a_table_cell_place_caret_word_and_cell() {
    let doc = laid(markdown("| слово другое третье |\n| --- |\n| x |"));
    let (_, view) = doc.table_at(0).unwrap();
    let cell = view.cell(0, 0).unwrap();
    let at = |byte: usize| {
        let b = Cursor::from_byte_index(&cell.layout, byte, Affinity::Downstream).geometry(&cell.layout, 0.0);
        view.cell_origin(cell, 0.0) + masonry::kurbo::Vec2::new(b.x0 + 2.0, (b.y0 + b.y1) / 2.0)
    };
    let word = "слово ".len();
    let mut session = crate::ui::editor::table::Session::new(0);
    for count in 1..=3 {
        let _ = session.press(view, at(word + 2), false, count, crate::ui::keymap::gesture::GRIP);
        let (anchor, caret) = (session.anchor.min(session.caret), session.anchor.max(session.caret));
        let expected = match count {
            1 => word + 2..word + 2,
            2 => word..word + "другое".len(),
            _ => 0.."слово другое третье".len(),
        };
        assert_eq!(anchor..caret, expected, "щелчков: {count}");
    }
}
