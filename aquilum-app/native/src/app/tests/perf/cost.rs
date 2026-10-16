use super::*;

use std::time::{Duration, Instant};

fn open_env_note(h: &mut Harness) {
    let source = std::path::PathBuf::from(std::env::var("AQ_NOTE").unwrap());
    let note = h.vault.path().join("заметка.md");
    std::fs::copy(&source, &note).unwrap();
    h.app.open(&mut h.root, note);
    let _ = h.root.redraw();
}

fn restyle_cost(b: &mut Bench, fonts: bool) -> (Duration, Duration, Duration) {
    let t = Instant::now();
    b.h.app.restyle(&mut b.h.root, fonts);
    let built = t.elapsed();
    let (layers, _) = b.h.root.redraw();
    let scene = t.elapsed() - built;
    let t2 = Instant::now();
    b.renderer.draw_frame(&layers, b.w as u16, b.hh as u16, b.scale, theme::background(), false).unwrap();
    (built, scene, t2.elapsed())
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
    let started = Instant::now();
    let mut worst = (Duration::ZERO, 0.0);
    let mut total = Duration::ZERO;
    while started.elapsed() < Duration::from_secs(8) {
        let t = Instant::now();
        h.step();
        let spent = t.elapsed();
        total += spent;
        if spent > worst.0 {
            worst = (spent, started.elapsed().as_secs_f64());
        }
        if spent > Duration::from_millis(30) {
            eprintln!("{:.2} с: обработка событий ядра {:?}", started.elapsed().as_secs_f64(), spent);
        }
        std::thread::sleep(Duration::from_millis(8));
    }
    eprintln!("всего на главном потоке {:?}, худшее {:?} на {:.2} с", total, worst.0, worst.1);
}

#[test]
#[ignore]
fn typing_cost() {
    aq_trace::start();
    let mut h = Harness::with(None, 1.75);
    open_env_note(&mut h);
    h.root.focus_on(h.app.editor_id());
    let _ = h.root.redraw();
    h.key(Key::Named(NamedKey::End), crate::test_dir::command());
    let before = h.app.text.byte_len();
    let mut renderer = Renderer::new(4);
    let (w, hh) = ((f64::from(W) * 1.75) as u16, (f64::from(H) * 1.75) as u16);
    let (mut event, mut deliver, mut scene, mut raster) = (Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let n = 30u32;
    for i in 0..n {
        let t0 = Instant::now();
        let ev = KeyboardEvent { state: KeyState::Down, key: Key::Named(NamedKey::Backspace), code: Code::Backspace, location: Location::Standard, modifiers: Modifiers::empty(), repeat: i > 0, is_composing: false };
        h.root.handle_text_event(TextEvent::Keyboard(ev));
        let t1 = Instant::now();
        h.deliver();
        let t2 = Instant::now();
        let (layers, _) = h.root.redraw();
        let t3 = Instant::now();
        renderer.draw_frame(&layers, w, hh, 1.75, theme::background(), i == 0).unwrap();
        let t4 = Instant::now();
        event += t1 - t0;
        deliver += t2 - t1;
        scene += t3 - t2;
        raster += t4 - t3;
    }
    if let Ok(out) = std::env::var("AQ_TRACE") {
        aq_trace::write(std::path::Path::new(&out)).unwrap();
    }
    eprintln!("удалено байт {}; нажатие: событие {:?}, действия {:?}, сцена {:?}, растр {:?}", before - h.app.text.byte_len(), event / n, deliver / n, scene / n, raster / n);
}

#[test]
#[ignore]
fn resize_cost() {
    let scale = 1.75;
    let mut h = Harness::with(None, scale);
    open_env_note(&mut h);
    let mut renderer = Renderer::new(4);
    let (mut scene, mut raster) = (Duration::ZERO, Duration::ZERO);
    let n = 30u32;
    for i in 0..n {
        let (w, hh) = (2400 - i * 8, 1500 - i * 4);
        let t0 = Instant::now();
        h.root.handle_window_event(masonry::core::WindowEvent::Resize(PhysicalSize::new(w, hh)));
        h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
        h.deliver();
        let (layers, _) = h.root.redraw();
        let t1 = Instant::now();
        renderer.draw_frame(&layers, w as u16, hh as u16, scale, theme::background(), true).unwrap();
        scene += t1 - t0;
        raster += t1.elapsed();
    }
    eprintln!("шаг изменения размера: сцена {:?}, растр {:?}", scene / n, raster / n);
}

#[test]
#[ignore]
fn settings_rebuild_cost() {
    let vault = crate::test_dir::tempdir();
    let note = vault.path().join("длинная.md");
    std::fs::write(&note, dense_body(400)).unwrap();
    let mut b = Bench::start(Some(vault.path()));
    b.h.app.open(&mut b.h.root, note);
    b.h.app.open_settings(&mut b.h.root);
    b.animate(30);
    for size in [15, 16, 14] {
        let mut config = b.h.config();
        config.ui.font.font_size_base = size;
        b.h.app.workspace.core.settings.update_config(config).unwrap();
        let (built, scene, raster) = restyle_cost(&mut b, true);
        eprintln!("шрифт {size}: restyle {built:?}, раскладка и сцена {scene:?}, растр {raster:?}");
    }
    for color in ["#e0457b", "#2a9d8f", "#1471eb"] {
        let mut config = b.h.config();
        config.ui.primary_color = color.into();
        b.h.app.workspace.core.settings.update_config(config).unwrap();
        let (built, scene, raster) = restyle_cost(&mut b, false);
        eprintln!("цвет {color}: restyle {built:?}, сцена {scene:?}, растр {raster:?}");
    }
}
