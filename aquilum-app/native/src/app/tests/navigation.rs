use super::*;

#[test]
fn note_reopens_at_saved_scroll_position() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let mut h = Harness::new();
    let long = h.vault.path().join("Длинная.md");
    let body: String = (0..300).map(|i| format!("Абзац {i}\n\n")).collect();
    std::fs::write(&long, &body).unwrap();
    let other = h.vault.path().join("Другая.md");
    std::fs::write(&other, "текст").unwrap();
    h.app.open(&mut h.root, long.clone());
    let _ = h.root.redraw();
    let state = pointer_at(600.0, 300.0, 1.0);
    h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -2500.0)), state }));
    h.deliver();
    let saved = note_offset(&h);
    assert!(saved > 2000.0, "прокрутка не сработала: {saved}");
    h.app.open(&mut h.root, other);
    let _ = h.root.redraw();
    h.app.open(&mut h.root, long);
    let _ = h.root.redraw();
    assert!(h.app.restore_view(&mut h.root), "позиция не восстановлена");
    let _ = h.root.redraw();
    let restored = note_offset(&h);
    assert!((restored - saved).abs() < 1.0, "сохранено {saved}, восстановлено {restored}");
}

#[test]
#[ignore]
fn real_tree_after_focus_mode() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let vault = std::path::PathBuf::from(std::env::var("AQ_VAULT").unwrap());
    let mut h = Harness::with(Some(&vault), 1.0);
    for _ in 0..100 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let shot = |h: &mut Harness, name: &str| {
        for _ in 0..20 {
            h.app.on_core_events(&mut h.root);
            h.deliver();
        }
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
        std::fs::write(out.join(name), r.png()).unwrap();
    };
    shot(&mut h, "tree-before.png");
    h.app.toggle_focus_mode(&mut h.root);
    shot(&mut h, "tree-focus.png");
    h.app.toggle_focus_mode(&mut h.root);
    shot(&mut h, "tree-after.png");
}

#[test]
fn tree_rows_survive_focus_mode() {
    let vault = crate::test_dir::tempdir();
    for i in 0..1000 {
        std::fs::write(vault.path().join(format!("Заметка {i:04}.md")), "текст").unwrap();
    }
    let mut h = Harness::with(Some(vault.path()), 1.0);
    h.app.open(&mut h.root, vault.path().join("Заметка 0900.md"));
    h.deliver();
    let offset = |h: &Harness| scroll_offset(h, h.app.tree.scroll_id().unwrap());
    let before = (offset(&h), h.app.tree.built_rows());
    h.app.toggle_focus_mode(&mut h.root);
    h.deliver();
    h.app.toggle_focus_mode(&mut h.root);
    h.deliver();
    let after = (offset(&h), h.app.tree.built_rows());
    assert_eq!(before, after);
}

#[test]
fn focus_mode_removes_the_rail_and_puts_settings_in_the_top_left_corner() {
    let mut h = Harness::new();
    h.app.open(&mut h.root, h.vault.path().join("a.md"));
    h.settle();
    let (rail, settings, strip, toggle) = {
        let chrome = h.app.chrome.as_ref().unwrap();
        (chrome.rail.id(), chrome.rail_settings, chrome.tab_strip.id(), chrome.toggle_sidebar)
    };
    let bounds = |h: &Harness, id| h.root.get_widget(id).unwrap().ctx().bounding_box();
    let (before, tabs) = (bounds(&h, toggle), bounds(&h, strip));
    assert!(bounds(&h, rail).width() > 0.0, "ряд значков слева не виден до режима фокуса");
    h.app.toggle_focus_mode(&mut h.root);
    h.settle();
    assert_eq!(bounds(&h, rail).width(), 0.0, "ряд значков остался в режиме фокуса");
    let corner = bounds(&h, settings);
    assert_eq!((corner.x0, corner.y0, corner.width(), corner.height()), (before.x0, before.y0, before.width(), before.height()), "настройки не на месте кнопки левого сайдбара");
    assert!(bounds(&h, strip).x0 >= corner.x1, "вкладки наезжают на кнопку настроек");
    h.click(corner.center().x, corner.center().y);
    assert!(h.section().is_some(), "кнопка в углу не открыла настройки");
    h.escape();
    h.app.toggle_focus_mode(&mut h.root);
    h.settle();
    assert!(bounds(&h, rail).width() > 0.0, "ряд значков не вернулся");
    assert!(h.root.get_widget(settings).unwrap().ctx().is_stashed(), "кнопка настроек в углу осталась после выхода из режима");
    assert_eq!(bounds(&h, strip).x0, tabs.x0, "после выхода из режима вкладки сдвинуты");
}

