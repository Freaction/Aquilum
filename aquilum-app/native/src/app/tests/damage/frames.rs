use super::*;
use crate::output::Area;

pub(super) struct Frames {
    pub(super) scale: f64,
    pub(super) width: u16,
    pub(super) height: u16,
    partial: Renderer,
    check: bool,
}

pub(super) struct Report {
    pub(super) raster: u64,
    pub(super) rects: Vec<(u16, u16, u16, u16)>,
    pub(super) moved: Vec<(u16, u16, u16, u16, i32)>,
    pub(super) shown: Vec<Area>,
    pub(super) time: std::time::Duration,
    pub(super) differ: usize,
    pub(super) causes: Vec<Rect>,
}

impl Report {
    pub(super) fn print(&self, label: &str) {
        let moved: u64 = self.moved.iter().map(|m| u64::from(m.2) * u64::from(m.3)).sum();
        eprintln!(
            "{label}: растр {} px {:?}; сдвиг {} px {:?}; вывод {} px {:?}; {:.2} мс",
            self.raster,
            self.rects,
            moved,
            self.moved,
            self.shown_area(),
            self.shown.iter().map(|a| (a.x, a.y, a.w, a.h)).collect::<Vec<_>>(),
            self.time.as_secs_f64() * 1000.0
        );
        let mut causes = self.causes.clone();
        causes.sort_by(|a, b| b.area().total_cmp(&a.area()));
        eprintln!("  крупнейшие изменения до слияния: {:?}", causes.iter().take(8).map(|r| (r.x0.round(), r.y0.round(), r.x1.round(), r.y1.round())).collect::<Vec<_>>());
    }

    pub(super) fn above(&self, y: f64) -> bool {
        self.rects.iter().any(|r| f64::from(r.1) + 4.0 < y.floor()) || self.moved.iter().any(|m| f64::from(m.1) < y.floor())
    }

    pub(super) fn shown_area(&self) -> u64 {
        self.shown.iter().map(|a| u64::from(a.w) * u64::from(a.h)).sum()
    }
}

impl Frames {
    pub(super) fn new(h: &mut Harness, spec: Spec, check: bool) -> Self {
        let (width, height) = (spec.width as u16, spec.height as u16);
        let mut frames = Frames { scale: spec.scale, width, height, partial: Renderer::new(raster_threads()), check };
        let (layers, _) = h.root.redraw();
        frames.partial.draw_frame(&layers, width, height, spec.scale, theme::background(), true).unwrap();
        frames
    }

    pub(super) fn draw(&mut self, h: &mut Harness) -> Report {
        let (layers, _) = h.root.redraw();
        let t = std::time::Instant::now();
        let mut shown = self.partial.draw_frame(&layers, self.width, self.height, self.scale, theme::background(), false).unwrap();
        #[cfg(not(target_os = "android"))]
        crate::output::merge(&mut shown);
        let time = t.elapsed();
        let differ = if self.check {
            differing_pixels(self.partial.frame_pixels(), render_full(&layers, self.width, self.height, self.scale).frame_pixels())
        } else {
            0
        };
        let rects = self.partial.rastered.clone();
        let raster = rects.iter().map(|r| u64::from(r.2) * u64::from(r.3)).sum();
        Report { raster, rects, moved: self.partial.moved.clone(), shown, time, differ, causes: std::mem::take(&mut self.partial.causes) }
    }
}

pub(super) fn summary(label: &str, reports: &[Report]) {
    let n = reports.len().max(1) as f64;
    let raster: f64 = reports.iter().map(|r| r.raster as f64).sum::<f64>() / n;
    let moved: f64 = reports.iter().map(|r| r.moved.iter().map(|m| f64::from(m.2) * f64::from(m.3)).sum::<f64>()).sum::<f64>() / n;
    let shown: f64 = reports.iter().flat_map(|r| r.shown.iter()).map(|a| f64::from(a.w) * f64::from(a.h)).sum::<f64>() / n;
    let time: f64 = reports.iter().map(|r| r.time.as_secs_f64() * 1000.0).sum::<f64>() / n;
    eprintln!("{label}: в среднем растр {raster:.0} px, сдвиг {moved:.0} px, вывод {shown:.0} px, {time:.2} мс на кадр");
}
