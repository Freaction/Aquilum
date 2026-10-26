use super::*;

fn suggesting(name: &str, text: &str, moves: &[NamedKey], typed: &str, what: &str) -> Harness {
    let mut h = Harness::new();
    let note = h.write_note(name, text);
    h.app.interface.set_metadata_expanded(&note, true);
    h.open(note);
    let _ = h.root.redraw();
    h.focus_editor();
    for key in moves {
        h.key(Key::Named(key.clone()), Modifiers::empty());
    }
    h.type_text(typed);
    h.app.on_core_events(&mut h.root);
    let _ = h.root.redraw();
    assert!(h.app.suggest_layer.is_some(), "{what}: {:?}", h.body_text());
    h
}

fn complete(name: &str, text: &str, moves: &[NamedKey], typed: &str, what: &str) -> String {
    let mut h = suggesting(name, text, moves, typed, what);
    h.key(Key::Named(NamedKey::Enter), Modifiers::empty());
    let _ = h.root.redraw();
    h.app.on_core_events(&mut h.root);
    h.body_text()
}

#[test]
fn page_cover_field_suggests_pattern_names() {
    let text = complete("Обложка.md", "---\nPage_cover:\n---\nтекст", &[NamedKey::ArrowDown, NamedKey::End], " tun", "подсказки обложек не появились");
    assert_eq!(text, "---\nPage_cover: tunnel\n---\nтекст");
    assert_eq!(crate::cover::pattern_id(Some("tunnel")), Some("tunnel"));
    assert_eq!(crate::cover::pattern_id(Some("pattern:Tunnel")), Some("tunnel"));
}

#[test]
fn frontmatter_keys_are_suggested_whatever_the_case() {
    let text = complete("Свойства.md", "---\n\n---\nтекст", &[NamedKey::ArrowDown], "PAGE_c", "подсказки ключей не появились");
    assert_eq!(text, "---\nPage_cover: \n---\nтекст");
}

#[test]
fn escape_closes_suggestions_without_editing() {
    let mut h = suggesting("Свойства.md", "---\n\n---\nтекст", &[NamedKey::ArrowDown], "PAGE_c", "подсказки ключей не появились");
    h.escape();
    let _ = h.root.redraw();
    h.app.on_core_events(&mut h.root);
    assert!(h.app.suggest_layer.is_none(), "Esc не закрыл подсказки");
    assert_eq!(h.body_text(), "---\nPAGE_c\n---\nтекст");
}
