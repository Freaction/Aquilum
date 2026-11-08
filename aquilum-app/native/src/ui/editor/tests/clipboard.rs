use std::cell::RefCell;
use std::rc::Rc;

use masonry::app::{RenderRoot, RenderRootSignal};
use masonry::core::NewWidget;
use masonry::core::keyboard::{Code, Key, Modifiers, NamedKey};
use masonry::core::{Ime, TextEvent};

use super::support::{TAG, click, command_key, editor_id, focused, named, paste, root, text};
use crate::test_dir::{command, key_with_code};
use crate::ui::editor::TextEditor;

fn capturing(editor: TextEditor) -> (RenderRoot, Rc<RefCell<Vec<String>>>) {
    let clipboard = Rc::new(RefCell::new(Vec::new()));
    let stored = clipboard.clone();
    let mut root = RenderRoot::new(NewWidget::new(editor).with_tag(TAG).erased(), move |signal| {
        if let RenderRootSignal::ClipboardStore(text) = signal {
            stored.borrow_mut().push(text);
        }
    }, crate::test_dir::options(600, 600, 1.0));
    root.focus_on(Some(editor_id(&root)));
    let _ = root.redraw();
    (root, clipboard)
}

#[test]
fn layout_copy_cut_paste_and_undo_work_in_fields_code_quotes_and_nested_lists() {
    for source in ["abc", "текст abc", "```rust\nabc\n```", "> > abc", "- пункт\n  - abc", "- [ ] abc", "> - пункт\n>   - abc", "- пункт\n  ```rust\n  abc\n  ```", "> ```rust\n> abc\n> ```"] {
        let editor = if source == "abc" { TextEditor::single_line(source) } else { TextEditor::new(source).markdown() };
        let (mut root, clipboard) = capturing(editor);
        let start = source.find("abc").unwrap();
        let id = editor_id(&root);
        root.edit_widget(id, |mut w| TextEditor::select_byte_range(&mut w.downcast(), start, start + 3));
        let _ = root.redraw();
        key_with_code(&mut root, Key::Character("c".into()), Code::KeyI, command());
        assert_eq!(clipboard.borrow().last().map(String::as_str), Some("abc"), "copy in {source:?}");
        assert_eq!(text(&root), source, "copy must not run formatting");
        clipboard.borrow_mut().clear();
        key_with_code(&mut root, Key::Character("x".into()), Code::KeyB, command());
        assert_eq!(clipboard.borrow().last().map(String::as_str), Some("abc"));
        assert_eq!(text(&root), source.replace("abc", ""));
        key_with_code(&mut root, Key::Character("z".into()), Code::Slash, command());
        assert_eq!(text(&root), source, "undo in {source:?}");
        key_with_code(&mut root, Key::Character("z".into()), Code::Slash, command() | Modifiers::SHIFT);
        assert_eq!(text(&root), source.replace("abc", ""));
        paste(&mut root, "abc");
        assert_eq!(text(&root), source, "paste in {source:?}");
    }
}

#[test]
fn layout_shortcuts_edit_table_cells_in_notes_quotes_and_lists() {
    for (source, x, y) in [
        ("| abc | b |\n| --- | --- |\n| 1 | 2 |", 60.0, 50.0),
        ("> | abc | b |\n> | --- | --- |\n> | 1 | 2 |", 90.0, 50.0),
        ("- пункт\n  | abc | b |\n  | --- | --- |\n  | 1 | 2 |", 90.0, 70.0),
    ] {
        let (mut root, clipboard) = capturing(TextEditor::new(source).markdown());
        click(&mut root, x, y);
        key_with_code(&mut root, Key::Character("a".into()), Code::KeyQ, command());
        key_with_code(&mut root, Key::Character("c".into()), Code::KeyI, command());
        assert_eq!(clipboard.borrow().last().map(String::as_str), Some("abc"), "copy in table {source:?}");
        assert_eq!(text(&root), source);
        clipboard.borrow_mut().clear();
        key_with_code(&mut root, Key::Character("x".into()), Code::KeyB, command());
        assert_eq!(clipboard.borrow().last().map(String::as_str), Some("abc"), "cut in table {source:?}");
        assert!(!text(&root).contains("abc"));
        key_with_code(&mut root, Key::Character("z".into()), Code::Slash, command());
        assert_eq!(text(&root), source, "undo in table {source:?}");
    }
}

#[test]
fn paste_and_ime_insert_text() {
    let mut root = root("ab");
    named(&mut root, NamedKey::ArrowRight);
    paste(&mut root, "1\n2");
    assert_eq!(text(&root), "a1\n2b");
    root.handle_text_event(TextEvent::Ime(Ime::Preedit("ко".into(), Some((4, 4)))));
    let _ = root.redraw();
    assert_eq!(text(&root), "a1\n2b");
    root.handle_text_event(TextEvent::Ime(Ime::Commit("кот".into())));
    let _ = root.redraw();
    assert_eq!(text(&root), "a1\n2котb");
}

#[test]
fn pasted_url_becomes_titled_link() {
    let mut root = focused(TextEditor::new("a_b").markdown(), 400, 300);
    command_key(&mut root, "a");
    paste(&mut root, "https://example.com");
    assert_eq!(text(&root), r"[a\_b](https://example.com)");
    named(&mut root, NamedKey::End);
    paste(&mut root, " https://x.org/a ");
    let pasted = text(&root);
    let placeholder = pasted.split('[').nth(2).and_then(|s| s.split(']').next()).expect("заглушка").to_owned();
    assert!(placeholder.starts_with("Fetching Title#"));
    let id = editor_id(&root);
    root.edit_widget(id, |mut w| TextEditor::resolve_link_title(&mut w.downcast(), &placeholder, "X*Y"));
    assert_eq!(text(&root), r"[a\_b](https://example.com)[X\*Y](https://x.org/a)");
    paste(&mut root, "https://x.org/p.png");
    assert!(text(&root).ends_with(")https://x.org/p.png"));
}
