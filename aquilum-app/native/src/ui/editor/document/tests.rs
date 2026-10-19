use std::collections::HashSet;
use std::sync::Arc;

use masonry::core::StyleSet;
use masonry::parley::{Affinity, Alignment, Cursor, FontContext, LayoutContext};

use super::super::markdown::{self, Block, Fonts, Frame};
use super::{Document, Fold, text};

pub(super) fn markdown(text: &str) -> Document {
    let mut doc = Document::new(text);
    doc.set_markdown(Some(Fonts::new(16.0, true, true)));
    doc
}

fn blocks(doc: &Document) -> Vec<(Frame, Block)> {
    doc.paragraphs().map(|p| (*p.frame, p.block.clone())).collect()
}

struct Files;

impl super::super::resolve::Resolver for Files {
    fn dataview(&self, _: &str) -> (super::super::resolve::Fetch<super::super::resolve::Grid>, u64) {
        (super::super::resolve::Fetch::Pending, 0)
    }
    fn media(&self, source: &str, _: bool) -> (super::super::resolve::Fetch<super::super::resolve::Media>, u64) {
        match source {
            "есть.png" => (super::super::resolve::Fetch::Pending, 0),
            _ => (super::super::resolve::Fetch::Missing(source.to_owned()), 0),
        }
    }
    fn book(&self, _: &str) -> (super::super::resolve::Fetch<super::super::resolve::Book>, u64) {
        (super::super::resolve::Fetch::Pending, 0)
    }
    fn request(&self, _: super::super::resolve::Request) {}
}

#[test]
fn missing_media_stays_visible_as_text() {
    let mut doc = markdown("![](есть.png)\n![](нет.png)\n![[тоже нет.png|300]]");
    doc.resolver = Some(Arc::new(Files));
    doc.keep = (0.0, 10_000.0);
    let mut fcx = FontContext::default();
    let mut lcx = LayoutContext::new();
    doc.layout(&mut fcx, &mut lcx, &StyleSet::new(16.0), Some(600.0), None, None);
    let embedded: Vec<bool> = doc.paragraphs().map(|p| p.embed.is_some()).collect();
    assert_eq!(embedded, [true, false, false], "ненайденная картинка спряталась вместо того, чтобы показать строку");
    let shown: Vec<(f64, Fold)> = doc.paragraphs().enumerate().map(|(i, p)| (doc.bottom(i) - doc.top(i), p.fold)).collect();
    assert!(shown[1..].iter().all(|(h, f)| *h > 0.0 && *f == Fold::None), "{shown:?}");
}

#[test]
fn table_cells_use_editor_markdown() {
    let mut doc = markdown("| **жирный** | [[Заметка\\|алиас]] |\n| --- | --- |\n| `код` | #тег и [сайт](https://e.com) |");
    doc.keep = (0.0, 10_000.0);
    let mut fcx = FontContext::default();
    let mut lcx = LayoutContext::new();
    doc.layout(&mut fcx, &mut lcx, &StyleSet::new(16.0), Some(600.0), None, None);
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
    let text = "| слово другое третье |\n| --- |\n| x |";
    let mut doc = markdown(text);
    doc.keep = (0.0, 10_000.0);
    let mut fcx = FontContext::default();
    let mut lcx = LayoutContext::new();
    doc.layout(&mut fcx, &mut lcx, &StyleSet::new(16.0), Some(600.0), None, None);
    let (_, view) = doc.table_at(0).unwrap();
    let cell = view.cell(0, 0).unwrap();
    let at = |byte: usize| {
        let b = Cursor::from_byte_index(&cell.layout, byte, Affinity::Downstream).geometry(&cell.layout, 0.0);
        view.cell_origin(cell, 0.0) + masonry::kurbo::Vec2::new(b.x0 + 2.0, (b.y0 + b.y1) / 2.0)
    };
    let word = "слово ".len();
    let mut session = crate::ui::editor::table::Session::new(0);
    for count in 1..=3 {
        let _ = session.press(view, at(word + 2), false, count);
        let (anchor, caret) = (session.anchor.min(session.caret), session.anchor.max(session.caret));
        let expected = match count {
            1 => word + 2..word + 2,
            2 => word..word + "другое".len(),
            _ => 0.."слово другое третье".len(),
        };
        assert_eq!(anchor..caret, expected, "щелчков: {count}");
    }
}

