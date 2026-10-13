use super::*;

#[test]
fn cover_actions_rewrite_frontmatter() {
    use crate::ui::widgets::Pressed;
    let mut h = Harness::new();
    let note = h.write_note("Книга.md", "---\ncover: true\ntype: book\nPage_cover: pattern:polka\nBook_cover: Files/x.png\n---\nтекст");
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

#[test]
fn book_page_renders_with_multithreaded_rasterizer() {
    let mut h = Harness::new();
    let note = h.write_note("Книга.md", "---\ncover: true\ntype: book\nPage_cover: pattern:polka\n---\nтекст");
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
    let text = "---\ncover: true\ntype: book\nPage_cover: pattern:polka\nBook_cover: Files/x.png\n---\nтекст";
    let note = h.write_note("Книга.md", text);
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
    let scale = bench_scale(1.25);
    for (name, text) in [
        ("книга", "---\ncover: true\ntype: book\nPage_cover: pattern:ruby\n---\n"),
        ("обложка", "---\ncover: true\nPage_cover: pattern:ruby\n---\n"),
        ("простая", ""),
    ] {
        let mut h = Harness::with(None, scale);
        let body: String = (0..200).map(|i| format!("Строка текста номер {i} для прокрутки страницы.\n")).collect();
        let note = h.write_note(&format!("{name}.md"), &format!("{text}{body}"));
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
            let dy = if i < frames / 2 { -12.0 } else { 12.0 };
            h.scroll((600.0 * scale, 400.0 * scale), scale, (0.0, dy));
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
fn every_pattern_is_painted_from_its_tile_like_from_the_whole_bake() {
    let scale = 1.75;
    for id in ["foliage", "weave", "pineapple", "stripes", "origami", "polka"] {
        let mut h = Harness::with(None, scale);
        let note = h.write_note("обложка.md", &format!("---
cover: true
Page_cover: pattern:{id}
---
текст"));
        h.app.open(&mut h.root, note);
        h.wait_cover(true);
        for _ in 0..8 {
            h.anim(std::time::Duration::from_millis(16));
        }
        let (w, hh) = ((f64::from(W) * scale) as u16, (f64::from(H) * scale) as u16);
        let shot = h.frame(w, hh, scale);
        if let Ok(out) = std::env::var("AQ_OUT") {
            std::fs::write(std::path::Path::new(&out).join(format!("cover-tile-{id}.png")), shot.png()).unwrap();
        }
        let cover = h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover).expect("обложка");
        let bounds = h.root.get_widget(cover.id()).unwrap().ctx().bounding_box();
        let (cw, ch) = ((bounds.width() * scale).round() as u32, (bounds.height() * scale).round() as u32);
        let span = if crate::cover::centered(id) { cw } else { ((f64::from(W) * scale / 128.0).ceil() * 128.0) as u32 };
        let pattern = crate::cover::render_pattern(id, span, ch, scale).expect("узор");
        let expected = crate::cover::expand(&pattern, span, ch);
        let (x0, y0) = ((bounds.x0 * scale).round() as u32, (bounds.y0 * scale).round() as u32);
        let mut checked = 0;
        let mut differ = 0;
        for (dx, dy) in [(40u32, 40u32), (300, 100), (cw / 2, ch / 2), (cw - 60, ch - 60), (97, ch - 30), (cw - 31, 61)] {
            let pixel = shot.frame_pixels()[((y0 + dy) * u32::from(w) + x0 + dx) as usize];
            let at = ((dy * span + dx) * 4) as usize;
            let (r, g, b) = (u32::from(expected[at]), u32::from(expected[at + 1]), u32::from(expected[at + 2]));
            let got = [(pixel >> 16) & 0xff, (pixel >> 8) & 0xff, pixel & 0xff];
            checked += 1;
            differ += usize::from(got.iter().zip([r, g, b]).any(|(a, b)| a.abs_diff(b) > 40));
        }
        assert!(differ <= 1, "{id}: {differ} из {checked} точек обложки не совпадают с узором");
    }
}

#[test]
fn cover_disappears_when_its_fields_are_removed() {
    use crate::ui::editor::TextAction;
    let mut h = Harness::new();
    let note = h.write_note("Книга.md", "---\ncover: true\nPage_cover: pattern:polka\n---\nтекст");
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
