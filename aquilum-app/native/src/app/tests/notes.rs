use super::*;

impl Harness {
    pub(super) fn body_text(&self) -> String {
        let body = self.app.note_ids.as_ref().unwrap().body;
        body.get(&self.root).unwrap().inner().text().to_string()
    }
}

#[test]
fn note_page_hides_frontmatter_renames_and_navigates() {
    use crate::ui::editor::TextAction;
    let mut h = Harness::new();
    let note = h.vault.path().join("b.md");
    std::fs::write(&note, "---\ntype: book\n---\nтекст").unwrap();
    h.app.open(&mut h.root, h.vault.path().join("a.md"));
    h.app.open(&mut h.root, note.clone());
    assert_eq!(h.body_text(), "текст");

    let body = h.app.note_ids.as_ref().unwrap().body.id();
    h.act(body, TextAction::Changed("новый текст".into()));
    assert_eq!(h.app.editor_text, "---\ntype: book\n---\nновый текст");

    let toggle = h.app.note_ids.as_ref().unwrap().metadata.id();
    h.act(toggle, crate::ui::note::MetadataToggled);
    assert_eq!(h.body_text(), "---\ntype: book\n---\nновый текст");
    assert!(h.app.interface.metadata_expanded(&note));
    h.act(toggle, crate::ui::note::MetadataToggled);
    assert_eq!(h.body_text(), "новый текст");

    let ids = h.app.note_ids.as_ref().unwrap();
    let (title, back) = (ids.title.id(), ids.back);
    h.act(title, TextAction::Entered("  ".into()));
    assert_eq!(h.app.note_ids.as_ref().unwrap().title.get(&h.root).unwrap().inner().text().to_string(), "b");
    h.act(title, TextAction::Entered("Книга".into()));
    let renamed = h.vault.path().join("Книга.md");
    for _ in 0..1500 {
        h.app.on_core_events(&mut h.root);
        if h.app.note.as_ref().is_some_and(|n| n.is(&renamed)) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&renamed)));
    assert!(h.app.interface.metadata_expanded(&renamed) == h.app.interface.metadata_expanded(&note));

    let outcome = h.act(back, crate::ui::widgets::Pressed);
    assert!(outcome.title);
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&h.vault.path().join("a.md"))));
    assert!(h.app.navigation.can_forward());

    h.app.toggle_focus_mode(&mut h.root);
    assert!(h.app.interface.focus_mode());
    assert!(!h.app.panel_visible());
}

#[test]
fn home_button_appears_only_for_resolved_home_page() {
    let mut h = Harness::new();
    assert!(h.app.chrome.as_ref().unwrap().home_button.is_none());
    let root = h.vault.path().to_string_lossy().into_owned();
    h.app.workspace.core.ui_state.set_home_page(&root, "a").unwrap();
    h.app.refresh_home();
    for _ in 0..250 {
        h.app.on_core_events(&mut h.root);
        if h.app.home.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(h.app.home.as_ref().is_some_and(|p| p.ends_with("a.md")));
    let button = h.app.chrome.as_ref().unwrap().home_button.expect("кнопка главной");
    h.act(button, crate::ui::widgets::Pressed);
    assert!(h.app.note.as_ref().is_some_and(|n| n.is(&h.vault.path().join("a.md"))));
}

#[test]
fn cover_actions_rewrite_frontmatter() {
    use crate::ui::widgets::Pressed;
    let mut h = Harness::new();
    let note = h.vault.path().join("Книга.md");
    std::fs::write(&note, "---\ncover: true\ntype: book\nPage_cover: pattern:polka\nBook_cover: Files/x.png\n---\nтекст").unwrap();
    h.app.open(&mut h.root, note.clone());
    let hero = |h: &Harness| h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).map(|(_, ids)| (ids.random, ids.remove, ids.thumbnail.map(|t| t.id())));
    let (random, _, _) = hero(&h).expect("обложка и книга");
    h.act(random.unwrap(), Pressed);
    let fields = crate::frontmatter::parse(&h.app.editor_text).unwrap();
    let cover = fields.text(crate::frontmatter::PAGE_COVER).unwrap().to_owned();
    let id = cover.strip_prefix("pattern:").expect("узор записан с префиксом, как в Таури");
    assert!(crate::cover::PATTERN_IDS.contains(&id) && id != "polka");
    assert_eq!(std::fs::read_to_string(&note).unwrap(), h.app.editor_text);

    let (_, remove, _) = hero(&h).unwrap();
    h.act(remove.unwrap(), Pressed);
    assert!(!h.app.editor_text.contains("cover: true") && !h.app.editor_text.contains("Page_cover"));
    let (_, _, thumbnail) = hero(&h).unwrap();
    h.act(thumbnail.unwrap(), crate::ui::book::ThumbAction::Remove);
    assert_eq!(h.app.editor_text, "---\ntype: book\n---\nтекст");
    assert!(hero(&h).is_some_and(|(random, _, _)| random.is_none()));
}

