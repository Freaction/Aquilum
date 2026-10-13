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

fn reader_size_field(h: &mut Harness) -> masonry::core::WidgetId {
    let file = h.vault.path().join("книга.fb2");
    std::fs::write(&file, sample_fb2()).unwrap();
    h.app.open_reader(&mut h.root, crate::app::reader::Opening { file, page: None, title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.click(936.0, 30.0);
    let _ = h.root.redraw();
    match &h.app.popup {
        Some(Popup::Reader(state)) => state
            .bindings
            .iter()
            .find_map(|(id, b)| matches!(b, crate::ui::settings::Binding::Number { number, .. } if number.max == 32.0).then_some(*id))
            .expect("поле размера шрифта"),
        _ => panic!("читалка закрылась"),
    }
}

#[test]
fn arrow_keys_step_the_font_size_inside_the_reader() {
    let mut h = Harness::new();
    let size_field = reader_size_field(&mut h);
    let field = h.root.get_widget(size_field).unwrap().ctx().bounding_box();
    h.click(field.center().x, field.center().y);
    let before = h.config().reader.font.font_size_base;
    for key in [NamedKey::ArrowUp, NamedKey::ArrowUp] {
        assert!(h.app.settings_key(&mut h.root, &Key::Named(key)).is_some(), "стрелка в поле размера шрифта читалки не обработана");
    }
    assert_eq!(h.config().reader.font.font_size_base, before + 2, "стрелка вверх не увеличила размер шрифта читалки");
    assert!(h.app.settings_key(&mut h.root, &Key::Named(NamedKey::ArrowDown)).is_some());
    assert_eq!(h.config().reader.font.font_size_base, before + 1, "стрелка вниз не уменьшила размер шрифта читалки");
    assert!(h.reader(), "стрелки закрыли читалку");
}

#[test]
fn changing_the_font_size_back_and_forth_keeps_the_reading_place() {
    let mut h = Harness::new();
    let (file, page) = book_note(&h, "");
    open_book(&mut h, &file, Some(&page), "Книга");
    turn_pages(&mut h, 7);
    let position = |p: &std::path::Path| std::fs::read_to_string(p).unwrap().lines().find_map(|l| l.strip_prefix("reader_position: ").map(str::to_owned)).unwrap();
    let before = position(&page);
    h.click(936.0, 30.0);
    let _ = h.root.redraw();
    let field = match &h.app.popup {
        Some(Popup::Reader(state)) => state
            .bindings
            .iter()
            .find_map(|(id, b)| matches!(b, crate::ui::settings::Binding::Number { number, .. } if number.max == 32.0).then_some(*id))
            .expect("поле размера шрифта"),
        _ => panic!("читалка закрылась"),
    };
    let area = h.root.get_widget(field).unwrap().ctx().bounding_box();
    h.click(area.center().x, area.center().y);
    let size = h.config().reader.font.font_size_base;
    for _ in 0..4 {
        h.app.settings_key(&mut h.root, &Key::Named(NamedKey::ArrowUp));
        let _ = h.root.redraw();
        h.deliver();
    }
    assert_eq!(h.config().reader.font.font_size_base, size + 4);
    for _ in 0..4 {
        h.app.settings_key(&mut h.root, &Key::Named(NamedKey::ArrowDown));
        let _ = h.root.redraw();
        h.deliver();
    }
    assert_eq!(h.config().reader.font.font_size_base, size);
    assert_eq!(position(&page), before, "после возврата размера шрифта читалка открыла другое место");
}

#[test]
fn reader_settings_inputs_keep_popover_open() {
    let mut h = Harness::new();
    let size_field = reader_size_field(&mut h);
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
