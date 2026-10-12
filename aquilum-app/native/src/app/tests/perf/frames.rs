use super::*;

use std::time::Duration;

#[test]
fn partial_frames_match_full_frames() {
    partial_frames(1.0, W, H, 40);
}

#[test]
fn partial_frames_match_full_frames_at_fractional_size() {
    partial_frames(1.75, 1750, 1051, 24);
}

fn partial_frames(scale: f64, width: u32, height: u32, steps: u32) {
    let mut h = Harness::with(None, scale);
    h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(width, height)));
    h.app.window_resized(&mut h.root, Size::new(f64::from(width) / scale, f64::from(height) / scale));
    let body: String = (0..120).map(|i| format!("Абзац {i}: текст заметки, который переносится на несколько строк в колонке редактора.\n\n")).collect();
    let note = h.write_note("Длинная.md", &format!("---\ncover: true\ntype: book\nPage_cover: pattern:polka\n---\n{body}"));
    h.app.open(&mut h.root, note);
    h.wait_cover(true);
    let (w, hh) = (width as u16, height as u16);
    let mut partial = Renderer::new(2);
    for i in 0..steps {
        if i % 4 == 3 {
            h.pointer_move(f64::from(40 + i * 7), f64::from(80 + i * 11));
        } else {
            let dy = if i < steps * 3 / 5 { -37.0 } else { 23.0 };
            h.scroll((f64::from(width) * 0.6, f64::from(height) * 0.5), scale, (0.0, dy));
        }
        h.anim(Duration::from_millis(16));
        let (layers, _) = h.root.redraw();
        partial.draw_frame(&layers, w, hh, scale, theme::background(), i == 0).unwrap();
        let full = render_full(&layers, w, hh, scale);
        let differ = differing_pixels(partial.frame_pixels(), full.frame_pixels());
        assert_eq!(differ, 0, "шаг {i}: частичная отрисовка расходится с полной в {differ} пикселях");
    }
    assert!(partial.shifts > steps as usize / 2, "прокрутка сдвигает пиксели, а не перерисовывает колонку: сдвигов {}", partial.shifts);
}

#[test]
fn live_cover_frames_match_full_frames() {
    let mut h = Harness::new();
    let note = h.write_note("Туннель.md", "---\ncover: true\nPage_cover: tunnel\n---\nТекст под обложкой.");
    h.app.open(&mut h.root, note);
    h.wait_cover(true);
    let (w, hh) = (W as u16, H as u16);
    h.anim(Duration::from_secs(1));
    let (layers, _) = h.root.redraw();
    let mut live = Renderer::new(2);
    crate::render::live::freeze(Some(0.0));
    live.draw_frame(&layers, w, hh, 1.0, theme::background(), true).unwrap();
    assert!(live.has_live(), "обложка не стала живым слоем");
    for step in 1..6 {
        let t = f64::from(step) * 0.137;
        crate::render::live::freeze(Some(t));
        assert!(!live.live_frame(&layers).unwrap().is_empty(), "шаг {step}: живой слой не перерисован");
        let full = render_full(&layers, w, hh, 1.0);
        dump(&format!("live-{step}.png"), &live);
        dump(&format!("full-{step}.png"), &full);
        let differ = differing_pixels(live.frame_pixels(), full.frame_pixels());
        assert_eq!(differ, 0, "шаг {step}: живой слой расходится с полной отрисовкой в {differ} пикселях");
    }
    crate::render::live::freeze(None);
}

#[test]
fn modal_intro_leaves_frames_like_full_redraw() {
    for close_early in [false, true] {
        let mut h = Harness::new();
        let note = h.write_note("заметка.md", "Текст заметки под окном настроек.");
        h.app.open(&mut h.root, note);
        let (w, hh) = (W as u16, H as u16);
        let mut renderer = Renderer::new(2);
        let (layers, _) = h.root.redraw();
        renderer.draw_frame(&layers, w, hh, 1.0, theme::background(), true).unwrap();
        let corner = renderer.frame_pixels()[usize::from(w) * 4 + 4];
        h.app.open_settings(&mut h.root);
        for step in 0..40 {
            h.anim(Duration::from_micros(8333));
            if close_early && step == 6 {
                h.key(Key::Named(NamedKey::Escape), Modifiers::empty());
            }
            let (layers, _) = h.root.redraw();
            renderer.draw_frame(&layers, w, hh, 1.0, theme::background(), false).unwrap();
            if !close_early {
                dump(&format!("modal-{step:02}.png"), &renderer);
            }
            if step == 12 && !close_early {
                let dimmed = renderer.frame_pixels()[usize::from(w) * 4 + 4];
                assert!(channel(dimmed, 8) < channel(corner, 8), "затемнение не наложено на середине появления");
            }
        }
        let (layers, _) = h.root.redraw();
        let full = render_full(&layers, w, hh, 1.0);
        let wrong = differing_pixels(renderer.frame_pixels(), full.frame_pixels());
        assert_eq!(wrong, 0, "после появления (закрыто рано: {close_early}) кадр расходится с полной отрисовкой в {wrong} пикселях");
    }
}

#[test]
fn settings_scroll_frames_match_full_frames() {
    let mut h = Harness::new();
    let text: String = (0..40).map(|i| format!("Строка {i} текста под окном настроек.\n\n")).collect();
    let note = h.write_note("заметка.md", &text);
    h.app.open(&mut h.root, note);
    h.app.open_settings(&mut h.root);
    let (w, hh) = (W as u16, H as u16);
    let mut renderer = Renderer::new(2);
    for _ in 0..30 {
        h.anim(Duration::from_micros(8333));
        let (layers, _) = h.root.redraw();
        renderer.draw_frame(&layers, w, hh, 1.0, theme::background(), false).unwrap();
    }
    let before = renderer.shifts;
    for step in 0..8 {
        let dy = if step < 5 { -23.0 } else { 17.0 };
        h.scroll((600.0, 300.0), 1.0, (0.0, dy));
        let (layers, _) = h.root.redraw();
        renderer.draw_frame(&layers, w, hh, 1.0, theme::background(), false).unwrap();
        let full = render_full(&layers, w, hh, 1.0);
        let differ = differing_pixels(renderer.frame_pixels(), full.frame_pixels());
        assert_eq!(differ, 0, "шаг {step}: прокрутка настроек расходится с полной отрисовкой в {differ} пикселях");
    }
    assert!(renderer.shifts > before, "прокрутка настроек не сдвигала пиксели");
}
