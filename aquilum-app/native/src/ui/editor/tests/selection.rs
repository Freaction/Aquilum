use masonry::core::keyboard::{Key, Modifiers, NamedKey};

use super::support::{command_key, markdown_root, named, press, root, selected, tap, text, typing};
use crate::test_dir::command;

#[test]
fn arrows_cross_paragraphs_and_keep_column() {
    let mut root = root("abcdef\nab\nabcdef");
    for _ in 0..5 {
        named(&mut root, NamedKey::ArrowRight);
    }
    named(&mut root, NamedKey::ArrowDown);
    named(&mut root, NamedKey::ArrowDown);
    typing(&mut root, "X");
    assert_eq!(text(&root), "abcdef\nab\nabcdeXf");
    named(&mut root, NamedKey::Home);
    named(&mut root, NamedKey::ArrowLeft);
    typing(&mut root, "Y");
    assert_eq!(text(&root), "abcdef\nabY\nabcdeXf");
}

#[test]
fn selection_is_replaced_by_typing() {
    let mut root = root("hello world");
    named(&mut root, NamedKey::End);
    for _ in 0..5 {
        press(&mut root, Key::Named(NamedKey::ArrowLeft), Modifiers::SHIFT);
    }
    typing(&mut root, "there");
    assert_eq!(text(&root), "hello there");
    press(&mut root, Key::Named(NamedKey::Backspace), command());
    assert_eq!(text(&root), "hello ");
    command_key(&mut root, "a");
    named(&mut root, NamedKey::Delete);
    assert_eq!(text(&root), "");
}

#[test]
fn double_tap_selects_word_everywhere() {
    for (source, word) in [("- & Буллит один\n\nтекст", "Буллит"), ("1. Буллит один\n\nтекст", "Буллит"), ("> [!quote] Буллит один\n\nтекст", "Буллит"), ("Буллит один\n\nтекст", "Буллит"), ("\t- Буллит один\n\nтекст", "Буллит")] {
        let mut root = markdown_root(source);
        let x = (0..400).step_by(4).map(f64::from).find(|x| {
            tap(&mut root, *x, 10.0, 2, 0.0);
            selected(&root) == word
        });
        let Some(x) = x else { panic!("{source:?}: слово не выделяется двойным щелчком") };
        tap(&mut root, x, 10.0, 1, 0.0);
        tap(&mut root, x, 10.0, 2, 3.0);
        assert_eq!(selected(&root), word, "{source:?}: после касания с дрожанием");
    }
}