#[test]
fn wiki_link_opens_note_from_subfolder_without_creating_duplicate() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::create_dir_all(vault.join("Base")).unwrap();
    std::fs::write(vault.join("Base").join("📖 Книга.md"), "---\nauthor: А\n---\nтекст").unwrap();
    h.app.open(&mut h.root, vault.join("a.md"));
    let mut opened = false;
    for _ in 0..250 {
        h.app.on_core_events(&mut h.root);
        if h.app.open_target(&mut h.root, crate::links::Target::Wiki("📖 Книга".into()), true) {
            opened = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(opened, "ссылка не открылась");
    assert!(!vault.join("📖 Книга.md").exists(), "создан дубль в корне");
    assert_eq!(h.tab_paths(), [Some("a".to_owned()), Some("📖 Книга".to_owned())]);
}

#[test]
#[ignore]
fn books_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.75;
    let dir = crate::test_dir::tempdir();
    let vault = dir.path().to_path_buf();
    std::fs::create_dir_all(vault.join("Base")).unwrap();
    std::fs::write(vault.join("Base").join("📖 Победители недр.md"), "---\nauthor: Иван Ефремов\npages: 120/300\nПуть к файлу: book.epub\n---\n").unwrap();
    std::fs::write(vault.join("book.epub"), "").unwrap();
    std::fs::write(vault.join("Base").join("📖 Марсианские хроники.md"), "---\nauthor: Рэй Брэдбери\npages: 300/300\n---\n").unwrap();
    std::fs::write(vault.join("Base").join("📖 Очень длинное название книги, которое точно не поместится в одну строку виджета.md"), "").unwrap();
    let note = vault.join("Книги.md");
    std::fs::write(&note, "Текст\n\n> [!book] [[📖 Победители недр]]\n> [!book] [[📖 Марсианские хроники]]\n\n> [!book] [[📖 Очень длинное название книги, которое точно не поместится в одну строку виджета]]\n> [!book] [[📖 Нет такой книги]]\n\nПосле").unwrap();
    let mut h = Harness::with(Some(&vault), scale);
    std::thread::sleep(std::time::Duration::from_secs(2));
    h.app.open(&mut h.root, note);
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(30));
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
    }
    let size = h.root.size();
    for (x, y) in [(1034.0, 457.0), (1034.0, 625.0)] {
        let state = pointer_at(x, y, scale);
        h.root.handle_pointer_event(moved(state));
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).unwrap();
        std::fs::write(out.join(format!("books-{y}.png")), r.png()).unwrap();
    }
}

