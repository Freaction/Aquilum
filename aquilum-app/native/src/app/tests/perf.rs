use super::*;

#[test]
#[ignore]
fn frame_parts() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let (w, hh) = bench_size().unwrap_or((W, H));
    let files: usize = std::env::var("AQUILUM_BENCH_FILES").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let vault = crate::test_dir::tempdir();
    for i in 0..files.max(1) {
        std::fs::write(vault.path().join(format!("Заметка {i:04}.md")), "текст").unwrap();
    }
    let scale: f64 = std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let mut h = Harness::with(Some(vault.path()), scale);
    h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
    let note = vault.path().join("длинная.md");
    let body: String = (0..400).map(|i| format!("Абзац {i}: плотный текст заметки, как в настоящей базе знаний, с длинными предложениями, которые переносятся на несколько строк и заполняют всю ширину колонки редактора от края до края. ")).collect();
    let head = if std::env::var_os("AQUILUM_BENCH_HERO").is_some() { "---
cover: true
type: book
Page_cover: pattern:ruby
---
" } else { "" };
    std::fs::write(&note, format!("{head}{body}")).unwrap();
    h.app.open(&mut h.root, note);
    for _ in 0..500 {
        if crate::cover::take_finished() {
            if let Some(cover) = h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
                cover.edit(&mut h.root, |mut c| crate::ui::book::PageCover::refresh(&mut c));
            }
            break;
        }
        if head.is_empty() {
            break;
        }
        let _ = h.root.redraw();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let threads: u16 = std::env::var("AQUILUM_BENCH_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(std::thread::available_parallelism().map_or(4, |n| n.get() as u16 - 1));
    let mut renderer = crate::render::Renderer::new(threads);
    let (mut scene, mut raster) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    let frames = 60u32;
    for i in 0..frames {
        let state = PointerState { position: PhysicalPosition::new(f64::from(w) / 2.0, f64::from(hh) / 2.0), count: 1, scale_factor: scale, ..PointerState::default() };
        let dy = if i < frames / 2 { -7.0 } else { 7.0 };
        h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, dy)), state }));
        let t = std::time::Instant::now();
        let (layers, _) = h.root.redraw();
        let t1 = std::time::Instant::now();
        renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), i == 0).unwrap();
        let t2 = std::time::Instant::now();
        scene += t1 - t;
        raster += t2 - t1;
    }
    eprintln!("{w}x{hh}: сцена {:?}, растр {:?}, всего {:?}", scene / frames, raster / frames, (scene + raster) / frames);
}


#[test]
fn long_tree_builds_only_rows_near_the_view() {
    let vault = crate::test_dir::tempdir();
    for i in 0..1000 {
        std::fs::write(vault.path().join(format!("Заметка {i:04}.md")), "текст").unwrap();
    }
    let mut h = Harness::with(Some(vault.path()), 1.0);
    assert!(h.app.tree.built_rows().len() <= 160);
    h.app.open(&mut h.root, vault.path().join("Заметка 0900.md"));
    assert!(h.app.tree.built_rows().contains(&900));
    let scroll = h.app.tree.scroll_id().unwrap();
    h.act(scroll, crate::ui::scroll::Scrolled(0.0));
    assert_eq!(h.app.tree.built_rows().start, 0);
    let _ = h.root.redraw();
}