#[test]
#[ignore]
fn real_vault_book_notes_open() {
    let Some(vault) = std::env::var_os("AQUILUM_TEST_VAULT").map(std::path::PathBuf::from) else { return };
    let mut notes = Vec::new();
    let mut stack = vec![vault.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md")
                && let Ok(text) = std::fs::read_to_string(&path)
                && crate::frontmatter::parse(&text).is_some_and(|f| f.has_page_cover() || f.is_book())
            {
                notes.push(path);
            }
        }
    }
    eprintln!("заметок с обложкой или книгой: {}", notes.len());
    for scale in [1.0, 1.25, 1.5] {
        let mut h = Harness::with(Some(&vault), scale);
        let mut renderer = crate::render::Renderer::new(4);
        let (w, hgt) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
        let mut frame = |h: &mut Harness| {
            let (layers, _) = h.root.redraw();
            if let Err(error) = renderer.render(&layers, w, hgt, scale, theme::background()) {
                panic!("кадр не нарисован: {error}");
            }
        };
        for note in &notes {
            eprintln!("{scale} {}", note.display());
            h.app.open(&mut h.root, note.clone());
            frame(&mut h);
            h.pointer_move(300.0 * scale, 150.0 * scale);
            frame(&mut h);
            for _ in 0..30 {
                h.app.on_core_events(&mut h.root);
                frame(&mut h);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }
}

impl Harness {
    pub(super) fn pointer_move(&mut self, x: f64, y: f64) {
        let state = PointerState { position: PhysicalPosition::new(x, y), count: 1, scale_factor: 1.0, ..PointerState::default() };
        self.root.handle_pointer_event(PointerEvent::Move(PointerUpdate { pointer: MOUSE, current: state, coalesced: vec![], predicted: vec![] }));
    }
}

#[test]
fn book_page_renders_with_multithreaded_rasterizer() {
    let mut h = Harness::new();
    let note = h.vault.path().join("Книга.md");
    std::fs::write(&note, "---\ncover: true\ntype: book\nPage_cover: pattern:polka\n---\nтекст").unwrap();
    h.app.open(&mut h.root, note);
    let mut renderer = crate::render::Renderer::new(2);
    for _ in 0..3 {
        h.app.on_core_events(&mut h.root);
        let (layers, _) = h.root.redraw();
        renderer.render(&layers, W as u16, H as u16, 1.0, theme::background()).expect("кадр книжной страницы");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn editing_cover_fields_in_metadata_keeps_rendering() {
    use crate::ui::editor::TextAction;
    let mut h = Harness::new();
    let note = h.vault.path().join("Книга.md");
    let text = "---\ncover: true\ntype: book\nPage_cover: pattern:polka\nBook_cover: Files/x.png\n---\nтекст";
    std::fs::write(&note, text).unwrap();
    h.app.open(&mut h.root, note);
    let toggle = h.app.note_ids.as_ref().unwrap().metadata.id();
    h.act(toggle, crate::ui::note::MetadataToggled);
    let mut renderer = crate::render::Renderer::new(2);
    let mut current = text.to_owned();
    for field in ["Page_cover: pattern:polka\n", "Book_cover: Files/x.png\n", "cover: true\n", "type: book\n"] {
        let start = current.find(field).unwrap();
        for _ in 0..field.chars().count() {
            let ch = current[start..].chars().next().unwrap();
            current.replace_range(start..start + ch.len_utf8(), "");
            let body = h.app.note_ids.as_ref().unwrap().body.id();
            h.act(body, TextAction::Changed(current.clone()));
            h.app.tick(&mut h.root, std::time::Instant::now() + std::time::Duration::from_secs(3600));
            for _ in 0..3 {
                std::thread::sleep(std::time::Duration::from_millis(15));
                h.app.on_core_events(&mut h.root);
                let (layers, _) = h.root.redraw();
                renderer.render(&layers, W as u16, H as u16, 1.0, theme::background()).expect("кадр");
            }
        }
    }
    assert_eq!(h.app.editor_text, "---\n---\nтекст");
}

#[test]
#[ignore]
fn scroll_frame_times() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let scale: f64 = std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.25);
    for (name, text) in [
        ("книга", "---
cover: true
type: book
Page_cover: pattern:ruby
---
"),
        ("обложка", "---
cover: true
Page_cover: pattern:ruby
---
"),
        ("простая", ""),
    ] {
        let mut h = Harness::with(None, scale);
        let note = h.vault.path().join(format!("{name}.md"));
        let body: String = (0..200).map(|i| format!("Строка текста номер {i} для прокрутки страницы.
")).collect();
        std::fs::write(&note, format!("{text}{body}")).unwrap();
        h.app.open(&mut h.root, note);
        let mut renderer = crate::render::Renderer::new(4);
        let (w, hh) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
        let warm = std::time::Instant::now();
        for _ in 0..1000 {
            let finished = crate::cover::take_finished();
            if finished {
                if let Some(cover) = h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
                    cover.edit(&mut h.root, |mut c| crate::ui::book::PageCover::refresh(&mut c));
                }
            }
            let (layers, _) = h.root.redraw();
            renderer.render(&layers, w, hh, scale, theme::background()).unwrap();
            if finished || text.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        eprintln!("{name}: узор готов через {:?}", warm.elapsed());
        let mut total = std::time::Duration::ZERO;
        let mut worst = std::time::Duration::ZERO;
        let frames = 60;
        for i in 0..frames {
            let state = PointerState { position: PhysicalPosition::new(600.0 * scale, 400.0 * scale), count: 1, scale_factor: scale, ..PointerState::default() };
            let dy = if i < frames / 2 { -12.0 } else { 12.0 };
            h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, dy)), state }));
            let started = std::time::Instant::now();
            let (layers, _) = h.root.redraw();
            renderer.draw_frame(&layers, w, hh, scale, theme::background(), i == 0).unwrap();
            let spent = started.elapsed();
            total += spent;
            worst = worst.max(spent);
        }
        eprintln!("{name}: средний кадр {:?}, худший {:?}", total / frames, worst);
    }
}

