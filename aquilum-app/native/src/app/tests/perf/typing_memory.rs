use super::*;

use std::time::{Duration, Instant};


use crate::memory_probe::{cpu_ms, heap, peak, report, reset_peak, trace};
use crate::stats::mb;

fn private() -> String {
    report().lines().next().unwrap_or_default().to_owned()
}

fn phase(b: &mut Bench, name: &str, base: usize, run: impl FnOnce(&mut Bench)) {
    reset_peak();
    let (t, cpu, threads) = (Instant::now(), cpu_ms(), Threads::now());
    run(b);
    eprintln!("{name}: {:?}, ЦП {:.0} мс, пик +{:.0} МБ, после +{:.0} МБ · {}", t.elapsed(), cpu_ms() - cpu, mb(peak().saturating_sub(base)), mb(heap().saturating_sub(base)), private());
    eprintln!("  потоки: {}", threads.busy(20.0));
}

fn sampler(stop: std::sync::Arc<std::sync::atomic::AtomicBool>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let (mut top, mut snapshot) = (heap(), String::new());
        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
            if heap() > top + 32 * 1024 * 1024 {
                top = heap();
                snapshot = format!("куча +{:.0} МБ\n{}", mb(top), report());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        snapshot
    })
}

fn background(b: &mut Bench, seconds: u64) -> String {
    let (mut top, mut snapshot) = (heap(), String::new());
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(seconds) {
        b.h.app.on_core_events(&mut b.h.root);
        b.h.app.tick(&mut b.h.root, Instant::now());
        b.timed(false);
        if heap() > top {
            (top, snapshot) = (heap(), report());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    snapshot
}

#[test]
#[ignore]
fn big_note_typing_memory() {
    aq_trace::start();
    let mut b = Bench::start(None);
    let note = b.h.copy_note();
    let (core, workspace) = (std::sync::Arc::clone(&b.h.app.workspace.core), b.h.vault.path().to_string_lossy().into_owned());
    core.search.prepare(&workspace).unwrap();
    assert!(wait_index(&core, &workspace), "индекс не готов");
    std::thread::sleep(Duration::from_secs(2));
    let base = heap();
    eprintln!("до открытия · {}", private());
    phase(&mut b, "открытие", base, |b| {
        b.h.app.open(&mut b.h.root, note.clone());
        b.timed(true);
    });
    phase(&mut b, "фон после открытия 10 с", base, |b| {
        background(b, 10);
    });
    let at = (f64::from(b.w) / 2.0, f64::from(b.hh) / 2.0);
    phase(&mut b, "Ctrl+End", base, |b| {
        b.h.focus_editor();
        b.h.click(at.0, at.1);
        b.h.key(Key::Named(NamedKey::End), Modifiers::CONTROL);
        b.timed(false);
    });
    let traced = std::env::var_os("AQ_PROBE_TRACE").is_some();
    trace(traced);
    for round in 1..=if traced { 1 } else { 3 } {
        phase(&mut b, &format!("круг {round}: 20 букв"), base, |b| {
            let mut slowest = (0.0f64, 0);
            for key in 0..20 {
                let t = Instant::now();
                b.h.key(Key::Character("ы".into()), Modifiers::empty());
                b.timed(false);
                let spent = t.elapsed().as_secs_f64() * 1000.0;
                if spent > slowest.0 {
                    slowest = (spent, key);
                }
            }
            eprintln!("самая долгая буква: №{} — {:.1} мс", slowest.1, slowest.0);
        });
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let watch = sampler(std::sync::Arc::clone(&stop));
        phase(&mut b, &format!("круг {round}: сохранение"), base, |b| {
            b.h.app.tick(&mut b.h.root, Instant::now() + Duration::from_secs(3600));
            crate::disk::wait();
        });
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if round == 1 {
            eprintln!("наибольший снимок сохранения:\n{}", watch.join().unwrap());
        }
        let mut snapshot = String::new();
        phase(&mut b, &format!("круг {round}: фон 15 с"), base, |b| {
            snapshot = background(b, 15);
        });
        if round == 1 {
            eprintln!("наибольший снимок фона:\n{snapshot}");
        }
    }
    if let Ok(out) = std::env::var("AQ_TRACE") {
        aq_trace::write(std::path::Path::new(&out)).unwrap();
    }
}
