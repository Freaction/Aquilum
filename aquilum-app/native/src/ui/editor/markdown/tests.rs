use super::blocks::{Block, Callout, classify};
use super::inline::{Kind, hashtags, inline};

fn blocks(text: &str) -> Vec<Block> {
    classify(text.split('\n'))
}

#[test]
fn classifies_lines() {
    let b = blocks("# Заголовок\n- пункт\n\t- вложенный\n\t\t\t- сирота\n1. раз\n> цитата\n---\n```rust\nlet x;\n```\nтекст");
    assert_eq!(b[0], Block::Heading { level: 1, marker: 2 });
    assert!(matches!(b[1], Block::Item { level: 0, ordered: false, .. }));
    assert!(matches!(b[2], Block::Item { level: 1, tabs: 1, .. }));
    assert_eq!(b[3], Block::Text);
    assert!(matches!(b[4], Block::Item { ordered: true, ref marker, .. } if *marker == (0..3)));
    assert_eq!(b[5], Block::Quote { marker: 2, callout: None });
    assert_eq!(b[6], Block::Rule);
    assert!(matches!(b[7], Block::Code { first: true, fence: true, lang: Some(_), .. }));
    assert!(matches!(b[8], Block::Code { first: false, last: false, fence: false, .. }));
    assert!(matches!(b[9], Block::Code { last: true, fence: true, .. }));
    assert_eq!(b[10], Block::Text);
}

#[test]
fn unclosed_fence_ends_at_document_end() {
    let b = blocks("```\nкод\nещё");
    assert!(matches!(b[2], Block::Code { first: false, last: true, fence: false, .. }));
}

#[test]
fn task_items_are_found() {
    let b = blocks("- [ ] дело\n- [x] сделано\n1. [ ] номер");
    assert!(matches!(b[0], Block::Item { task: Some((ref r, false)), .. } if *r == (2..6)));
    assert!(matches!(b[1], Block::Item { task: Some((_, true)), .. }));
    assert!(matches!(b[2], Block::Item { task: Some(_), .. }));
}

#[test]
fn list_callouts_and_continuations() {
    let b = blocks("- $ деньги\n\t2) пункт\n\t   продолжение\n\t1) дальше");
    assert!(matches!(b[0], Block::Item { callout: Some((2, 5)), .. }));
    assert!(matches!(b[2], Block::Continuation { lead: 4, width: 3, .. }));
    assert!(matches!(b[3], Block::Item { ordered: true, .. }));
}

#[test]
fn quote_callouts() {
    let b = blocks("> [!quote] текст\n> дальше\n\n> [!book] [[Книга]]");
    assert!(matches!(b[0], Block::Quote { callout: Some((ref r, Callout::Quote)), .. } if *r == (2..11)));
    assert_eq!(b[1], Block::Quote { marker: 2, callout: None });
    assert!(matches!(b[3], Block::Quote { callout: Some((_, Callout::Book)), .. }));
}

#[test]
fn tables_form_runs() {
    let b = blocks("текст\n| a | b |\n| --- | --- |\n| 1 | 2 |\n<!--q-table:{}-->\nпосле");
    assert_eq!(b[1], Block::Table { first: true, last: false });
    assert_eq!(b[3], Block::Table { first: false, last: false });
    assert_eq!(b[4], Block::Table { first: false, last: true });
    assert_eq!(b[5], Block::Text);
}

#[test]
fn hashtags_need_space_around() {
    let text = "#tag #тег a#no #ok_1 #";
    let tags: Vec<&str> = hashtags(text).into_iter().map(|r| &text[r]).collect();
    assert_eq!(tags, ["#tag", "#тег", "#ok_1"]);
}

#[test]
fn inline_spans_have_inner_ranges() {
    let text = "a *b* **c** ~~d~~ `e` [f](g) [[h|i]] [3](quantum-reader:x)";
    let spans = inline(text);
    let pick = |k: Kind| spans.iter().find(|s| s.kind == k).map(|s| &text[s.inner.clone()]).unwrap();
    assert_eq!(pick(Kind::Emphasis), "b");
    assert_eq!(pick(Kind::Strong), "c");
    assert_eq!(pick(Kind::Strike), "d");
    assert_eq!(pick(Kind::Code), "e");
    assert_eq!(pick(Kind::Reader), "3");
    let links: Vec<&str> = spans.iter().filter(|s| s.kind == Kind::Link).map(|s| &text[s.inner.clone()]).collect();
    assert_eq!(links, ["f", "i"]);
}
