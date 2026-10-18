use super::super::blocks::Nest;
use super::{Block, blocks, frames};

fn levels(text: &str) -> Vec<Option<usize>> {
    blocks(text).iter().map(|b| if let Block::Item(i) = b { Some(i.level()) } else { None }).collect()
}

fn owners(text: &str) -> Vec<Option<usize>> {
    frames(text).iter().map(|f| f.list.map(|nest| nest.level)).collect()
}

#[test]
fn nesting_follows_indent_columns_not_the_characters() {
    assert_eq!(levels("- a\n  - b\n    - c\n\t- d\n- e"), [Some(0), Some(1), Some(2), Some(2), Some(0)]);
    assert_eq!(levels("- a\n\t- b\n    - c\n\t\t\t- d\n        - e\n    - f"), [Some(0), Some(1), Some(1), Some(3), Some(2), Some(1)]);
    assert_eq!(levels("* a\n+ b\n- c\n1. d\n2) e"), [Some(0); 5]);
    assert_eq!(levels("\t\t\t- сирота\n- пункт"), [None, Some(0)]);
}

#[test]
fn list_content_stays_in_its_item() {
    let text = "1. Пункт\n\n   Абзац\n\n   ```bash\n   echo\n   ```\n   после кода\n2. Второй";
    let (b, f) = (blocks(text), frames(text));
    assert_eq!((&b[2], f[2].list, f[2].lead), (&Block::Text, Some(Nest { level: 0, width: 3 }), 3));
    assert!(matches!(b[4], Block::Code { first: true, fence: true, lang: Some(_), .. }));
    assert_eq!((f[4].list, f[4].lead), (Some(Nest { level: 0, width: 3 }), 3));
    assert!(matches!(b[5], Block::Code { first: false, fence: false, .. }));
    assert!(matches!(b[6], Block::Code { last: true, fence: true, .. }));
    assert_eq!((&b[7], owners(text)[7]), (&Block::Text, Some(0)));
    assert!(matches!(b[8], Block::Item(i) if i.ordered && i.level() == 0));
}

#[test]
fn a_line_without_a_marker_or_an_indent_closes_the_list() {
    let text = "- пункт\nабзац\n- новый список\n\tвложенный текст пункта";
    let b = blocks(text);
    assert_eq!((&b[1], owners(text)[1]), (&Block::Text, None));
    assert!(matches!(b[2], Block::Item(i) if i.level() == 0));
    assert_eq!((&b[3], owners(text)[3]), (&Block::Text, Some(0)));
}

#[test]
fn every_tab_moves_an_item_one_level_deeper() {
    assert_eq!(levels("1. a\n\t\t1. b\n\t\t\t\t- c\n\t1. d\n2. e"), [Some(0), Some(2), Some(4), Some(1), Some(0)]);
    assert_eq!(levels("- a\n\t\t- b\n- c"), [Some(0), Some(2), Some(0)]);
}

#[test]
fn a_continuation_goes_to_the_item_it_is_indented_past() {
    let text = "- a\n  - b\n    текст b\n  текст a\n  - c";
    assert_eq!(owners(text)[2..4], [Some(1), Some(0)]);
    assert!(matches!(blocks(text)[4], Block::Item(i) if i.level() == 1));
}

#[test]
fn task_items_are_found() {
    let b = blocks("- [ ] дело\n- [x] сделано\n1. [ ] номер");
    assert!(matches!(b[0], Block::Item(i) if i.task() == Some((2..5, false)) && i.content() == 6));
    assert!(matches!(b[1], Block::Item(i) if matches!(i.task(), Some((_, true)))));
    assert!(matches!(b[2], Block::Item(i) if i.task().is_some()));
}

#[test]
fn list_callouts_and_continuations() {
    let text = "- $ деньги\n\t2) пункт\n\t   продолжение\n\t1) дальше";
    let b = blocks(text);
    assert!(matches!(b[0], Block::Item(i) if i.callout() == Some((2, 5))));
    assert_eq!((frames(text)[2].lead, frames(text)[2].list.map(|nest| nest.width)), (4, Some(3)));
    assert!(matches!(b[3], Block::Item(i) if i.ordered));
}