#[test]
fn cover_disappears_when_its_fields_are_removed() {
    use crate::ui::editor::TextAction;
    let mut h = Harness::new();
    let note = h.vault.path().join("Книга.md");
    let text = "---\ncover: true\nPage_cover: pattern:polka\n---\nтекст";
    std::fs::write(&note, text).unwrap();
    h.app.open(&mut h.root, note);
    let toggle = h.app.note_ids.as_ref().unwrap().metadata.id();
    h.act(toggle, crate::ui::note::MetadataToggled);
    assert!(h.app.note_ids.as_ref().unwrap().hero.is_some());
    let body = h.app.note_ids.as_ref().unwrap().body.id();
    h.act(body, TextAction::Changed("---\nPage_cover: pattern:polka\n---\nтекст".into()));
    assert!(h.app.note_ids.as_ref().unwrap().hero.is_none());
    h.act(body, TextAction::Changed("---\ncover: true\n---\nтекст".into()));
    assert!(h.app.note_ids.as_ref().unwrap().hero.is_some());
    let _ = h.root.redraw();
}

#[test]
fn page_cover_field_suggests_pattern_names() {
    let mut h = Harness::new();
    let note = h.vault.path().join("Обложка.md");
    std::fs::write(&note, "---\nPage_cover:\n---\nтекст").unwrap();
    h.app.interface.set_metadata_expanded(&note, true);
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    let editor = h.app.editor_id().unwrap();
    h.root.handle_text_event(TextEvent::WindowFocusChange(true));
    h.root.focus_on(Some(editor));
    let text = |h: &Harness| h.root.get_widget(editor).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text().to_owned();
    h.key(Key::Named(NamedKey::ArrowDown), Modifiers::empty());
    h.key(Key::Named(NamedKey::End), Modifiers::empty());
    for ch in " tun".chars() {
        h.key(Key::Character(ch.to_string().into()), Modifiers::empty());
    }
    h.app.on_core_events(&mut h.root);
    let _ = h.root.redraw();
    assert!(h.app.suggest_layer.is_some(), "подсказки обложек не появились: {:?}", text(&h));
    h.key(Key::Named(NamedKey::Enter), Modifiers::empty());
    let _ = h.root.redraw();
    h.app.on_core_events(&mut h.root);
    assert_eq!(text(&h), "---\nPage_cover: tunnel\n---\nтекст");
    assert_eq!(crate::cover::pattern_id(Some("tunnel")), Some("tunnel"));
    assert_eq!(crate::cover::pattern_id(Some("pattern:Tunnel")), Some("tunnel"));
}

