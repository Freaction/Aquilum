use super::*;
use crate::render::Renderer;

#[cfg(feature = "memory-probe")]
mod analysis;
mod atlas;
#[cfg(feature = "memory-probe")]
mod big;
mod cost;
mod frames;
#[cfg(feature = "memory-probe")]
mod open_memory;
mod sash;
#[cfg(feature = "memory-probe")]
mod search;
mod timing;
#[cfg(feature = "memory-probe")]
mod typing_memory;

use crate::stats::percentiles;

fn dense_body(paragraphs: usize) -> String {
    (0..paragraphs).map(|i| format!("Абзац {i}: плотный текст заметки с длинными предложениями, которые переносятся на несколько строк.\n\n")).collect()
}

struct Bench {
    h: Harness,
    renderer: Renderer,
    w: u32,
    hh: u32,
    scale: f64,
}

impl Bench {
    fn start(vault: Option<&std::path::Path>) -> Bench {
        let (w, hh) = bench_size().unwrap_or((W, H));
        let scale = bench_scale(1.0);
        let mut h = Harness::with(vault, scale);
        h.app.window_resized(&mut h.root, Size::new(f64::from(w) / scale, f64::from(hh) / scale));
        Bench { h, renderer: Renderer::new(raster_threads()), w, hh, scale }
    }

    fn timed(&mut self, full: bool) -> (std::time::Duration, std::time::Duration) {
        let t = std::time::Instant::now();
        let (layers, _) = self.h.root.redraw();
        let t1 = std::time::Instant::now();
        self.renderer.draw_frame(&layers, self.w as u16, self.hh as u16, self.scale, theme::background(), full).unwrap();
        (t1 - t, t1.elapsed())
    }

    fn animate(&mut self, frames: u32) {
        for _ in 0..frames {
            self.h.anim(std::time::Duration::from_micros(8333));
            self.timed(false);
        }
    }
}

#[cfg(feature = "memory-probe")]
struct Threads(std::collections::HashMap<u32, f64>);

#[cfg(feature = "memory-probe")]
impl Threads {
    fn now() -> Self {
        Self(crate::memory_probe::threads().into_iter().map(|(id, _, ms)| (id, ms)).collect())
    }

    fn busy(&self, least: f64) -> String {
        let mut busy: Vec<(String, f64)> = crate::memory_probe::threads()
            .into_iter()
            .map(|(id, name, ms)| (format!("{id} {name}"), ms - self.0.get(&id).copied().unwrap_or(0.0)))
            .filter(|(_, ms)| *ms > least)
            .collect();
        busy.sort_by(|a, b| b.1.total_cmp(&a.1));
        busy.iter().map(|(name, ms)| format!("{name}: {ms:.0} мс")).collect::<Vec<_>>().join("; ")
    }
}

impl Harness {
    fn copy_note(&self) -> std::path::PathBuf {
        let note = self.vault.path().join("заметка.md");
        std::fs::copy(std::env::var("AQ_NOTE").expect("AQ_NOTE"), &note).unwrap();
        note
    }
}

#[cfg(feature = "memory-probe")]
pub(super) fn wait_index(core: &aquilum_core::Core, workspace: &str) -> bool {
    wait_until(6000, 50, || {
        let status = core.search.status(Some(workspace));
        status.state == aquilum_core::search::models::SearchIndexState::Ready && !status.updating
    })
}

#[cfg(feature = "memory-probe")]
fn copy_notes(from: &std::path::Path, to: &std::path::Path) {
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let (path, name) = (entry.path(), entry.file_name());
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            std::fs::create_dir_all(to.join(&name)).unwrap();
            copy_notes(&path, &to.join(&name));
        } else if path.extension().is_some_and(|e| e == "md") {
            std::fs::copy(&path, to.join(&name)).unwrap();
        }
    }
}

#[cfg(feature = "memory-probe")]
fn clock<T>(work: impl FnOnce() -> T) -> (T, f64) {
    let t = std::time::Instant::now();
    let out = work();
    (out, t.elapsed().as_secs_f64() * 1000.0)
}

#[cfg(feature = "memory-probe")]
fn measured<T>(work: impl FnOnce() -> T) -> (T, f64, f64) {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let base = crate::memory_probe::heap();
    let (stop, peak) = (std::sync::Arc::new(AtomicBool::new(false)), std::sync::Arc::new(AtomicUsize::new(base)));
    let sampler = {
        let (stop, peak) = (std::sync::Arc::clone(&stop), std::sync::Arc::clone(&peak));
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                peak.fetch_max(crate::memory_probe::heap(), Ordering::Relaxed);
                std::thread::sleep(std::time::Duration::from_micros(200));
            }
        })
    };
    let (out, ms) = clock(work);
    stop.store(true, Ordering::Relaxed);
    sampler.join().unwrap();
    (out, ms, crate::stats::mb(peak.load(Ordering::Relaxed).saturating_sub(base)))
}
