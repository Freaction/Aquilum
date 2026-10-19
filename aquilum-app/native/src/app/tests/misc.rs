use super::*;

fn cached(h: &Harness) -> usize {
    h.app.embeds.as_ref().unwrap().cached_media()
}

fn image_note(h: &Harness) -> std::path::PathBuf {
    image::RgbaImage::new(40, 30).save(h.vault.path().join("pic.png")).unwrap();
    h.write_note("Картинка.md", "текст\n\n![](pic.png)\n\nконец")
}

fn wait_cached(h: &mut Harness) {
    wait_until(100, 10, || {
        h.step();
        cached(h) == 2
    });
}

#[test]
fn removed_images_leave_caches() {
    let mut h = Harness::new();
    let note = image_note(&h);
    h.open(note);
    wait_cached(&mut h);
    assert_eq!(cached(&h), 2, "картинка не попала в кэш заметки и общий кэш");
    h.app.set_text(&mut h.root, crop::Rope::from("текст\n\nконец"));
    let _ = h.root.redraw();
    assert_eq!(cached(&h), 0, "удалённая с холста картинка осталась в кэше");
    h.app.set_text(&mut h.root, crop::Rope::from("текст\n\n![](pic.png)\n\nконец"));
    wait_cached(&mut h);
    h.app.embeds.as_ref().unwrap().forget_paths(&[h.vault.path().join("pic.png")]);
    assert_eq!(cached(&h), 0, "удалённый файл остался в кэше");
}

#[test]
fn leaving_the_note_lets_go_of_its_embeds() {
    let mut h = Harness::new();
    let note = image_note(&h);
    h.open(note);
    wait_cached(&mut h);
    assert_eq!(cached(&h), 2, "картинка не загрузилась");
    let alive = h.app.embeds.as_ref().unwrap().watch();
    let _ = h.app.new_tab(&mut h.root);
    let _ = h.root.redraw();
    assert!(!alive(), "вставки ушедшей заметки остались в памяти");
}

#[test]
fn settings_dropdown_toggles_like_web() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    h.open_section(Section::History);
    assert_eq!(h.section(), Some(Section::History));
    h.click(816.0, 173.0);
    assert!(h.menu_open());
    h.snapshot("dropdown.png");
    h.click(816.0, 173.0);
    assert!(!h.menu_open(), "повторный щелчок по полю не закрыл список");
    h.click(816.0, 173.0);
    assert!(h.menu_open());
    h.escape();
    assert!(!h.menu_open());
    assert_eq!(h.section(), Some(Section::History));
}

#[test]
#[ignore]
fn mcp_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    h.click(115.0, 364.0);
    assert_eq!(h.section(), Some(Section::Mcp));
    assert!(!h.config().mcp.enabled);
    assert!(h.config().mcp.token.is_empty(), "токен не создаётся при открытии настроек");
    h.scroll((600.0, 400.0), 1.0, (0.0, -420.0));
    h.deliver();
    std::fs::write(out.join("mcp.png"), h.frame(W as u16, H as u16, 1.0).png()).unwrap();
}

#[test]
#[ignore]
fn book_page_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.0;
    let mut h = Harness::with(None, scale);
    h.write_note("book.epub", "");
    h.write_note("С обложкой.md", "---\ncover: true\ntype: book\nPage_cover: pattern:tunnel\nПуть к файлу: book.epub\n---\n\nТекст");
    h.write_note("Без обложки.md", "---\ntype: book\n---\n\nТекст");
    for name in ["С обложкой", "Без обложки"] {
        h.open(h.vault.path().join(format!("{name}.md")));
        let _ = h.root.redraw();
        let size = h.root.size();
        std::fs::write(out.join(format!("{name}.png")), h.frame(size.width as u16, size.height as u16, scale).png()).unwrap();
    }
}

#[test]
fn mcp_navigation_opens_notes_and_tracks_active_note() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    h.write_note("b.md", "текст");
    h.open(vault.join("a.md"));
    assert_eq!(h.app.workspace.core.active_note.get(), Some(vault.join("a.md")));
    aquilum_core::mcp::bridge::open_note(&h.app.workspace.core, &vault.join("b.md"), true);
    wait_until(50, 10, || {
        h.app.on_core_events(&mut h.root);
        h.tab_paths().len() == 2
    });
    h.parked();
    assert_eq!(h.tab_paths(), [Some("a".to_owned()), Some("b".to_owned())]);
    assert_eq!(h.app.workspace.core.active_note.get(), Some(vault.join("b.md")));
}

#[test]
fn editor_font_and_column_follow_settings() {
    let mut h = Harness::new();
    h.open(h.vault.path().join("a.md"));
    let _ = h.root.redraw();
    let mut config = h.config();
    config.editor.font.font_size_base = 22;
    config.editor.max_width_ch = 50;
    config.editor.line_height = 1.8;
    h.app.workspace.core.settings.update_config(config).unwrap();
    h.app.rebuild(&mut h.root);
    let _ = h.root.redraw();
    let font = theme::editor_font_settings();
    assert_eq!((font.size, font.max_width_ch, font.line_height), (22.0, 50, 1.8));
}

#[test]
fn unresolved_wiki_links_are_dimmed_like_tauri() {
    let mut h = Harness::new();
    h.write_note("Есть.md", "текст");
    let text = "[[Есть]] и [[Нет такой]]";
    let note = h.write_note("Ссылки.md", text);
    h.open(note);
    let editor = h.app.editor_id().unwrap();
    let brush = |h: &Harness, at: usize| h.root.get_widget(editor).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().brush_at(at);
    let (present, missing) = (text.find("Есть").unwrap(), text.find("Нет").unwrap());
    wait_until(500, 10, || {
        h.step();
        brush(&h, missing) != brush(&h, present)
    });
    assert!(brush(&h, present).is_some());
    assert_ne!(brush(&h, missing), brush(&h, present), "ссылка на несуществующую заметку не приглушена");
}

#[test]
fn trimmed_renderer_draws_the_same_frame() {
    let mut h = Harness::new();
    image::RgbaImage::from_pixel(40, 30, image::Rgba([10, 120, 200, 255])).save(h.vault.path().join("pic.png")).unwrap();
    let note = h.write_note("Картинка.md", "# Заголовок\n\nТекст **жирный**.\n\n![](pic.png)\n");
    h.open(note);
    for _ in 0..100 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let (layers, _) = h.root.redraw();
    let mut renderer = crate::render::Renderer::new(2);
    renderer.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
    let before = renderer.png();
    renderer.trim();
    renderer.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
    assert!(before == renderer.png(), "после сброса кадр отличается");
}

#[test]
fn tiny_window_and_back_keeps_the_note_consistent() {
    let mut h = Harness::with(None, 1.75);
    h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(2880, 1716)));
    let body: String = (0..400).map(|i| if i % 5 == 0 { format!("Длинный абзац {i}: {}\n\n", "слово ".repeat(80)) } else { format!("- пункт {i}\n") }).collect();
    let note = h.write_note("длинная.md", &body);
    h.open(note);
    let _ = h.root.redraw();
    for size in [(1, 1), (2880, 1716), (276, 45), (2880, 1716)] {
        h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(size.0, size.1)));
        let _ = h.root.redraw();
        let _ = h.root.redraw();
    }
    assert_eq!(h.body_text(), body);
}
