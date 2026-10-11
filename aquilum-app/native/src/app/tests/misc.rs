use super::*;

#[test]
fn removed_images_leave_caches() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    image::RgbaImage::new(40, 30).save(vault.join("pic.png")).unwrap();
    let note = vault.join("Картинка.md");
    std::fs::write(&note, "текст\n\n![](pic.png)\n\nконец").unwrap();
    h.app.open(&mut h.root, note);
    let cached = |h: &Harness| h.app.embeds.as_ref().unwrap().cached_media();
    for _ in 0..100 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if cached(&h) == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(cached(&h), 2, "картинка не попала в кэш заметки и общий кэш");
    h.app.set_editor_text(&mut h.root, "текст\n\nконец".into());
    let _ = h.root.redraw();
    assert_eq!(cached(&h), 0, "удалённая с холста картинка осталась в кэше");
    h.app.set_editor_text(&mut h.root, "текст\n\n![](pic.png)\n\nконец".into());
    for _ in 0..100 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if cached(&h) == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    h.app.embeds.as_ref().unwrap().forget_paths(&[vault.join("pic.png")]);
    assert_eq!(cached(&h), 0, "удалённый файл остался в кэше");
}

#[test]
fn settings_dropdown_toggles_like_web() {
    let mut h = Harness::new();
    h.click(242.0, 578.0);
    h.click(115.0, 398.0);
    assert_eq!(h.section(), Some(Section::History));
    h.click(816.0, 173.0);
    assert!(h.menu_open());
    if let Ok(out) = std::env::var("AQ_OUT") {
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
        std::fs::write(std::path::Path::new(&out).join("dropdown.png"), r.png()).unwrap();
    }
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
    {
        use masonry::core::{PointerScrollEvent, ScrollDelta};
        let state = PointerState { position: PhysicalPosition::new(600.0, 400.0), count: 1, scale_factor: 1.0, ..PointerState::default() };
        h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -420.0)), state }));
        h.deliver();
    }
    let (layers, _) = h.root.redraw();
    let mut r = crate::render::Renderer::new(2);
    r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
    std::fs::write(out.join("mcp.png"), r.png()).unwrap();
}

#[test]
#[ignore]
fn book_page_snapshot() {
    let out = std::path::PathBuf::from(std::env::var("AQ_OUT").unwrap());
    let scale = 1.0;
    let mut h = Harness::with(None, scale);
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("book.epub"), "").unwrap();
    std::fs::write(vault.join("С обложкой.md"), "---\ncover: true\ntype: book\nPage_cover: pattern:tunnel\nПуть к файлу: book.epub\n---\n\nТекст").unwrap();
    std::fs::write(vault.join("Без обложки.md"), "---\ntype: book\n---\n\nТекст").unwrap();
    for name in ["С обложкой", "Без обложки"] {
        h.app.open(&mut h.root, vault.join(format!("{name}.md")));
        let _ = h.root.redraw();
        let size = h.root.size();
        let (layers, _) = h.root.redraw();
        let mut r = crate::render::Renderer::new(2);
        r.render(&layers, size.width as u16, size.height as u16, scale, theme::background()).unwrap();
        std::fs::write(out.join(format!("{name}.png")), r.png()).unwrap();
    }
}

#[test]
fn mcp_navigation_opens_notes_and_tracks_active_note() {
    let mut h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("b.md"), "текст").unwrap();
    h.app.open(&mut h.root, vault.join("a.md"));
    assert_eq!(h.app.workspace.core.active_note.get(), Some(vault.join("a.md")));
    aquilum_core::mcp::bridge::open_note(&h.app.workspace.core, &vault.join("b.md"), true);
    for _ in 0..50 {
        h.app.on_core_events(&mut h.root);
        if h.tab_paths().len() == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(h.tab_paths(), [Some("a".to_owned()), Some("b".to_owned())]);
    assert_eq!(h.app.workspace.core.active_note.get(), Some(vault.join("b.md")));
}

#[test]
fn editor_font_and_column_follow_settings() {
    let mut h = Harness::new();
    h.app.open(&mut h.root, h.vault.path().join("a.md"));
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
    let vault = h.vault.path().to_path_buf();
    std::fs::write(vault.join("Есть.md"), "текст").unwrap();
    let note = vault.join("Ссылки.md");
    let text = "[[Есть]] и [[Нет такой]]";
    std::fs::write(&note, text).unwrap();
    h.app.open(&mut h.root, note);
    let editor = h.app.editor_id().unwrap();
    let brush = |h: &Harness, at: usize| h.root.get_widget(editor).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().brush_at(at);
    let (present, missing) = (text.find("Есть").unwrap(), text.find("Нет").unwrap());
    for _ in 0..500 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
        if brush(&h, missing) != brush(&h, present) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(brush(&h, present).is_some());
    assert_ne!(brush(&h, missing), brush(&h, present), "ссылка на несуществующую заметку не приглушена");
}

#[test]
fn trimmed_renderer_draws_the_same_frame() {
    let mut h = Harness::new();
    let note = h.vault.path().join("Картинка.md");
    image::RgbaImage::from_pixel(40, 30, image::Rgba([10, 120, 200, 255])).save(h.vault.path().join("pic.png")).unwrap();
    std::fs::write(&note, "# Заголовок\n\nТекст **жирный**.\n\n![](pic.png)\n").unwrap();
    h.app.open(&mut h.root, note);
    for _ in 0..100 {
        h.app.on_core_events(&mut h.root);
        let _ = h.root.redraw();
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
    let note = h.vault.path().join("длинная.md");
    std::fs::write(&note, &body).unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    for size in [(1, 1), (2880, 1716), (276, 45), (2880, 1716)] {
        h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(size.0, size.1)));
        let _ = h.root.redraw();
        let _ = h.root.redraw();
    }
    assert_eq!(h.body_text(), body);
}
