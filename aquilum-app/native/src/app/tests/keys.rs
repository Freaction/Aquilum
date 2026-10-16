use super::*;

#[test]
fn page_keys_scroll_the_note_by_a_page_and_carry_the_caret() {
    let mut h = Harness::new();
    let note = h.vault.path().join("Длинная.md");
    std::fs::write(&note, (0..300).map(|i| format!("Абзац {i}

")).collect::<String>()).unwrap();
    h.app.open(&mut h.root, note);
    let body = h.app.note_ids.as_ref().unwrap().body;
    h.root.focus_on(Some(body.id()));
    h.deliver();
    let selection = |h: &Harness| body.get(&h.root).unwrap().inner().selection();
    h.key(Key::Named(NamedKey::PageDown), Modifiers::empty());
    let (page, first) = (note_offset(&h), caret(&h));
    assert!(page > 200.0 && first > 0, "PageDown: прокрутка {page}, каретка {first}");
    h.key(Key::Named(NamedKey::PageDown), Modifiers::SHIFT);
    assert!((note_offset(&h) - 2.0 * page).abs() < 1.0, "второй PageDown сдвинул на {}", note_offset(&h) - page);
    assert_eq!(selection(&h).start, first, "Shift+PageDown не выделил страницу");
    assert!(selection(&h).end > first);
    for _ in 0..3 {
        h.key(Key::Named(NamedKey::PageUp), Modifiers::empty());
    }
    assert_eq!((note_offset(&h), caret(&h)), (0.0, 0), "PageUp у начала не вернул в начало");
}