#[test]
fn page_search_counts_and_steps_through_matches() {
    let mut h = Harness::new();
    let note = h.vault.path().join("Коты.md");
    std::fs::write(&note, "кот и Кот\n\nещё КОТ").unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    h.app.open_find(&mut h.root);
    let _ = h.root.redraw();
    let counter = |h: &Harness| h.app.find.as_ref().unwrap().counter.get(&h.root).unwrap().inner().text().to_owned();
    for ch in "кот".chars() {
        h.key(Key::Character(ch.to_string().into()), Modifiers::empty());
    }
    assert_eq!(counter(&h), "1/3");
    assert!(h.app.find_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::empty()));
    assert_eq!(counter(&h), "2/3");
    assert!(h.app.find_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::SHIFT));
    assert!(h.app.find_key(&mut h.root, &Key::Named(NamedKey::Enter), Modifiers::SHIFT));
    assert_eq!(counter(&h), "3/3");
    assert!(h.app.find_key(&mut h.root, &Key::Named(NamedKey::Escape), Modifiers::empty()));
    assert!(h.app.find.is_none());
}

#[test]
fn pasted_image_is_saved_and_embedded() {
    let mut h = Harness::new();
    let note = h.vault.path().join("Картинки.md");
    std::fs::write(&note, "до").unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(2000, 10, image::Rgba([255, 0, 0, 255])).write_to(&mut png, image::ImageFormat::Png).unwrap();
    h.app.paste_image(&mut h.root, png.get_ref());
    h.deliver();
    let editor = h.app.editor_id().unwrap();
    let text = h.root.get_widget(editor).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).unwrap().inner().text().to_owned();
    let embed = text.lines().find(|l| l.starts_with("![")).unwrap_or_else(|| panic!("нет встраивания: {text:?}"));
    let path = embed.split("](<").nth(1).and_then(|p| p.strip_suffix(">)")).or_else(|| embed.split("](").nth(1).and_then(|p| p.strip_suffix(')'))).unwrap();
    assert!(h.vault.path().join(path).is_file(), "файл не сохранён: {path}");
    let width: f64 = embed[2..].split(['|', ']']).next().unwrap().parse().unwrap();
    assert!(width < 2000.0, "ширина не ограничена колонкой: {embed}");
}

#[test]
fn conflicting_edit_is_kept_as_a_copy() {
    let mut h = Harness::new();
    let path = h.vault.path().join("a.md");
    h.app.open(&mut h.root, path.clone());
    std::fs::write(&path, "снаружи").unwrap();
    h.app.editor_text = "моя правка".into();
    h.app.note.as_mut().unwrap().edit(std::time::Duration::ZERO);
    h.app.tick(&mut h.root, std::time::Instant::now() + std::time::Duration::from_secs(1));
    let copies: Vec<_> = std::fs::read_dir(h.vault.path()).unwrap().flatten().map(|e| e.path()).filter(|p| p.to_string_lossy().contains("конфликт")).collect();
    assert_eq!(copies.len(), 1, "копия-конфликт не записана");
    assert!(std::fs::read_to_string(&copies[0]).unwrap().contains("моя правка"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "снаружи");
    assert!(h.app.editor_text.contains("снаружи"), "{}", h.app.editor_text);
}

#[test]
fn inserted_text_reaches_the_disk() {
    let mut h = Harness::new();
    let path = h.vault.path().join("a.md");
    h.app.open(&mut h.root, path.clone());
    h.app.commit_editor_text(&mut h.root, "---\ntags: []\n---\nтекст".into());
    h.app.tick(&mut h.root, std::time::Instant::now() + std::time::Duration::from_secs(3600));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "---\ntags: []\n---\nтекст");
    assert_eq!(h.app.metadata_state(), Some(true), "вставленные метаданные раскрыты");
}

