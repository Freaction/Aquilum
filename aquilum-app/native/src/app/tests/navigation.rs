use super::*;

#[test]
fn wiki_link_opens_note_from_subfolder_without_creating_duplicate() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::create_dir_all(vault.join("Base")).unwrap();
    std::fs::write(vault.join("Base").join("📖 Книга.md"), "---\nauthor: А\n---\nтекст").unwrap();
    h.open(vault.join("a.md"));
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
    h.open(note);
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(30));
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
    }
    let size = h.root.size();
    for (x, y) in [(1034.0, 457.0), (1034.0, 625.0)] {
        let state = pointer_at(x, y, scale);
        h.root.handle_pointer_event(moved(MOUSE, state));
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
    h.open(note);
    let embeds = |h: &Harness| h.app.embeds.as_ref().unwrap().clone();
    let ok = h.wait_for(|h| embeds(h).cached_book("📖 Книга").is_some_and(|b| b.pages == Some((10, 100))) && embeds(h).cached_rows(query) == Some(2));
    assert!(ok, "{:?} {:?}", embeds(&h).cached_book("📖 Книга"), embeds(&h).cached_rows(query));
    let core = h.app.workspace.core.clone();
    let hash = aquilum_core::files::document::hash_file(&book).unwrap();
    aquilum_core::files::gate::write(&core, &book, "---
author: Второй
pages: 50/100
---
", Some(&hash), aquilum_core::history::Source::Me, None).unwrap();
    assert!(h.wait_for(|h| embeds(h).cached_book("📖 Книга").is_some_and(|b| b.pages == Some((50, 100)) && b.author == "Второй")), "виджет книги не обновился");
    aquilum_core::files::gate::create(&core, &vault.join("новая.md"), &"[[📖 Книга]]", aquilum_core::history::Source::Me, None).unwrap();
    assert!(h.wait_for(|h| embeds(h).cached_rows(query) == Some(3)), "новая ссылка не попала в таблицу");
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
