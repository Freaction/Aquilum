use super::*;

use std::time::{Duration, Instant};

use crate::stats::mb;

fn frames(b: &mut Bench, at: (f64, f64)) -> (f64, f64, f64, f64) {
    let mut times = Vec::new();
    for speed in [60.0, -45.0, 60.0] {
        let mut v: f64 = speed;
        while v.abs() > 1.0 {
            let t = Instant::now();
            b.h.scroll(at, b.scale, (0.0, -v));
            b.h.deliver();
            b.timed(false);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
            v *= 0.955;
        }
    }
    let [p50, p95, max] = percentiles(&times, [0.5, 0.95, 1.0]);
    (p50, p95, max, times.len() as f64)
}

fn speeds(b: &mut Bench, at: (f64, f64)) {
    for px in [100.0, 300.0, 1000.0, 3000.0] {
        let mut times = Vec::new();
        for _ in 0..120 {
            let t = Instant::now();
            b.h.scroll(at, b.scale, (0.0, -px));
            b.h.deliver();
            b.timed(false);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        let [p50, p95, max] = percentiles(&times, [0.5, 0.95, 1.0]);
        eprintln!("быстрая прокрутка {px} px за кадр: p50 {p50:.1} мс, p95 {p95:.1}, максимум {max:.1}");
    }
}

fn typing(b: &mut Bench, at: (f64, f64)) {
    let scroll = b.h.app.note_ids.as_ref().unwrap().scroll.clone();
    scroll.edit(&mut b.h.root, |mut area| crate::ui::scroll::ScrollArea::set_fraction(&mut area, 0.5));
    b.h.deliver();
    b.h.focus_editor();
    b.h.click(at.0, at.1);
    b.timed(false);
    let keys = std::iter::repeat_n(Key::Character("ы".into()), 5).chain(std::iter::repeat_n(Key::Named(NamedKey::Enter), 3)).chain(std::iter::repeat_n(Key::Named(NamedKey::Backspace), 3));
    let mut times = Vec::new();
    for key in keys {
        let t = Instant::now();
        b.h.key(key, Modifiers::empty());
        b.timed(false);
        times.push(format!("{:.1}", t.elapsed().as_secs_f64() * 1000.0));
    }
    eprintln!("набор в середине (5 букв, 3 Enter, 3 Backspace), мс: {}", times.join(" "));
    let t = Instant::now();
    b.h.app.tick(&mut b.h.root, Instant::now() + Duration::from_secs(3600));
    let ui = t.elapsed();
    crate::disk::wait();
    eprintln!("сохранение: поток интерфейса {ui:?}, запись в фоне {:?}", t.elapsed());
}

#[test]
#[ignore]
fn big_note_audit() {
    aq_trace::start();
    let mut b = Bench::start(None);
    let note = b.h.copy_note();
    let heap = crate::memory_probe::heap();
    let t = Instant::now();
    b.h.open(note);
    let opened = t.elapsed();
    let (layers, _) = b.h.root.redraw();
    let scene = t.elapsed();
    b.renderer.draw_frame(&layers, b.w as u16, b.hh as u16, b.scale, theme::background(), true).unwrap();
    let shown = t.elapsed();
    let busy = |b: &Bench| b.h.app.embeds.as_ref().is_some_and(crate::app::embeds::Embeds::busy);
    while busy(&b) && t.elapsed() < Duration::from_secs(60) {
        b.h.app.on_core_events(&mut b.h.root);
        b.timed(false);
    }
    let settled = t.elapsed();
    let doc = b.h.app.text.byte_len();
    eprintln!("заметка {:.0} МБ: открытие {opened:?}, первая сцена {scene:?}, первый кадр {shown:?}, вставки готовы {settled:?}", mb(doc));
    eprintln!("куча после открытия +{:.0} МБ", mb(crate::memory_probe::heap().saturating_sub(heap)));
    let before = Threads::now();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get) as f64;
    let mut windows = Vec::new();
    for _ in 0..12 {
        let (cpu, idle) = (crate::memory_probe::cpu_ms(), Instant::now());
        while idle.elapsed() < Duration::from_secs(5) {
            b.h.app.on_core_events(&mut b.h.root);
            std::thread::sleep(Duration::from_millis(16));
        }
        windows.push(format!("{:.1}", (crate::memory_probe::cpu_ms() - cpu) / idle.elapsed().as_secs_f64() / 10.0 / threads));
    }
    eprintln!("простой по 5 с, % от всех {threads} потоков: {}", windows.join(" "));
    eprintln!("куча после простоя +{:.0} МБ", mb(crate::memory_probe::heap().saturating_sub(heap)));
    eprintln!("потоки в простое: {}", before.busy(50.0));
    let at = (f64::from(b.w) / 2.0, f64::from(b.hh) / 2.0);
    let scroll = b.h.app.note_ids.as_ref().unwrap().scroll.clone();
    for fraction in [0.0, 0.5, 0.95] {
        let t = Instant::now();
        scroll.edit(&mut b.h.root, |mut area| crate::ui::scroll::ScrollArea::set_fraction(&mut area, fraction));
        b.h.deliver();
        b.timed(false);
        let jump = t.elapsed();
        let (p50, p95, max, n) = frames(&mut b, at);
        let t = Instant::now();
        b.h.app.flush_view();
        let flush = t.elapsed();
        eprintln!("место {:>3.0}%: прыжок {jump:?}; кадров {n}, p50 {p50:.2} мс, p95 {p95:.2}, максимум {max:.2}; запись места {flush:?}", fraction * 100.0);
    }
    let mut drag = Vec::new();
    for step in 0..10 {
        let _zone = aq_trace::zone("стенд: шаг ползунка");
        let t = Instant::now();
        scroll.edit(&mut b.h.root, |mut area| crate::ui::scroll::ScrollArea::set_fraction(&mut area, 0.10 + f64::from(step) * 0.01));
        b.h.deliver();
        b.timed(false);
        drag.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    eprintln!("ползунок, шаги по 1 %: {}", drag.iter().map(|t| format!("{t:.0}")).collect::<Vec<_>>().join(" "));
    let (mut stuck, mut worst) = (0, 0.0f64);
    for _ in 0..200 {
        let before = super::super::support::note_offset(&b.h);
        let t = Instant::now();
        b.h.scroll(at, b.scale, (0.0, -3.0));
        b.h.deliver();
        b.timed(false);
        worst = worst.max(t.elapsed().as_secs_f64() * 1000.0);
        stuck += usize::from(super::super::support::note_offset(&b.h) == before);
    }
    eprintln!("тачпад по 3 px: событий 200, без сдвига {stuck}, худший кадр {worst:.1} мс");
    speeds(&mut b, at);
    typing(&mut b, at);
    let toggle = b.h.app.chrome.as_ref().unwrap().toggle_sidebar;
    b.h.act(toggle, crate::ui::widgets::Pressed);
    let mut panel = Vec::new();
    for _ in 0..30 {
        let t = Instant::now();
        b.h.anim(Duration::from_micros(8333));
        b.timed(false);
        panel.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let [p50, max] = percentiles(&panel, [0.5, 1.0]);
    eprintln!("панель: кадров 30, p50 {p50:.1} мс, максимум {max:.1} мс");
    if let Ok(out) = std::env::var("AQ_TRACE") {
        aq_trace::write(std::path::Path::new(&out)).unwrap();
    }
}