#[test]
#[ignore]
fn core_events_cost() {
    let Some(source) = std::env::var_os("AQUILUM_TEST_VAULT").map(std::path::PathBuf::from) else { return };
    let mut h = Harness::with(Some(&source), 1.75);
    let started = std::time::Instant::now();
    let mut worst = (std::time::Duration::ZERO, 0.0);
    let mut total = std::time::Duration::ZERO;
    while started.elapsed() < std::time::Duration::from_secs(8) {
        let t = std::time::Instant::now();
        h.app.on_core_events(&mut h.root);
        let (layers, _) = h.root.redraw();
        drop(layers);
        let spent = t.elapsed();
        total += spent;
        if spent > worst.0 {
            worst = (spent, started.elapsed().as_secs_f64());
        }
        if spent > std::time::Duration::from_millis(30) {
            eprintln!("{:.2} с: обработка событий ядра {:?}", started.elapsed().as_secs_f64(), spent);
        }
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
    eprintln!("всего на главном потоке {:?}, худшее {:?} на {:.2} с", total, worst.0, worst.1);
}


#[test]
fn partial_frames_match_full_frames() {
    partial_frames(1.0, W, H, 40);
}

#[test]
fn partial_frames_match_full_frames_at_fractional_size() {
    partial_frames(1.75, 1750, 1051, 24);
}

fn partial_frames(scale: f64, width: u32, height: u32, steps: u32) {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let mut h = Harness::with(None, scale);
    h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(width, height)));
    h.app.window_resized(&mut h.root, Size::new(f64::from(width) / scale, f64::from(height) / scale));
    let note = h.vault.path().join("Длинная.md");
    let body: String = (0..120).map(|i| format!("Абзац {i}: текст заметки, который переносится на несколько строк в колонке редактора.\n\n")).collect();
    std::fs::write(&note, format!("---\ncover: true\ntype: book\nPage_cover: pattern:polka\n---\n{body}")).unwrap();
    h.app.open(&mut h.root, note);
    for _ in 0..500 {
        if crate::cover::take_finished() {
            if let Some(cover) = h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
                cover.edit(&mut h.root, |mut c| crate::ui::book::PageCover::refresh(&mut c));
            }
            break;
        }
        let _ = h.root.redraw();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let (w, hh) = (width as u16, height as u16);
    let mut partial = crate::render::Renderer::new(2);
    let mut full = crate::render::Renderer::new(2);
    let mut a = vec![0u32; (width * height) as usize];
    let mut b = vec![0u32; (width * height) as usize];
    for i in 0..steps {
        if i % 4 == 3 {
            h.pointer_move(f64::from(40 + i * 7), f64::from(80 + i * 11));
        } else {
            let state = PointerState { position: PhysicalPosition::new(f64::from(width) * 0.6, f64::from(height) * 0.5), count: 1, scale_factor: scale, ..PointerState::default() };
            let dy = if i < steps * 3 / 5 { -37.0 } else { 23.0 };
            h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, dy)), state }));
        }
        h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_millis(16)));
        let (layers, _) = h.root.redraw();
        partial.draw_frame(&layers, w, hh, scale, theme::background(), i == 0).unwrap();
        a.copy_from_slice(partial.frame_pixels());
        full.render(&layers, w, hh, scale, theme::background()).unwrap();
        b.copy_from_slice(full.frame_pixels());
        let channel = |p: u32, shift: u32| ((p >> shift) & 0xff) as i32;
        let differ = a.iter().zip(&b).filter(|&(&x, &y)| [0, 8, 16].iter().any(|&s| (channel(x, s) - channel(y, s)).abs() > 2)).count();
        assert_eq!(differ, 0, "шаг {i}: частичная отрисовка расходится с полной в {differ} пикселях");
    }
    assert!(partial.shifts > steps as usize / 2, "прокрутка сдвигает пиксели, а не перерисовывает колонку: сдвигов {}", partial.shifts);
}

