use masonry::core::keyboard::{Key, Modifiers, NamedKey};

use super::support::{command_key, markdown_root, named, press, root, selected, text, typing};
use crate::test_dir::command;
use crate::ui::editor::Smart;
use crate::ui::editor::TextEditor;

#[test]
fn typing_and_enter_build_paragraphs() {
    let mut root = root("");
    typing(&mut root, "привет");
    named(&mut root, NamedKey::Enter);
    typing(&mut root, "мир");
    assert_eq!(text(&root), "привет\nмир");
}

#[test]
fn backspace_at_paragraph_start_joins_paragraphs() {
    let mut root = root("один\nдва");
    press(&mut root, Key::Named(NamedKey::End), command());
    for _ in 0..3 {
        named(&mut root, NamedKey::ArrowLeft);
    }
    named(&mut root, NamedKey::Backspace);
    assert_eq!(text(&root), "одиндва");
    named(&mut root, NamedKey::Delete);
    assert_eq!(text(&root), "одинва");
}

#[test]
fn edit_relayouts_only_its_paragraph() {
    let body: String = (0..2000).map(|i| format!("Абзац {i}: текст заметки, который переносится на несколько строк в колонке редактора.\n")).collect();
    let mut root = root(&body);
    press(&mut root, Key::Named(NamedKey::End), command());
    let started = std::time::Instant::now();
    typing(&mut root, "abcdefghij");
    let per_key = started.elapsed() / 10;
    assert!(text(&root).ends_with("abcdefghij"));
    assert!(per_key < std::time::Duration::from_millis(5), "нажатие в большом документе: {per_key:?}");
}

#[test]
fn undo_and_redo_group_typing_and_deleting() {
    let mut root = root("");
    typing(&mut root, "привет мир");
    named(&mut root, NamedKey::Backspace);
    named(&mut root, NamedKey::Backspace);
    assert_eq!(text(&root), "привет м");
    command_key(&mut root, "z");
    assert_eq!(text(&root), "привет мир");
    command_key(&mut root, "z");
    assert_eq!(text(&root), "");
    command_key(&mut root, "y");
    assert_eq!(text(&root), "привет мир");
    press(&mut root, Key::Character("z".into()), command() | Modifiers::SHIFT);
    assert_eq!(text(&root), "привет м");
    typing(&mut root, "!");
    command_key(&mut root, "y");
    assert_eq!(text(&root), "привет м!");
}

#[test]
fn redo_restores_the_selection_inside_wrapped_brackets() {
    let mut root = root("abc");
    command_key(&mut root, "a");
    typing(&mut root, "(");
    assert_eq!((text(&root).as_str(), selected(&root).as_str()), ("(abc)", "abc"));
    command_key(&mut root, "z");
    assert_eq!(text(&root), "abc");
    command_key(&mut root, "y");
    assert_eq!((text(&root).as_str(), selected(&root).as_str()), ("(abc)", "abc"));
}

#[test]
fn double_hyphen_and_space_become_dash() {
    let mut root = markdown_root("");
    let id = super::support::editor_id(&root);
    root.edit_widget(id, |mut w| TextEditor::set_smart(&mut w.downcast(), Smart { dashes: false, ..Default::default() }));
    typing(&mut root, "a-- b");
    assert_eq!(text(&root), "a-- b");
    root.edit_widget(id, |mut w| TextEditor::set_smart(&mut w.downcast(), Smart::default()));
    typing(&mut root, " c-- d--- e");
    assert_eq!(text(&root), "a-- b c— d--- e");
}
