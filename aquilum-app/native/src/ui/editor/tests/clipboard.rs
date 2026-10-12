use masonry::core::keyboard::NamedKey;
use masonry::core::{Ime, TextEvent};

use super::support::{command_key, editor_id, focused, named, paste, root, text};
use crate::ui::editor::TextEditor;

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