#[test]
#[ignore]
fn typing_cost() {
    let source = std::path::PathBuf::from(std::env::var("AQ_NOTE").unwrap());
    aq_trace::start();
    let mut h = Harness::with(None, 1.75);
    let note = h.vault.path().join("заметка.md");
    std::fs::copy(&source, &note).unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    h.root.focus_on(h.app.editor_id());
    let _ = h.root.redraw();
    h.key(Key::Named(NamedKey::End), Modifiers::CONTROL);
    let before = h.app.editor_text.len();
    let mut renderer = crate::render::Renderer::new(4);
    let (w, hh) = ((f64::from(W) * 1.75) as u16, (f64::from(H) * 1.75) as u16);
    let (mut event, mut deliver, mut scene, mut raster) = (std::time::Duration::ZERO, std::time::Duration::ZERO, std::time::Duration::ZERO, std::time::Duration::ZERO);
    let n = 30u32;
    for i in 0..n {
        let t0 = std::time::Instant::now();
        let ev = KeyboardEvent { state: KeyState::Down, key: Key::Named(NamedKey::Backspace), code: Code::Backspace, location: Location::Standard, modifiers: Modifiers::empty(), repeat: i > 0, is_composing: false };
        h.root.handle_text_event(TextEvent::Keyboard(ev));
        let t1 = std::time::Instant::now();
        h.deliver();
        let t2 = std::time::Instant::now();
        let (layers, _) = h.root.redraw();
        let t3 = std::time::Instant::now();
        renderer.draw_frame(&layers, w, hh, 1.75, theme::background(), i == 0).unwrap();
        let t4 = std::time::Instant::now();
        event += t1 - t0;
        deliver += t2 - t1;
        scene += t3 - t2;
        raster += t4 - t3;
    }
    if let Ok(out) = std::env::var("AQ_TRACE") {
        aq_trace::write(std::path::Path::new(&out)).unwrap();
    }
    eprintln!("удалено байт {}; нажатие: событие {:?}, действия {:?}, сцена {:?}, растр {:?}", before - h.app.editor_text.len(), event / n, deliver / n, scene / n, raster / n);
}

#[test]
#[ignore]
fn resize_cost() {
    use masonry::core::WindowEvent as MasonryWindowEvent;
    let source = std::path::PathBuf::from(std::env::var("AQ_NOTE").unwrap());
    let scale = 1.75;
    let mut h = Harness::with(None, scale);
    let note = h.vault.path().join("заметка.md");
    std::fs::copy(&source, &note).unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
    let mut renderer = crate::render::Renderer::new(4);
    let (mut scene, mut raster) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    let n = 30u32;
    for i in 0..n {
        let (w, hh) = (2400 - i * 8, 1500 - i * 4);
        let t0 = std::time::Instant::now();
        h.root.handle_window_event(MasonryWindowEvent::Resize(masonry::dpi::PhysicalSize::new(w, hh)));
        h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
        h.deliver();
        let (layers, _) = h.root.redraw();
        let t1 = std::time::Instant::now();
        renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), true).unwrap();
        scene += t1 - t0;
        raster += t1.elapsed();
    }
    eprintln!("шаг изменения размера: сцена {:?}, растр {:?}", scene / n, raster / n);
}

#[test]
fn live_cover_frames_match_full_frames() {
    let mut h = Harness::new();
    let note = h.vault.path().join("Туннель.md");
    std::fs::write(&note, "---\ncover: true\nPage_cover: tunnel\n---\nТекст под обложкой.").unwrap();
    h.app.open(&mut h.root, note);
    for _ in 0..500 {
        if crate::cover::take_finished() {
            if let Some(cover) = h.app.note_ids.as_ref().and_then(|ids| ids.hero.as_ref()).and_then(|(_, ids)| ids.cover) {
                cover.edit(&mut h.root, |mut c| crate::ui::book::PageCover::refresh(&mut c));
            }
            break;
        }
        let _ = h.root.redraw();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let (w, hh) = (W as u16, H as u16);
    h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_secs(1)));
    let (layers, _) = h.root.redraw();
    let mut live = crate::render::Renderer::new(2);
    crate::render::live::freeze(Some(0.0));
    live.draw_frame(&layers, w, hh, 1.0, theme::background(), true).unwrap();
    assert!(live.has_live(), "обложка не стала живым слоем");
    for step in 1..6 {
        let t = f64::from(step) * 0.137;
        crate::render::live::freeze(Some(t));
        assert!(!live.live_frame(&layers).unwrap().is_empty(), "шаг {step}: живой слой не перерисован");
        let mut full = crate::render::Renderer::new(2);
        full.render(&layers, w, hh, 1.0, theme::background()).unwrap();
        let channel = |p: u32, shift: u32| ((p >> shift) & 0xff) as i32;
        if let Ok(out) = std::env::var("AQ_OUT") {
            std::fs::write(std::path::Path::new(&out).join(format!("live-{step}.png")), live.png()).unwrap();
            std::fs::write(std::path::Path::new(&out).join(format!("full-{step}.png")), full.png()).unwrap();
        }
        let differ = live.frame_pixels().iter().zip(full.frame_pixels()).filter(|&(&x, &y)| [0, 8, 16].iter().any(|&s| (channel(x, s) - channel(y, s)).abs() > 2)).count();
        assert_eq!(differ, 0, "шаг {step}: живой слой расходится с полной отрисовкой в {differ} пикселях");
    }
    crate::render::live::freeze(None);
}

