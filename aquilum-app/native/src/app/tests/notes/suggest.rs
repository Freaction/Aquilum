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

#[test]
fn typing_suggests_notes_and_enter_links_them() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("Проектирование.md"), "x").unwrap();
    let note = vault.join("Черновик.md");
    std::fs::write(&note, "").unwrap();
    h.app.tree.paths_changed(&mut h.root, &[vault.join("Проектирование.md"), note.clone()]);
    h.open(note);
    let _ = h.root.redraw();
    let editor = h.app.editor_id().unwrap();
    h.root.handle_text_event(TextEvent::WindowFocusChange(true));
    h.root.focus_on(Some(editor));
    let text = |h: &Harness| h.root.get_widget(editor).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text();
    for round in ["", "[["] {
        for ch in round.chars().chain("Прое".chars()) {
            h.key(Key::Character(ch.to_string().into()), Modifiers::empty());
        }
        let mut shown = false;
        for _ in 0..200 {
            h.app.on_core_events(&mut h.root);
            let _ = h.root.redraw();
            if h.app.suggest_layer.is_some() {
                shown = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(shown, "список подсказок не появился ({round:?}): {:?}", text(&h));
        if let Ok(out) = std::env::var("AQ_OUT") {
            let (layers, _) = h.root.redraw();
            let mut r = crate::render::Renderer::new(2);
            r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
            std::fs::write(std::path::Path::new(&out).join(format!("suggest-{}.png", round.len())), r.png()).unwrap();
        }
        h.key(Key::Named(NamedKey::Enter), Modifiers::empty());
        let _ = h.root.redraw();
        h.app.on_core_events(&mut h.root);
        assert!(text(&h).ends_with("[[Проектирование]]"), "{:?}", text(&h));
        assert!(h.app.suggest_layer.is_none(), "список не закрылся");
        h.key(Key::Character(" ".into()), Modifiers::empty());
    }
    assert_eq!(text(&h), "[[Проектирование]] [[Проектирование]] ");
}