#[test]
fn unsaved_text_is_written_on_shutdown() {
    let mut h = Harness::new();
    let path = h.vault.path().join("a.md");
    h.app.open(&mut h.root, path.clone());
    h.app.commit_editor_text(&mut h.root, "несохранённая правка".into());
    h.app.shutdown();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "несохранённая правка");
}

#[test]
fn page_search_marks_table_cells_and_counter_fits() {
    let mut h = Harness::new();
    let note = h.vault.path().join("таблица.md");
    std::fs::write(&note, "Слово в тексте.\n\n| Колонка | Ещё |\n| --- | --- |\n| слово тут | нет |\n| и снова слово | да |\n").unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    h.app.open_find(&mut h.root);
    for ch in "слово".chars() {
        h.key(Key::Character(ch.to_string().into()), Modifiers::empty());
    }
    h.settle();
    let counter = h.app.find.as_ref().unwrap().counter.id();
    let width = h.root.get_widget(counter).unwrap().ctx().bounding_box().width();
    assert!(width > 15.0, "счётчик сжат до многоточия: {width}");
    let (layers, _) = h.root.redraw();
    let mut r = crate::render::Renderer::new(2);
    r.render(&layers, W as u16, H as u16, 1.0, theme::background()).unwrap();
    let pink = r.frame_pixels().iter().filter(|&&p| {
        let (red, green, blue) = (((p >> 16) & 0xff) as i32, ((p >> 8) & 0xff) as i32, (p & 0xff) as i32);
        red > green + 30 && red > blue
    }).count();
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join("find-table.png"), r.png()).unwrap();
    }
    assert!(pink > 200, "совпадения в таблице не подсвечены: {pink}");
}

#[test]
fn pdf_export_keeps_what_the_editor_shows() {
    let h = Harness::new();
    let vault = h.vault.path().to_path_buf();
    image::RgbaImage::from_pixel(120, 80, image::Rgba([200, 60, 60, 255])).save(vault.join("pic.png")).unwrap();
    let note = vault.join("Экспорт.md");
    let mut body = "# Заголовок\n\nТекст с **жирным** и *курсивом*, Latin **bold** and *italic*.\n\n- пункт один\n- пункт два\n\n| Колонка | Число |\n|---|---|\n| яблоко | 2 |\n\n![](pic.png)\n\n> цитата\n".to_owned();
    for i in 0..80 {
        body.push_str(&format!("\nParagraph {i}: the quick brown fox jumps over the lazy dog, съешь же ещё этих мягких французских булок.\n"));
    }
    let body = body.as_str();
    std::fs::write(&note, body).unwrap();
    let embeds = crate::app::embeds::Embeds::new(std::sync::Arc::clone(&h.app.workspace.core), vault.clone(), note, std::env::temp_dir(), std::sync::Arc::new(|| {}));
    let pdf = crate::app::note_page::export::render("Экспорт", body, crate::ui::editor::Smart::default(), Some(embeds)).unwrap();
    if let Ok(out) = std::env::var("AQ_OUT") {
        std::fs::write(std::path::Path::new(&out).join("export.pdf"), &pdf).unwrap();
    }
    assert!(pdf.starts_with(b"%PDF"), "не PDF");
}

#[test]
fn typed_edits_reach_the_file_behind_hidden_frontmatter() {
    let mut h = Harness::new();
    let path = h.vault.path().join("правки.md");
    std::fs::write(&path, "---\ntags: [a]\n---\nпривет мир").unwrap();
    h.app.open(&mut h.root, path.clone());
    assert!(h.app.hidden > 0, "метаданные должны быть скрыты");
    let body = h.app.note_ids.as_ref().unwrap().body;
    let end = h.body_text().len();
    h.root.focus_on(Some(body.id()));
    body.edit(&mut h.root, |mut b| crate::ui::editor::TextEditor::select_byte_range(&mut b, end, end));
    for ch in ["!", "ё"] {
        h.key(Key::Character(ch.into()), Modifiers::empty());
    }
    h.key(Key::Named(NamedKey::Backspace), Modifiers::empty());
    body.edit(&mut h.root, |mut b| crate::ui::editor::TextEditor::select_byte_range(&mut b, 0, 0));
    h.key(Key::Character("Ж".into()), Modifiers::empty());
    assert_eq!(h.app.editor_text, "---\ntags: [a]\n---\nЖпривет мир!");
    h.app.tick(&mut h.root, std::time::Instant::now() + std::time::Duration::from_secs(5));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "---\ntags: [a]\n---\nЖпривет мир!");
}
