use super::super::blocks::{Callout, heading};
use super::{Block, blocks, frames};
use crate::ui::editor::highlight::Lang;

#[test]
fn classifies_lines() {
    let text = "# Заголовок\n- пункт\n\t- вложенный\n\t\t\t- глубже\n1. раз\n> цитата\n---\n```rust\nlet x;\n```\nтекст";
    let b = blocks(text);
    assert_eq!(b[0], Block::Heading { level: 1, marker: 2 });
    assert!(matches!(b[1], Block::Item(i) if i.level() == 0 && !i.ordered));
    assert!(matches!(b[2], Block::Item(i) if i.level() == 1 && i.indent() == 1));
    assert!(matches!(b[3], Block::Item(i) if i.level() == 3 && i.indent() == 3));
    assert!(matches!(b[4], Block::Item(i) if i.ordered && i.marker() == (0..3)));
    assert_eq!((&b[5], frames(text)[5].quote, frames(text)[5].lead), (&Block::Text, 1, 2));
    assert_eq!(b[6], Block::Rule);
    assert!(matches!(b[7], Block::Code { first: true, fence: true, lang: Some(_), .. }));
    assert!(matches!(b[8], Block::Code { first: false, last: false, fence: false, .. }));
    assert!(matches!(b[9], Block::Code { last: true, fence: true, .. }));
    assert_eq!(b[10], Block::Text);
}

#[test]
fn indented_code_outside_lists() {
    let b = blocks("Блок:\n\n    function x() {\n\t    return 1;\n    }\n\nтекст\n    продолжение абзаца\n\n    - пункт с отступом\n- пункт\n\n    дальше в пункте");
    assert!(matches!(b[2], Block::Code { first: true, last: false, fence: false, lang: None, .. }));
    assert!(matches!(b[3], Block::Code { first: false, last: false, .. }));
    assert!(matches!(b[4], Block::Code { first: false, last: true, .. }));
    assert_eq!(b[7], Block::Text, "отступ внутри абзаца стал кодом");
    assert!(!matches!(b[9], Block::Code { .. }), "пункт списка с отступом стал кодом");
    assert!(!matches!(b[12], Block::Code { .. }), "продолжение пункта стало кодом");
}

#[test]
fn nested_quote_markers_are_one_prefix() {
    let text = "> > Второй уровень\n> > > Третий\n>> плотно\n> ### Заголовок в цитате #";
    let quotes: Vec<_> = frames(text).iter().map(|f| (f.quote, f.lead)).collect();
    assert_eq!(quotes, [(2, 4), (3, 6), (2, 3), (1, 2)]);
    assert_eq!(blocks(text)[3], Block::Heading { level: 3, marker: 4 });
    assert_eq!(heading("### Заголовок в цитате #"), Some(Block::Heading { level: 3, marker: 4 }));
}

#[test]
fn list_items_inside_quotes_keep_their_marker_and_box_after_the_quote_prefix() {
    let b = blocks("> - [x] задача\n> > 1. пункт\n> текст");
    let task = b[0].item().expect("пункт в цитате");
    assert_eq!((task.marker(), task.task()), (2..4, Some((4..7, true))));
    let ordered = b[1].item().expect("пункт во вложенной цитате");
    assert!(ordered.ordered && ordered.marker() == (4..7));
    assert_eq!(b[2].item(), None);
}

#[test]
fn a_fence_inside_a_quote_is_code_until_the_quote_ends() {
    let text = "> ```js\n> > не цитата\n>\nтекст\n> ```\n> код";
    let b = blocks(text);
    assert!(matches!(b[0], Block::Code { first: true, fence: true, lang: Some(_), .. }));
    assert!(matches!(b[1], Block::Code { fence: false, .. }));
    assert!(matches!(b[2], Block::Code { last: true, .. }));
    assert_eq!(frames(text)[..3].iter().map(|f| (f.quote, f.lead)).collect::<Vec<_>>(), [(1, 2), (1, 2), (1, 1)]);
    assert_eq!(b[3], Block::Text);
    assert!(matches!(b[4], Block::Code { first: true, .. }));
    assert!(matches!(b[5], Block::Code { last: true, fence: false, .. }));
}

#[test]
fn unclosed_fence_ends_at_document_end() {
    let b = blocks("```\nкод\nещё");
    assert!(matches!(b[2], Block::Code { first: false, last: true, fence: false, .. }));
}

#[test]
fn quote_callouts() {
    let b = blocks("> [!quote] текст\n> дальше\n\n> [!book] [[Книга]]");
    assert_eq!(b[0], Block::Callout { range: 2..11, kind: Callout::Quote });
    assert_eq!(b[1], Block::Text);
    assert!(matches!(b[3], Block::Callout { kind: Callout::Book, .. }));
}

#[test]
fn media_lines_follow_tauri_rules() {
    let b = blocks("![](a.png)
![[кадр.mp4|300]]
![[Заметка]]
![a](x.png) ![b](y.png)
![](<с пробелом.png>)
![[a[b].png]]
![](f(1).png)");
    assert_eq!(b[0], Block::Image);
    assert_eq!(b[1], Block::Image);
    assert_ne!(b[2], Block::Image, "вики-вставка без расширения медиа — не картинка");
    assert_ne!(b[3], Block::Image, "две картинки в строке — не блок картинки");
    assert_eq!(b[4], Block::Image);
    assert_ne!(b[5], Block::Image);
    assert_ne!(b[6], Block::Image);
}

#[test]
fn setext_headings_and_their_underline() {
    let b = blocks("Заголовок\n=========\n\nВторой\n---\n\nтекст\n---\n- пункт\n---");
    assert_eq!(b[0], Block::Heading { level: 1, marker: 0 });
    assert_eq!(b[1], Block::Underline);
    assert_eq!(b[3], Block::Heading { level: 2, marker: 0 });
    assert_eq!(b[4], Block::Underline);
    assert_eq!(b[6], Block::Heading { level: 2, marker: 0 });
    assert_eq!(b[7], Block::Underline);
    assert!(matches!(b[8], Block::Item(_)));
    assert_eq!(b[9], Block::Rule, "линия после пункта списка остаётся линией");
}

#[test]
fn definitions_follow_a_term() {
    let b = blocks("Термин\n: Первое\n: Второе\n\n: без термина");
    assert_eq!(b[0], Block::Text);
    assert_eq!(b[1], Block::Definition { marker: 2 });
    assert_eq!(b[2], Block::Definition { marker: 2 });
    assert_eq!(b[4], Block::Text);
}

#[test]
fn html_comments_are_muted_blocks_even_across_lines() {
    let b = blocks("до\n<!-- один -->\n<!--\nмного\n```\n$$\n-->\nпосле\n- [ ] пункт");
    assert_eq!(b[0], Block::Text);
    assert_eq!(b[1], Block::Comment);
    assert!(b[2..=6].iter().all(|b| *b == Block::Comment), "{b:?}");
    assert_eq!(b[7], Block::Text);
    assert!(matches!(b[8], Block::Item(_)));
}

#[test]
fn display_math_is_kept_as_source_between_dollar_lines() {
    let b = blocks("$$\n\\int_0^1 x\\,dx\n$$\nтекст");
    assert!(matches!(b[0], Block::Code { first: true, fence: true, lang: Some(Lang::Math), .. }));
    assert!(matches!(b[1], Block::Code { first: false, last: false, fence: false, .. }));
    assert!(matches!(b[2], Block::Code { last: true, fence: true, .. }));
    assert_eq!(b[3], Block::Text);
}
