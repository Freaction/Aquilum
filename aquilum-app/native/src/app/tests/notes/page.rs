use super::*;

#[test]
fn note_page_hides_frontmatter_renames_and_navigates() {
    use crate::ui::editor::TextAction;
    let mut h = Harness::new();
    let note = h.write_note("b.md", "---\ntype: book\n---\nтекст");
    h.app.open(&mut h.root, h.vault.path().join("a.md"));
    h.app.open(&mut h.root, note.clone());
    assert_eq!(h.body_text(), "текст");

    h.replace_body("новый текст");
    assert_eq!(h.app.text.to_string(), "---\ntype: book\n---\nновый текст");

    let toggle = h.app.note_ids.as_ref().unwrap().metadata.id();
    h.act(toggle, crate::ui::note::MetadataToggled);
    assert_eq!(h.body_text(), "---\ntype: book\n---\nновый текст");
    assert!(h.app.interface.metadata_expanded(&note));
    h.act(toggle, crate::ui::note::MetadataToggled);
    assert_eq!(h.body_text(), "новый текст");

    let ids = h.app.note_ids.as_ref().unwrap();
    let (title, back) = (ids.title.id(), ids.back);
    h.act(title, TextAction::Entered("  ".into()));
    assert_eq!(h.app.note_ids.as_ref().unwrap().title.get(&h.root).unwrap().inner().text(), "b");
    h.act(title, TextAction::Entered("Книга".into()));
    let renamed = h.vault.path().join("Книга.md");
    wait_until(1500, 20, || {
        h.app.on_core_events(&mut h.root);
        h.app.shows(&renamed)
    });
    assert!(h.app.shows(&renamed));
    assert!(h.app.interface.metadata_expanded(&renamed) == h.app.interface.metadata_expanded(&note));

    let outcome = h.act(back, crate::ui::widgets::Pressed);
    assert!(outcome.title);
    assert!(h.app.shows(&h.vault.path().join("a.md")));
    assert!(h.app.navigation.can_forward());

    h.app.toggle_focus_mode(&mut h.root);
    assert!(h.app.interface.focus_mode());
    assert!(!h.app.panel_visible());
}

#[test]
fn a_new_note_renamed_from_its_title_stays_open_with_the_caret_in_the_body() {
    use crate::ui::editor::TextAction;
    let mut h = Harness::new();
    h.app.open(&mut h.root, h.vault.path().join("a.md"));
    assert!(h.app.new_note(&mut h.root));
    let tabs = h.app.tabs.tabs().len();
    let ids = h.app.note_ids.as_ref().unwrap();
    let (title, body) = (ids.title.id(), ids.body.id());
    h.act(title, TextAction::Entered("Новая".into()));
    let renamed = h.vault.path().join("Новая.md");
    wait_until(100, 20, || {
        h.app.on_core_events(&mut h.root);
        h.app.shows(&renamed)
    });
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(30));
        h.app.on_core_events(&mut h.root);
    }
    assert_eq!(h.app.tabs.tabs().len(), tabs, "вкладка закрылась");
    assert_eq!(h.app.tabs.active().path.as_deref(), Some(renamed.as_path()));
    assert!(h.app.shows(&renamed));
    assert_eq!(h.root.focused_widget(), Some(body), "каретка не в тексте заметки");
}

#[test]
fn home_button_appears_only_for_resolved_home_page() {
    let mut h = Harness::new();
    assert!(h.app.chrome.as_ref().unwrap().home_button.is_none());
    let root = h.vault.path().to_string_lossy().into_owned();
    h.app.workspace.core.ui_state.set_home_page(&root, "a").unwrap();
    h.app.refresh_home();
    wait_until(250, 20, || {
        h.app.on_core_events(&mut h.root);
        h.app.home.is_some()
    });
    assert!(h.app.home.as_ref().is_some_and(|p| p.ends_with("a.md")));
    let button = h.app.chrome.as_ref().unwrap().home_button.expect("кнопка главной");
    h.act(button, crate::ui::widgets::Pressed);
    assert!(h.app.shows(&h.vault.path().join("a.md")));
}

#[test]
fn pasted_image_is_saved_and_embedded() {
    let mut h = Harness::new();
    let note = h.write_note("Картинки.md", "до");
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(2000, 10, image::Rgba([255, 0, 0, 255])).write_to(&mut png, image::ImageFormat::Png).unwrap();
    let ids = h.app.note_ids.as_ref().unwrap();
    let (title, body) = (ids.title.id(), ids.body.id());
    h.root.focus_on(Some(title));
    assert!(!h.app.paste_image(&mut h.root, png.get_ref()), "картинка вставилась из заголовка");
    h.root.focus_on(Some(body));
    assert!(h.app.paste_image(&mut h.root, png.get_ref()));
    h.deliver();
    let text = h.body_text();
    let embed = text.lines().find(|l| l.starts_with("![")).unwrap_or_else(|| panic!("нет встраивания: {text:?}"));
    let path = embed.split("](<").nth(1).and_then(|p| p.strip_suffix(">)")).or_else(|| embed.split("](").nth(1).and_then(|p| p.strip_suffix(')'))).unwrap();
    assert!(h.vault.path().join(path).is_file(), "файл не сохранён: {path}");
    let width: f64 = embed[2..].split(['|', ']']).next().unwrap().parse().unwrap();
    assert!(width < 2000.0, "ширина не ограничена колонкой: {embed}");
}

#[test]
fn pdf_export_keeps_what_the_editor_shows() {
    let h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    image::RgbaImage::from_pixel(120, 80, image::Rgba([200, 60, 60, 255])).save(vault.join("pic.png")).unwrap();
    let mut body = "# Заголовок\n\nТекст с **жирным** и *курсивом*, Latin **bold** and *italic*.\n\n- пункт один\n- пункт два\n\n| Колонка | Число |\n|---|---|\n| яблоко | 2 |\n\n![](pic.png)\n\n> цитата\n".to_owned();
    for i in 0..80 {
        body.push_str(&format!("\nParagraph {i}: the quick brown fox jumps over the lazy dog, съешь же ещё этих мягких французских булок.\n"));
    }
    let note = h.write_note("Экспорт.md", &body);
    let embeds = crate::app::embeds::Embeds::new(std::sync::Arc::clone(&h.app.workspace.core), vault, note, std::env::temp_dir(), std::sync::Arc::new(|| {}));
    let pdf = crate::app::note_page::export::render("Экспорт", crop::Rope::from(body.as_str()), crate::ui::editor::Smart::default(), Some(embeds)).unwrap();
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join("export.pdf"), &pdf).unwrap();
    }
    assert!(pdf.starts_with(b"%PDF"), "не PDF");
}
