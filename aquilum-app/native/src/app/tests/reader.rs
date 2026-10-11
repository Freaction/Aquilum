use super::*;

fn sample_fb2() -> String {
    let paragraphs: String = (1..=400).map(|i| format!("<p>Абзац номер {i}. Съешь же ещё этих мягких французских булок да выпей чаю, повторяя это снова и снова.</p>")).collect();
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\"><body><section><title><p>Глава первая</p></title>{paragraphs}</section><section><title><p>Глава вторая</p></title>{paragraphs}</section></body></FictionBook>")
}

impl Harness {
    pub(super) fn reader(&self) -> bool {
        matches!(&self.app.popup, Some(Popup::Reader(_)))
    }
}

#[test]
fn reader_turns_pages_and_saves_position() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::create_dir_all(vault.join("Files")).unwrap();
    let file = vault.join("Files").join("книга.fb2");
    std::fs::write(&file, sample_fb2()).unwrap();
    let page = vault.join("📖 Книга.md");
    std::fs::write(&page, "---
Путь к файлу: Files/книга.fb2
pages:
---
").unwrap();
    h.app.open_reader(&mut h.root, crate::app::reader::Opening { file: file.clone(), page: Some(page.clone()), title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.deliver();
    assert!(h.reader(), "читалка не открылась");
    let fields = |p: &std::path::Path| std::fs::read_to_string(p).unwrap();
    assert!(fields(&page).contains("pages: 0/"), "{}", fields(&page));
    for _ in 0..5 {
        h.key(Key::Named(NamedKey::ArrowRight), Modifiers::empty());
        let _ = h.root.redraw();
        h.deliver();
    }
    let text = fields(&page);
    assert!(text.contains("reader_position: epubcfi(/6/2!"), "{text}");
    let first = text.lines().find(|l| l.starts_with("reader_position")).unwrap().to_owned();
    for _ in 0..200 {
        h.key(Key::Named(NamedKey::ArrowRight), Modifiers::empty());
        let _ = h.root.redraw();
        h.deliver();
    }
    let text = fields(&page);
    assert!(text.contains("reader_position: epubcfi(/6/4!"), "вторая глава не открылась: {text}");
    assert_ne!(first, text.lines().find(|l| l.starts_with("reader_position")).unwrap());
    let percent: u32 = text.lines().find_map(|l| l.strip_prefix("read_percent: ")).unwrap().trim().parse().unwrap();
    assert!(percent > 50, "{text}");
    h.escape();
    assert!(!h.reader(), "Escape не закрыл читалку");
    let saved = text.lines().find_map(|l| l.strip_prefix("reader_position: ")).unwrap().to_owned();
    h.app.open_reader(&mut h.root, crate::app::reader::Opening { file: file.clone(), page: Some(page.clone()), title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.deliver();
    let text = fields(&page);
    let reopened = text.lines().find_map(|l| l.strip_prefix("reader_position: ")).unwrap();
    assert_eq!(reopened, saved, "позиция не восстановилась");
}

#[test]
fn reader_settings_inputs_keep_popover_open() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    let file = vault.join("книга.fb2");
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
    h.key(Key::Character("a".into()), Modifiers::CONTROL);
    for ch in ["2", "0"] {
        h.key(Key::Character(ch.into()), Modifiers::empty());
    }
    let _ = h.root.redraw();
    assert!(open(&h));
    assert_eq!(h.config().reader.font.font_size_base, 20);
    h.click(500.0, 300.0);
    let _ = h.root.redraw();
    assert!(!open(&h), "щелчок по книге не закрыл настройки");
    assert!(h.reader());
}

#[test]
fn font_size_field_keeps_focus_and_blank_means_default() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    let editor_nav = match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog.nav.iter().find(|(_, s)| *s == Section::Editor).map(|(id, _)| *id).unwrap(),
        _ => panic!("настройки не открылись"),
    };
    let nav = h.root.get_widget(editor_nav).unwrap().ctx().bounding_box();
    h.click(nav.center().x, nav.center().y);
    let field = |h: &Harness| match &h.app.popup {
        Some(Popup::Settings { dialog, .. }) => dialog
            .bindings
            .iter()
            .find_map(|(id, b)| matches!(b, crate::ui::settings::Binding::Number { number, .. } if number.blank).then_some(*id))
            .unwrap(),
        _ => panic!("настройки закрылись"),
    };
    let rect = h.root.get_widget(field(&h)).unwrap().ctx().bounding_box();
    h.click(rect.center().x, rect.center().y);
    h.key(Key::Named(NamedKey::End), Modifiers::empty());
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    let _ = h.root.redraw();
    assert_eq!(h.config().editor.font.font_size_base, 12, "меньше 12 не бывает, как в Таури");
    assert_eq!(h.root.focused_widget(), Some(field(&h)), "фокус слетел");
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    let _ = h.root.redraw();
    assert_eq!(h.config().editor.font.font_size_base, 16, "пустое поле — размер по умолчанию");
    assert_eq!(h.root.focused_widget(), Some(field(&h)), "фокус слетел");
    let text = |h: &Harness| h.root.get_widget(field(h)).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text().to_owned();
    assert_eq!(text(&h), "");
    for ch in ["2", "0"] {
        h.key(Key::Character(ch.into()), Modifiers::empty());
    }
    let _ = h.root.redraw();
    assert_eq!(h.config().editor.font.font_size_base, 20);
    assert_eq!(text(&h), "20");
    assert_eq!(h.root.focused_widget(), Some(field(&h)), "фокус слетел");
}

#[test]
fn reader_quotes_round_trip() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::create_dir_all(vault.join("Files")).unwrap();
    let file = vault.join("Files").join("книга.fb2");
    std::fs::write(&file, sample_fb2()).unwrap();
    let page = vault.join("📖 Книга.md");
    std::fs::write(&page, "---
Путь к файлу: Files/книга.fb2
pages:
---
Заметки о книге").unwrap();
    h.app.open(&mut h.root, page.clone());
    let _ = h.root.redraw();
    h.app.open_reader(&mut h.root, crate::app::reader::Opening { file: file.clone(), page: Some(page.clone()), title: "Книга".into(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.deliver();
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

#[test]
#[ignore]
fn reader_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let book = std::path::PathBuf::from(std::env::var("AQ_BOOK").unwrap());
    let mut h = Harness::new();
    let note = h.vault.path().join("a.md");
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    h.app.open_reader(&mut h.root, crate::app::reader::Opening { file: book.clone(), page: None, title: book.file_stem().unwrap().to_string_lossy().into_owned(), initial: None, peek: false });
    h.deliver();
    let _ = h.root.redraw();
    h.deliver();
    let size = h.root.size();
    for i in 0..4 {
        let started = std::time::Instant::now();
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, size.width as u16, size.height as u16, 1.0, theme::background()).unwrap();
        std::fs::write(out.join(format!("reader-{i}.png")), r.png()).unwrap();
        eprintln!("страница {i}: {:?}", started.elapsed());
        for _ in 0..3 {
            h.key(Key::Named(NamedKey::ArrowRight), Modifiers::empty());
            let _ = h.root.redraw();
            h.deliver();
        }
    }
    h.drag((120.0, 160.0), (420.0, 220.0));
    let _ = h.root.redraw();
    h.deliver();
    let (layers, _) = h.root.redraw();
    let mut r = crate::render::Renderer::new(2);
    r.render(&layers, size.width as u16, size.height as u16, 1.0, theme::background()).unwrap();
    std::fs::write(out.join("reader-selection.png"), r.png()).unwrap();
    h.click(936.0, 30.0);
    let _ = h.root.redraw();
    h.deliver();
    let (layers, _) = h.root.redraw();
    let mut r = crate::render::Renderer::new(2);
    r.render(&layers, size.width as u16, size.height as u16, 1.0, theme::background()).unwrap();
    std::fs::write(out.join("reader-settings.png"), r.png()).unwrap();
}

#[test]
#[ignore]
fn quote_links_open_reader_at_quote() {
    let source_note = std::path::PathBuf::from(std::env::var("AQ_NOTE").unwrap());
    let source_vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").unwrap());
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let doc = std::fs::read_to_string(&source_note).unwrap();
    let book = crate::frontmatter::parse(&doc).and_then(|f| f.text(crate::frontmatter::BOOK_FILE).map(str::to_owned)).unwrap();
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::create_dir_all(vault.join(&book).parent().unwrap()).unwrap();
    std::fs::copy(source_vault.join(&book), vault.join(&book)).unwrap();
    let note = vault.join(source_note.file_name().unwrap());
    std::fs::write(&note, &doc).unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    for (n, line) in doc.lines().filter(|l| crate::reader::quotes::is_reader(l.rsplit_once("](").map_or("", |(_, href)| href))).enumerate() {
        let href = line.rsplit_once("](").unwrap().1.trim_end().trim_end_matches(')');
        assert!(h.app.open_reader_link(&mut h.root, href), "ссылка не открыла читалку: {href}");
        h.deliver();
        assert!(h.reader(), "читалка не открылась");
        h.settle();
        h.settle();
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
        std::fs::write(out.join(format!("quote-{n}.png")), r.png()).unwrap();
        h.escape();
    }
}
