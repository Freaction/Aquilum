use super::*;

#[test]
fn reader_turns_pages_and_saves_position() {
    let mut h = Harness::new();
    let (file, page) = book_note(&h, "");
    open_book(&mut h, &file, Some(&page), "Книга");
    assert!(h.reader(), "читалка не открылась");
    let fields = |p: &std::path::Path| std::fs::read_to_string(p).unwrap();
    assert!(fields(&page).contains("pages: 0/"), "{}", fields(&page));
    turn_pages(&mut h, 5);
    let text = fields(&page);
    assert!(text.contains("reader_position: epubcfi(/6/2!"), "{text}");
    let first = text.lines().find(|l| l.starts_with("reader_position")).unwrap().to_owned();
    turn_pages(&mut h, 200);
    let text = fields(&page);
    assert!(text.contains("reader_position: epubcfi(/6/4!"), "вторая глава не открылась: {text}");
    assert_ne!(first, text.lines().find(|l| l.starts_with("reader_position")).unwrap());
    let percent: u32 = text.lines().find_map(|l| l.strip_prefix("read_percent: ")).unwrap().trim().parse().unwrap();
    assert!(percent > 50, "{text}");
    h.escape();
    assert!(!h.reader(), "Escape не закрыл читалку");
    let saved = text.lines().find_map(|l| l.strip_prefix("reader_position: ")).unwrap().to_owned();
    open_book(&mut h, &file, Some(&page), "Книга");
    let text = fields(&page);
    let reopened = text.lines().find_map(|l| l.strip_prefix("reader_position: ")).unwrap();
    assert_eq!(reopened, saved, "позиция не восстановилась");
}

#[test]
fn reader_settings_inputs_keep_popover_open() {
    let mut h = Harness::new();
    let file = h.vault.path().join("книга.fb2");
    std::fs::write(&file, sample_fb2()).unwrap();
    h.app.open_reader(&mut h.root, crate::app::reader::Opening { file, page: None, title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.click(936.0, 30.0);
    let _ = h.root.redraw();
    let size_field = match &h.app.popup {
        Some(Popup::Reader(state)) => state
            .bindings
            .iter()
            .find_map(|(id, b)| matches!(b, crate::ui::settings::Binding::Number { number, .. } if number.max == 32.0).then_some(*id))
            .expect("поле размера шрифта"),
        _ => panic!("читалка закрылась"),
    };
    let open = |h: &Harness| match &h.app.popup {
        Some(Popup::Reader(state)) => h.root.get_widget(state.layer).and_then(|w| w.downcast::<crate::ui::reader::ReaderView>().map(|v| v.inner().has_popover())).unwrap_or(false),
        _ => false,
    };
    assert!(open(&h), "настройки не открылись");
    let field = h.root.get_widget(size_field).unwrap().ctx().bounding_box();
    h.click(field.center().x, field.center().y);
    let _ = h.root.redraw();
    assert!(open(&h), "щелчок в поле закрыл настройки");
    h.key(Key::Character("a".into()), crate::test_dir::command());
    h.type_text("20");
    let _ = h.root.redraw();
    assert!(open(&h));
    assert_eq!(h.config().reader.font.font_size_base, 20);
    h.click(500.0, 300.0);
    let _ = h.root.redraw();
    assert!(!open(&h), "щелчок по книге не закрыл настройки");
    assert!(h.reader());
}