#[test]
#[ignore]
fn panel_animation_frames() {
    let (w, hh) = bench_size().unwrap_or((W, H));
    let scale: f64 = std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let vault = crate::test_dir::tempdir();
    let note = vault.path().join("длинная.md");
    let body: String = (0..400).map(|i| format!("Абзац {i}: плотный текст заметки с длинными предложениями, которые переносятся на несколько строк.\n\n")).collect();
    std::fs::write(&note, body).unwrap();
    let mut h = Harness::with(Some(vault.path()), scale);
    h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
    h.app.open(&mut h.root, note);
    let mut renderer = crate::render::Renderer::new(std::thread::available_parallelism().map_or(4, |n| n.get() as u16 - 1));
    let (layers, _) = h.root.redraw();
    renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), true).unwrap();
    for name in ["появление", "фокус", "выход", "панель"] {
        match name {
            "появление" => {}
            "панель" => {
                let toggle = h.app.chrome.as_ref().unwrap().toggle_sidebar;
                h.act(toggle, crate::ui::widgets::Pressed);
            }
            _ => h.app.toggle_focus_mode(&mut h.root),
        }
        let (mut worst, mut total, mut frames) = (std::time::Duration::ZERO, std::time::Duration::ZERO, 0u32);
        for _ in 0..40 {
            let t = std::time::Instant::now();
            h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_micros(8333)));
            let (layers, _) = h.root.redraw();
            renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), false).unwrap();
            let spent = t.elapsed();
            worst = worst.max(spent);
            total += spent;
            frames += 1;
        }
        eprintln!("{name}: средний кадр {:?}, худший {worst:?}", total / frames);
    }
}

#[test]
#[ignore]
fn modal_intro_frames() {
    let (w, hh) = bench_size().unwrap_or((W, H));
    let scale: f64 = std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let vault = crate::test_dir::tempdir();
    let note = vault.path().join("заметка.md");
    let body: String = (0..60).map(|i| format!("Абзац {i}: плотный текст заметки с длинными предложениями, которые переносятся на несколько строк.\n\n")).collect();
    std::fs::write(&note, body).unwrap();
    let mut h = Harness::with(Some(vault.path()), scale);
    h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
    h.app.open(&mut h.root, note);
    let mut renderer = crate::render::Renderer::new(std::thread::available_parallelism().map_or(4, |n| n.get() as u16 - 1));
    let (layers, _) = h.root.redraw();
    renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), true).unwrap();
    let t = std::time::Instant::now();
    h.app.open_settings(&mut h.root);
    let built = t.elapsed();
    let mut frames = Vec::new();
    for i in 0..30 {
        let t = std::time::Instant::now();
        if i > 0 {
            h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_micros(8333)));
        }
        let (layers, _) = h.root.redraw();
        let t1 = std::time::Instant::now();
        renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), false).unwrap();
        frames.push((t1 - t, t1.elapsed()));
    }
    eprintln!("{w}x{hh}@{scale}: сборка окна {built:?}");
    for (i, (scene, raster)) in frames.iter().enumerate() {
        eprintln!("кадр {i}: сцена {scene:?}, растр {raster:?}");
    }
}

