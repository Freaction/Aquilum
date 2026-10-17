use super::*;

use std::time::Duration;

fn summary(b: &Bench) -> String {
    let (pages, bytes, glyphs) = b.renderer.glyph_atlas_summary().unwrap_or_default();
    format!("страниц {pages}, {:.1} МБ, записей глифов {glyphs}", crate::stats::mb(bytes))
}

#[test]
#[ignore]
fn editor_scroll_frames() {
    let mut b = Bench::start(None);
    let copy = b.h.copy_note();
    b.h.app.open_in(&mut b.h.root, copy, true);
    for _ in 0..40 {
        b.h.app.on_core_events(&mut b.h.root);
        let _ = b.h.root.redraw();
        std::thread::sleep(Duration::from_millis(15));
    }
    b.h.settle();
    let at = (f64::from(b.w) / 2.0 / b.scale, f64::from(b.hh) / 2.0 / b.scale);
    let _ = b.timed(true);
    let mut times: Vec<f64> = Vec::new();
    let flick = |b: &mut Bench, speed: f64, times: &mut Vec<f64>| {
        let mut v = speed;
        while v.abs() > 1.0 {
            b.h.scroll((at.0 * b.scale, at.1 * b.scale), b.scale, (0.0, -v));
            let (s, r) = b.timed(false);
            times.push((s + r).as_secs_f64() * 1000.0);
            v *= 0.955;
        }
    };
    for _ in 0..12 {
        flick(&mut b, 60.0, &mut times);
    }
    for _ in 0..6 {
        flick(&mut b, -45.0, &mut times);
    }
    let [p50, p95, p99, max] = percentiles(&times, [0.5, 0.95, 0.99, 1.0]);
    let over = |ms: f64| times.iter().filter(|&&t| t > ms).count();
    eprintln!("кадров {}, p50 {p50:.2} мс, p95 {p95:.2}, p99 {p99:.2}, максимум {max:.2}; дольше 4 мс: {}, дольше 8,3 мс: {}", times.len(), over(4.0), over(8.3));
    let worst: Vec<String> = times.iter().enumerate().filter(|&(_, &t)| t > 4.0).take(12).map(|(i, t)| format!("{i}:{t:.1}")).collect();
    eprintln!("медленные кадры (номер:мс): {}", worst.join(" "));
    eprintln!("{}", summary(&b));
}
