use std::collections::HashSet;

use masonry::core::StyleSet;
use masonry::parley::{Alignment, FontContext, LayoutContext};

use super::super::markdown::{self, Block, Fonts, Frame};
use super::{Document, text};

pub(super) fn markdown(text: &str) -> Document {
    let mut doc = Document::new(text);
    doc.set_markdown(Some(Fonts::new(16.0, true, true)));
    doc
}

pub(super) fn laid(mut doc: Document) -> Document {
    doc.keep = (0.0, 10_000.0);
    doc.layout(&mut FontContext::default(), &mut LayoutContext::new(), &StyleSet::new(16.0), Some(600.0), None, None);
    doc
}

pub(super) fn blocks(doc: &Document) -> Vec<(Frame, Block)> {
    doc.paragraphs().map(|p| (*p.frame, p.block.clone())).collect()
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