#[test]
fn modal_intro_leaves_frames_like_full_redraw() {
    let channel = |p: u32, shift: u32| ((p >> shift) & 0xff) as i32;
    let differ = |a: &[u32], b: &[u32]| a.iter().zip(b).filter(|&(&x, &y)| [0, 8, 16].iter().any(|&s| (channel(x, s) - channel(y, s)).abs() > 2)).count();
    for close_early in [false, true] {
        let mut h = Harness::new();
        let note = h.vault.path().join("заметка.md");
        std::fs::write(&note, "Текст заметки под окном настроек.").unwrap();
        h.app.open(&mut h.root, note);
        let (w, hh) = (W as u16, H as u16);
        let mut renderer = crate::render::Renderer::new(2);
        let (layers, _) = h.root.redraw();
        renderer.draw_frame(&layers, w, hh, 1.0, theme::background(), true).unwrap();
        let corner = renderer.frame_pixels()[usize::from(w) * 4 + 4];
        h.app.open_settings(&mut h.root);
        for step in 0..40 {
            h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_micros(8333)));
            if close_early && step == 6 {
                h.key(Key::Named(NamedKey::Escape), Modifiers::empty());
            }
            let (layers, _) = h.root.redraw();
            renderer.draw_frame(&layers, w, hh, 1.0, theme::background(), false).unwrap();
            if let Ok(out) = std::env::var("AQ_OUT").map(std::path::PathBuf::from)
                && !close_early
            {
                std::fs::write(out.join(format!("modal-{step:02}.png")), renderer.png()).unwrap();
            }
            if step == 12 && !close_early {
                let dimmed = renderer.frame_pixels()[usize::from(w) * 4 + 4];
                assert!(channel(dimmed, 8) < channel(corner, 8), "затемнение не наложено на середине появления");
            }
        }
        let (layers, _) = h.root.redraw();
        let mut full = crate::render::Renderer::new(2);
        full.render(&layers, w, hh, 1.0, theme::background()).unwrap();
        let wrong = differ(renderer.frame_pixels(), full.frame_pixels());
        assert_eq!(wrong, 0, "после появления (закрыто рано: {close_early}) кадр расходится с полной отрисовкой в {wrong} пикселях");
    }
}

#[test]
#[ignore]
fn settings_scroll_frames() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let (w, hh) = bench_size().unwrap_or((W, H));
    let scale: f64 = std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let mut h = Harness::with(None, scale);
    h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
    h.app.open_settings(&mut h.root);
    let mut renderer = crate::render::Renderer::new(std::thread::available_parallelism().map_or(4, |n| n.get() as u16 - 1));
    for _ in 0..40 {
        h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_micros(8333)));
        let (layers, _) = h.root.redraw();
        renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), false).unwrap();
    }
    let (mut scene, mut raster, mut worst, mut shifts) = (std::time::Duration::ZERO, std::time::Duration::ZERO, std::time::Duration::ZERO, 0usize);
    let frames = 60u32;
    for i in 0..frames {
        let state = PointerState { position: PhysicalPosition::new(f64::from(w) * 0.6, f64::from(hh) / 2.0), count: 1, scale_factor: scale, ..PointerState::default() };
        let dy = if i < frames / 2 { -9.0 } else { 9.0 };
        h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, dy)), state }));
        let t = std::time::Instant::now();
        let (layers, _) = h.root.redraw();
        let t1 = std::time::Instant::now();
        let before = renderer.shifts;
        renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), false).unwrap();
        shifts += renderer.shifts - before;
        let t2 = std::time::Instant::now();
        scene += t1 - t;
        raster += t2 - t1;
        worst = worst.max(t2 - t);
    }
    eprintln!("{w}x{hh}@{scale}: сцена {:?}, растр {:?}, худший {worst:?}, сдвигов {shifts} из {frames}", scene / frames, raster / frames);
}