const TYPED: &str = "![610](https://via.placeholder.com/300x120?text=Title \"Подсказка\")
https://example.com/auto — голая автоссылка, www.example.com и <mail@example.com>
[Ссылка `код`](https://example.com 'title') и [[Заметка#Раздел|текст]] и ![[image.png|300x200]]
**жирный *курсив* `код`** ~~зачёркнутый~~ ==выделение== **** `` `` ** ** _ _
# Заголовок с закрывающими #
> [!note]+ Выноска с **текстом**
- [x] задача #тег/вложенный и ^block-id
\t1. [ ] вложенная задача
---
$E = mc^2$ и $$x$$ и %% комментарий %%";

#[test]
fn typing_markdown_never_breaks_layout() {
    let mut fcx = FontContext::default();
    let mut lcx = LayoutContext::new();
    let styles = StyleSet::new(16.0);
    let fonts = Fonts { ch: 8.0, ..Fonts::new(16.0, true, true) };
    let unresolved = HashSet::new();
    let context = markdown::Context { fonts: &fonts, unresolved: &unresolved, resolver: None, inside: false, gutter: 0 };
    for line in TYPED.lines() {
        for end in line.char_indices().map(|(i, _)| i).chain([line.len()]) {
            let text = &line[..end];
            let (frame, block) = markdown::classify(std::iter::once(text)).remove(0);
            for caret in [None, Some(end..end)] {
                let _ = text::build(&mut fcx, &mut lcx, &styles, text, Some(end..end), Some(400.0), Alignment::Start, Some((&context, &frame, &block, caret)));
            }
        }
    }
}

#[test]
fn blank_line_turns_indented_text_into_code_and_back() {
    let mut text = "абзац\n    отступ\n    ещё\nконец".to_owned();
    let mut doc = markdown(&text);
    for (from, to) in [("абзац\n", "абзац\n\n"), ("абзац\n\n", "абзац\n"), ("    ещё", "\n    ещё")] {
        let at = text.find(from).unwrap();
        doc.replace(at..at + from.len(), to);
        text.replace_range(at..at + from.len(), to);
        assert_eq!(blocks(&doc), markdown::classify(text.split('\n')), "после правки «{from:?}» → «{to:?}»");
    }
}

#[test]
fn partial_classification_matches_a_full_pass() {
    let base = "# Заголовок\n\nтекст\n\n- пункт\n\n\t- вложенный\n\t  продолжение\n\n> цитата\n> [!note] выноска\n\n| а | б |\n| --- | --- |\n| 1 | 2 |\n\n```rust\nlet x = 1;\n```\n\nконец";
    let edits: [(&str, &str); 15] = [
        ("\n```rust", "| 5 | 6 |\n```rust"),
        ("| --- | --- |", "текст2"),
        ("текст2", "| --- | --- |"),
        ("текст", "```\nоткрыли"),
        ("открыли", "открыли\n```"),
        ("| 1 | 2 |", "| 1 | 2 |\n| 3 | 4 |"),
        ("\n| --- | --- |", ""),
        ("| а | б |", "| а | б |\n| --- | --- |"),
        ("- пункт", "1. пункт"),
        ("\n\n\t- вложенный", "\n\t- вложенный"),
        ("> цитата", "цитата"),
        ("```rust", "~~~rust"),
        ("~~~rust", "```rust"),
        ("let x = 1;\n```", "let x = 1;"),
        ("конец", "конец\n```"),
    ];
    let mut text = base.to_owned();
    let mut doc = markdown(&text);
    for (from, to) in edits {
        let at = text.find(from).unwrap_or_else(|| panic!("нет «{from}» в тексте"));
        doc.replace(at..at + from.len(), to);
        text.replace_range(at..at + from.len(), to);
        assert_eq!(doc.whole(), text);
        assert_eq!(blocks(&doc), markdown::classify(text.split('\n')), "после правки «{from}» → «{to}»");
    }
}

#[test]
fn frontmatter_lines_are_metadata_and_follow_the_block_boundary() {
    let mut doc = markdown("---\ntitle: **не жирный**\n# не заголовок\n- не пункт\n---\n# Заголовок\n- пункт");
    let leaves = |doc: &Document| -> Vec<Block> { doc.paragraphs().map(|p| p.block.clone()).collect() };
    let kinds = leaves(&doc);
    let framed = |kinds: &[Block]| kinds[0] == Block::Rule && kinds[4] == Block::Rule && kinds[1..4].iter().all(|b| *b == Block::Meta);
    assert!(framed(&kinds), "{kinds:?}");
    assert!(matches!(kinds[5], Block::Heading { .. }));
    assert!(matches!(kinds[6], Block::Item(_)));
    doc.replace(0..0, "x");
    assert!(leaves(&doc).iter().all(|b| *b != Block::Meta), "без открывающей черты блока метаданных нет");
    doc.replace(0..1, "");
    assert!(framed(&leaves(&doc)));
    let closing = doc.whole().rfind("\n---").unwrap();
    doc.replace(closing + 1..closing + 4, "--");
    assert!(leaves(&doc).iter().all(|b| *b != Block::Meta), "без закрывающей черты блока метаданных нет");
}

#[test]
fn indented_text_lines_keep_their_inline_markup() {
    let mut doc = markdown("текст\n    `код` и #тег\n\tещё `код`");
    doc.keep = (0.0, 10_000.0);
    doc.layout(&mut FontContext::default(), &mut LayoutContext::new(), &StyleSet::new(16.0), Some(600.0), None, None);
    assert_eq!((doc.paragraph(1).decor().codes.len(), doc.paragraph(1).decor().tags.len()), (1, 1), "{:?}", blocks(&doc));
    assert_eq!(doc.paragraph(2).decor().codes.len(), 1, "{:?}", blocks(&doc));
}

fn laid_out(text: &str, tags: bool) -> Document {
    let mut doc = Document::new(text);
    doc.set_markdown(Some(Fonts::new(16.0, true, tags)));
    doc.keep = (0.0, 10_000.0);
    let mut fcx = FontContext::default();
    let mut lcx = LayoutContext::new();
    doc.layout(&mut fcx, &mut lcx, &StyleSet::new(16.0), Some(600.0), None, None);
    doc
}

#[test]
fn chips_of_neighbouring_tags_do_not_touch_but_text_around_them_is_left_alone() {
    let text = "слово #один #два #три. И #четыре, конец";
    let doc = laid_out(text, true);
    let p = doc.paragraph(0);
    let layout = p.laid().expect("абзац свёрстан");
    let chips = markdown::tag_chips(layout, p.decor(), masonry::kurbo::Vec2::ZERO);
    assert_eq!(chips.len(), 4);
    let x = |at: usize| Cursor::from_byte_index(layout, at, Affinity::Downstream).geometry(layout, 0.0).x0;
    for pair in chips.windows(2) {
        assert!(pair[0].0.rect().x1 + 0.5 <= pair[1].0.rect().x0, "плашки тегов наезжают друг на друга: {:?} и {:?}", pair[0].0.rect(), pair[1].0.rect());
    }
    let plain = laid_out("слово #один #два #три. И #четыре, конец".replace('#', "").as_str(), true);
    let plain_layout = plain.paragraph(0).laid().expect("абзац свёрстан");
    let before_tags = Cursor::from_byte_index(plain_layout, "слово ".len(), Affinity::Downstream).geometry(plain_layout, 0.0).x0;
    assert!((x("слово ".len()) - before_tags).abs() < 0.5, "текст перед первым тегом сдвинут");
}

#[test]
fn tags_get_palette_colors_or_the_accent_when_colors_are_off() {
    let colored = laid_out("#один #два #один #Один", true);
    let slots: Vec<Option<usize>> = colored.paragraph(0).decor().tags.iter().map(|(_, slot)| *slot).collect();
    assert!(slots.iter().all(Option::is_some));
    assert_eq!((slots[0], slots[0]), (slots[2], slots[3]), "один и тот же тег получил разные цвета");
    let plain = laid_out("#один #два", false);
    assert!(plain.paragraph(0).decor().tags.iter().all(|(_, slot)| slot.is_none()), "при выключенных цветах теги не акцентные");
}