#[test]
fn book_widgets_and_dataview_follow_changes() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    let book = vault.join("📖 Книга.md");
    std::fs::write(&book, "---
author: Первый
pages: 10/100
---
").unwrap();
    std::fs::write(vault.join("ссылка.md"), "[[📖 Книга]]").unwrap();
    let query = "LIST FROM [[📖 Книга]]";
    let note = vault.join("Главная.md");
    std::fs::write(&note, format!("> [!book] [[📖 Книга]]

```dataview
{query}
```
")).unwrap();
    h.app.open(&mut h.root, note);
    let wait = |h: &mut Harness, done: &dyn Fn(&Harness) -> bool| {
        for _ in 0..300 {
            h.app.on_core_events(&mut h.root);
            let _ = h.root.redraw();
            if done(h) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    };
    let embeds = |h: &Harness| h.app.embeds.as_ref().unwrap().clone();
    let ok = wait(&mut h, &|h| embeds(h).cached_book("📖 Книга").is_some_and(|b| b.pages == Some((10, 100))) && embeds(h).cached_rows(query) == Some(2));
    assert!(ok, "{:?} {:?}", embeds(&h).cached_book("📖 Книга"), embeds(&h).cached_rows(query));
    let core = h.app.workspace.core.clone();
    let snapshot = aquilum_core::files::document::read_file_snapshot_impl(&book).unwrap();
    aquilum_core::files::gate::write(&core, &book, "---
author: Второй
pages: 50/100
---
", Some(&snapshot.hash), aquilum_core::history::Source::Me, None).unwrap();
    assert!(wait(&mut h, &|h| embeds(h).cached_book("📖 Книга").is_some_and(|b| b.pages == Some((50, 100)) && b.author == "Второй")), "виджет книги не обновился");
    aquilum_core::files::gate::create(&core, &vault.join("новая.md"), &"[[📖 Книга]]", aquilum_core::history::Source::Me, None).unwrap();
    assert!(wait(&mut h, &|h| embeds(h).cached_rows(query) == Some(3)), "новая ссылка не попала в таблицу");
}

#[test]
fn tree_rows_return_after_small_scroll_up() {
    let vault = crate::test_dir::tempdir();
    for i in 0..300 {
        std::fs::write(vault.path().join(format!("Заметка {i:04}.md")), "текст").unwrap();
    }
    let mut h = Harness::with(Some(vault.path()), 1.0);
    h.deliver();
    let id = h.app.tree.scroll_id().unwrap();
    let step = crate::ui::tokens::size::SIDEBAR_ITEM_PADDING * 2.0 + crate::ui::tokens::size::SIDEBAR_ICON_SIZE + crate::ui::tokens::size::SPACE_2;
    for rows in [200.0, 45.0, 0.0] {
        let action = masonry::core::ErasedAction::from(Box::new(crate::ui::scroll::Scrolled(rows * step)));
        let _ = h.app.tree.on_action(&mut h.root, id, &action);
        h.deliver();
    }
    assert_eq!(h.app.tree.built_rows().start, 0, "после возврата наверх верхние строки не построены");
}

#[test]
fn note_search_button_sits_left_of_the_menu_opens_the_find_bar_and_hides_in_focus_mode() {
    let mut h = Harness::new();
    h.app.open(&mut h.root, h.vault.path().join("a.md"));
    h.settle();
    let (search, more) = {
        let ids = h.app.note_ids.as_ref().unwrap();
        (ids.search, ids.more)
    };
    let bounds = |h: &Harness, id| h.root.get_widget(id).unwrap().ctx().bounding_box();
    assert!(bounds(&h, search).x1 <= bounds(&h, more).x0, "поиск не слева от троеточия");
    assert!(h.app.find.is_none());
    let center = bounds(&h, search).center();
    h.click(center.x, center.y);
    assert!(h.app.find.is_some(), "кнопка не открыла поиск по заметке");
    h.app.toggle_focus_mode(&mut h.root);
    h.settle();
    assert!(h.root.get_widget(search).unwrap().ctx().is_stashed(), "кнопка поиска осталась в режиме фокуса");
    h.app.toggle_focus_mode(&mut h.root);
    h.settle();
    assert!(!h.root.get_widget(search).unwrap().ctx().is_stashed(), "кнопка поиска не вернулась после режима фокуса");
}

fn long_note(h: &Harness) -> std::path::PathBuf {
    let path = h.vault.path().join("Длинная.md");
    let body: String = (0..400)
        .map(|i| match i % 4 {
            0 => format!("Абзац {i}: {}

", "длинный текст абзаца, который переносится на несколько строк, ".repeat(6)),
            1 => format!("## Заголовок {i}

"),
            _ => format!("Абзац {i}

"),
        })
        .collect();
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn a_restarted_app_reopens_a_long_note_at_the_place_it_was_left() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let mut h = Harness::new();
    let long = long_note(&h);
    h.app.open(&mut h.root, long);
    let _ = h.root.redraw();
    let state = pointer_at(600.0, 300.0, 1.0);
    h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -9000.0)), state }));
    h.settle();
    h.deliver();
    h.app.capture_view(&h.root);
    let saved = h.app.view.expect("позиция запомнена");
    let mut h = h.restarted(1.0);
    h.app.restore(&mut h.root);
    for _ in 0..3 {
        h.app.restore_view(&mut h.root);
        let _ = h.root.redraw();
    }
    h.app.capture_view(&h.root);
    let reopened = h.app.view.expect("позиция после перезапуска");
    assert_eq!(reopened.0, saved.0, "после перезапуска открылась другая часть заметки");
    assert!((reopened.1 - saved.1).abs() < 1.0, "сдвиг внутри абзаца: было {}, стало {}", saved.1, reopened.1);
}