#[test]
fn settings_scroll_frames_match_full_frames() {
    use masonry::core::{PointerScrollEvent, ScrollDelta};
    let channel = |p: u32, shift: u32| ((p >> shift) & 0xff) as i32;
    let mut h = Harness::new();
    let note = h.vault.path().join("заметка.md");
    std::fs::write(&note, (0..40).map(|i| format!("Строка {i} текста под окном настроек.\n\n")).collect::<String>()).unwrap();
    h.app.open(&mut h.root, note);
    h.app.open_settings(&mut h.root);
    let (w, hh) = (W as u16, H as u16);
    let mut renderer = crate::render::Renderer::new(2);
    for _ in 0..30 {
        h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_micros(8333)));
        let (layers, _) = h.root.redraw();
        renderer.draw_frame(&layers, w, hh, 1.0, theme::background(), false).unwrap();
    }
    let before = renderer.shifts;
    for step in 0..8 {
        let state = PointerState { position: PhysicalPosition::new(600.0, 300.0), count: 1, scale_factor: 1.0, ..PointerState::default() };
        let dy = if step < 5 { -23.0 } else { 17.0 };
        h.root.handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent { pointer: MOUSE, delta: ScrollDelta::PixelDelta(PhysicalPosition::new(0.0, dy)), state }));
        let (layers, _) = h.root.redraw();
        renderer.draw_frame(&layers, w, hh, 1.0, theme::background(), false).unwrap();
        let mut full = crate::render::Renderer::new(2);
        full.render(&layers, w, hh, 1.0, theme::background()).unwrap();
        let differ = renderer.frame_pixels().iter().zip(full.frame_pixels()).filter(|&(&x, &y)| [0, 8, 16].iter().any(|&s| (channel(x, s) - channel(y, s)).abs() > 2)).count();
        assert_eq!(differ, 0, "шаг {step}: прокрутка настроек расходится с полной отрисовкой в {differ} пикселях");
    }
    assert!(renderer.shifts > before, "прокрутка настроек не сдвигала пиксели");
}

#[test]
#[ignore]
fn settings_rebuild_cost() {
    let (w, hh) = bench_size().unwrap_or((W, H));
    let scale: f64 = std::env::var("AQUILUM_BENCH_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let vault = crate::test_dir::tempdir();
    let note = vault.path().join("длинная.md");
    let body: String = (0..400).map(|i| format!("Абзац {i}: плотный текст заметки с длинными предложениями, которые переносятся на несколько строк.\n\n")).collect();
    std::fs::write(&note, body).unwrap();
    let mut h = Harness::with(Some(vault.path()), scale);
    h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
    h.app.open(&mut h.root, note);
    h.app.open_settings(&mut h.root);
    let mut renderer = crate::render::Renderer::new(std::thread::available_parallelism().map_or(4, |n| n.get() as u16 - 1));
    for _ in 0..30 {
        h.root.handle_window_event(masonry::core::WindowEvent::AnimFrame(std::time::Duration::from_micros(8333)));
        let (layers, _) = h.root.redraw();
        renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), false).unwrap();
    }
    for size in [15, 16, 14] {
        let mut config = h.config();
        config.ui.font.font_size_base = size;
        h.app.workspace.core.settings.update_config(config).unwrap();
        let t = std::time::Instant::now();
        h.app.restyle(&mut h.root, true);
        let built = t.elapsed();
        let (layers, _) = h.root.redraw();
        let scene = t.elapsed() - built;
        let t2 = std::time::Instant::now();
        renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), false).unwrap();
        eprintln!("шрифт {size}: restyle {built:?}, раскладка и сцена {scene:?}, растр {:?}", t2.elapsed());
    }
    for color in ["#e0457b", "#2a9d8f", "#1471eb"] {
        let mut config = h.config();
        config.ui.primary_color = color.into();
        h.app.workspace.core.settings.update_config(config).unwrap();
        let t = std::time::Instant::now();
        h.app.restyle(&mut h.root, false);
        let built = t.elapsed();
        let (layers, _) = h.root.redraw();
        let scene = t.elapsed() - built;
        let t2 = std::time::Instant::now();
        renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), false).unwrap();
        eprintln!("цвет {color}: restyle {built:?}, сцена {scene:?}, растр {:?}", t2.elapsed());
    }
}
