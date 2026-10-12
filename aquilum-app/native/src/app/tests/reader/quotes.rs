use super::*;

#[test]
fn reader_quotes_round_trip() {
    let mut h = Harness::new();
    let (file, page) = book_note(&h, "Заметки о книге");
    h.app.open(&mut h.root, page.clone());
    let _ = h.root.redraw();
    open_book(&mut h, &file, Some(&page), "Книга");
    h.drag((120.0, 160.0), (420.0, 220.0));
    let _ = h.root.redraw();
    h.deliver();
    let quote = match &h.app.popup {
        Some(Popup::Reader(state)) => state.ids.quote.unwrap(),
        _ => panic!("читалка закрылась"),
    };
    let button = h.root.get_widget(quote).unwrap().ctx().bounding_box();
    assert!(button.width() > 0.0, "нет кнопки цитаты");
    h.click(button.center().x, button.center().y);
    let _ = h.root.redraw();
    h.deliver();
    let text = h.app.editor_text.clone();
    let found = crate::reader::quotes::find(&text);
    assert_eq!(found.len(), 1, "{text}");
    assert_eq!(found[0].label, "1");
    assert_eq!(found[0].book.as_deref(), Some("Files/книга.fb2"));
    assert!(found[0].cfi.starts_with("epubcfi(/6/2!"), "{}", found[0].cfi);
    assert!(text[found[0].text.clone()].contains("Абзац номер"), "{text}");
    assert!(std::fs::read_to_string(&page).unwrap().contains("[!quote]"), "цитата не сохранена в файл");
    let _ = h.root.redraw();
    let label = match &h.app.popup {
        Some(Popup::Reader(state)) => h.root.get_widget(state.layer).and_then(|w| w.downcast::<crate::ui::reader::ReaderView>().map(|v| v.inner().label_rects())).unwrap(),
        _ => panic!("читалка закрылась"),
    };
    assert_eq!(label.len(), 1, "метка цитаты не нарисована");
    h.click(label[0].center().x, label[0].center().y);
    assert!(!h.reader(), "щелчок по метке не закрыл читалку");
    let selected = h.root.get_widget(h.app.editor_id().unwrap()).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().selected();
    assert_eq!(selected, text[found[0].text.clone()]);
    let url = format!("aquilum-reader:cfi={}", found[0].cfi.replace('(', "%28").replace(')', "%29"));
    h.app.open_target(&mut h.root, crate::links::Target::Url(url), false);
    h.deliver();
    let _ = h.root.redraw();
    assert!(h.reader(), "ссылка на цитату не открыла книгу");
}
