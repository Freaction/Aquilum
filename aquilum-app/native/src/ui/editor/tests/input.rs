use masonry::app::RenderRoot;
use masonry::core::keyboard::{Key, Modifiers, NamedKey};

use super::support::{command_key, markdown_root, named, press, root, selected, text, typing, TAG};
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

fn at_second_line_start(body: &str) -> RenderRoot {
    let mut root = root(body);
    press(&mut root, Key::Named(NamedKey::Home), command());
    named(&mut root, NamedKey::ArrowDown);
    named(&mut root, NamedKey::Home);
    let caret = root.get_widget_with_tag(TAG).expect("редактор").inner().caret_offset();
    assert!(caret > 0 && caret < body.len(), "каретка не на второй строке: {caret}");
    root
}

#[test]
fn backspace_at_a_wrapped_line_start_deletes_the_character_before_it() {
    let body = "слово ".repeat(40);
    let mut root = at_second_line_start(&body);
    named(&mut root, NamedKey::Backspace);
    assert_eq!(text(&root).chars().count(), body.chars().count() - 1);
}

#[test]
fn delete_at_a_wrapped_line_end_deletes_the_character_after_it() {
    let body = "слово ".repeat(40);
    let mut root = at_second_line_start(&body);
    named(&mut root, NamedKey::ArrowLeft);
    named(&mut root, NamedKey::Delete);
    assert_eq!(text(&root).chars().count(), body.chars().count() - 1);
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
fn words_and_backspaces_from_the_keyboard_undo_in_groups() {
    use masonry::core::{Ime, TextEvent};

    let mut root = root("");
    for word in ["при", "вет ", "мир"] {
        root.handle_text_event(TextEvent::Ime(Ime::Commit(word.into())));
    }
    for _ in 0..2 {
        root.handle_text_event(TextEvent::Ime(Ime::DeleteSurrounding { before_bytes: 2, after_bytes: 0 }));
    }
    assert_eq!(text(&root), "привет м");
    command_key(&mut root, "z");
    assert_eq!(text(&root), "привет мир");
    command_key(&mut root, "z");
    assert_eq!(text(&root), "");
}

#[test]
fn formatted_paste_goes_into_the_note_but_never_into_code_or_plain_fields() {
    let paste = |root: &mut RenderRoot, text: &str| {
        let id = super::support::editor_id(root);
        root.edit_widget(id, |mut widget| TextEditor::paste_markdown(&mut widget.downcast(), text))
    };
    let mut note = markdown_root("текст ");
    press(&mut note, Key::Named(NamedKey::End), command());
    assert!(paste(&mut note, "**жирный**"));
    assert_eq!(text(&note), "текст **жирный**");
    let mut code = markdown_root("```\nкод\n```");
    named(&mut code, NamedKey::ArrowDown);
    assert!(!paste(&mut code, "**жирный**"));
    assert_eq!(text(&code), "```\nкод\n```");
    assert!(!paste(&mut root("текст"), "**жирный**"));
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
