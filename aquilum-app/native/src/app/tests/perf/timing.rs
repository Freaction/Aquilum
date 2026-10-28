use super::*;

use std::time::{Duration, Instant};

#[test]
#[ignore]
fn frame_parts() {
    let files: usize = std::env::var("AQUILUM_BENCH_FILES").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let vault = crate::test_dir::tempdir();
    for i in 0..files.max(1) {
        std::fs::write(vault.path().join(format!("Заметка {i:04}.md")), "текст").unwrap();
    }
    let mut b = Bench::start(Some(vault.path()));
    let note = vault.path().join("длинная.md");
    let body: String = (0..400).map(|i| format!("Абзац {i}: плотный текст заметки, как в настоящей базе знаний, с длинными предложениями, которые переносятся на несколько строк и заполняют всю ширину колонки редактора от края до края. ")).collect();
    let head = if std::env::var_os("AQUILUM_BENCH_HERO").is_some() { "---\ncover: true\ntype: book\nPage_cover: pattern:ruby\n---\n" } else { "" };
    std::fs::write(&note, format!("{head}{body}")).unwrap();
    b.h.open(note);
    b.h.wait_cover(!head.is_empty());
    let (mut scene, mut raster) = (Duration::ZERO, Duration::ZERO);
    let frames = 60u32;
    for i in 0..frames {
        let dy = if i < frames / 2 { -7.0 } else { 7.0 };
        b.h.scroll((f64::from(b.w) / 2.0, f64::from(b.hh) / 2.0), b.scale, (0.0, dy));
        let (s, r) = b.timed(i == 0);
        scene += s;
        raster += r;
    }
    eprintln!("{}x{}: сцена {:?}, растр {:?}, всего {:?}", b.w, b.hh, scene / frames, raster / frames, (scene + raster) / frames);
}

#[test]
#[ignore]
fn panel_animation_frames() {
    let vault = crate::test_dir::tempdir();
    let note = vault.path().join("длинная.md");
    std::fs::write(&note, dense_body(400)).unwrap();
    let mut b = Bench::start(Some(vault.path()));
    b.h.open(note);
    b.timed(true);
    for name in ["появление", "фокус", "выход", "панель"] {
        match name {
            "появление" => {}
            "панель" => {
                let toggle = b.h.app.chrome.as_ref().unwrap().toggle_sidebar;
                b.h.act(toggle, crate::ui::widgets::Pressed);
            }
            _ => b.h.app.toggle_focus_mode(&mut b.h.root),
        }
        let (mut worst, mut total, mut frames) = (Duration::ZERO, Duration::ZERO, 0u32);
        for _ in 0..40 {
            let t = Instant::now();
            b.h.anim(Duration::from_micros(8333));
            b.timed(false);
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
    let vault = crate::test_dir::tempdir();
    let note = vault.path().join("заметка.md");
    std::fs::write(&note, dense_body(60)).unwrap();
    let mut b = Bench::start(Some(vault.path()));
    b.h.open(note);
    b.timed(true);
    let t = Instant::now();
    b.h.app.open_settings(&mut b.h.root);
    let built = t.elapsed();
    let mut frames = Vec::new();
    for i in 0..30 {
        let t = Instant::now();
        if i > 0 {
            b.h.anim(Duration::from_micros(8333));
        }
        let (layers, _) = b.h.root.redraw();
        let t1 = Instant::now();
        b.renderer.draw_frame(&layers, b.w as u16, b.hh as u16, b.scale, theme::background(), false).unwrap();
        frames.push((t1 - t, t1.elapsed()));
    }
    eprintln!("{}x{}@{}: сборка окна {built:?}", b.w, b.hh, b.scale);
    for (i, (scene, raster)) in frames.iter().enumerate() {
        eprintln!("кадр {i}: сцена {scene:?}, растр {raster:?}");
    }
}

#[test]
#[ignore]
fn settings_scroll_frames() {
    let mut b = Bench::start(None);
    b.h.app.open_settings(&mut b.h.root);
    b.animate(40);
    let (mut scene, mut raster, mut worst, mut shifts) = (Duration::ZERO, Duration::ZERO, Duration::ZERO, 0usize);
    let frames = 60u32;
    for i in 0..frames {
        let dy = if i < frames / 2 { -9.0 } else { 9.0 };
        b.h.scroll((f64::from(b.w) * 0.6, f64::from(b.hh) / 2.0), b.scale, (0.0, dy));
        let before = b.renderer.shifts;
        let (s, r) = b.timed(false);
        shifts += b.renderer.shifts - before;
        scene += s;
        raster += r;
        worst = worst.max(s + r);
    }
    eprintln!("{}x{}@{}: сцена {:?}, растр {:?}, худший {worst:?}, сдвигов {shifts} из {frames}", b.w, b.hh, b.scale, scene / frames, raster / frames);
}
