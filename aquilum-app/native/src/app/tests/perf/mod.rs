use super::*;
use crate::render::Renderer;

mod cost;
mod frames;
mod timing;

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
