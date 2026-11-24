use std::time::{Duration, Instant};

use super::damage::Damage;
use super::layer::area;
use super::renderer::Renderer;
use super::stamp::{Stamps, covering};
use crate::output::{Area, Canvas};

const FLASH: Duration = Duration::from_millis(350);
const EDGE: u32 = 2;

pub use super::mark::Mark;

fn color(mark: Mark) -> [f32; 3] {
    match mark {
        Mark::Raster => [1.0, 0.23, 0.19],
        Mark::Live => [0.69, 0.32, 0.87],
        Mark::Shift => [0.04, 0.52, 1.0],
        Mark::Region => [0.2, 0.78, 0.35],
        Mark::Layer => [1.0, 0.84, 0.04],
        Mark::Sprite => [0.39, 0.82, 1.0],
    }
}

fn blend(under: u32, rgb: [f32; 3], alpha: f32) -> u32 {
    let under = super::in_canvas_order(under);
    let channel = |shift: u32, value: f32| {
        let dest = ((under >> shift) & 0xff) as f32;
        ((dest * (1.0 - alpha) + value * alpha * 255.0).round().clamp(0.0, 255.0) as u32) << shift
    };
    super::in_canvas_order(0xff00_0000 | channel(0, rgb[0]) | channel(8, rgb[1]) | channel(16, rgb[2]))
}

#[derive(Default)]
pub struct Overlay {
    pub on: bool,
    pending: bool,
    flashes: Vec<(Area, Instant, Mark)>,
    shifted: Vec<Area>,
}

impl Overlay {
    pub fn toggle(&mut self) {
        self.on = !self.on;
        self.pending = true;
    }

    pub fn busy(&self) -> bool {
        self.on || self.pending
    }

    pub fn damage(&mut self, damage: &Damage) {
        self.shifted = damage.moves.iter().map(|m| area((m.x, m.y, m.w, m.h))).collect();
        for &rect in &damage.areas {
            self.flash(area(rect), Mark::Raster);
        }
    }

    pub fn live(&mut self, area: Area) {
        self.flash(area, Mark::Live);
    }

    pub fn flash(&mut self, area: Area, mark: Mark) {
        if self.on && area.w > 0 && area.h > 0 {
            self.flashes.push((area, Instant::now(), mark));
        }
    }

    pub fn show(&mut self, canvas: &mut dyn Canvas, stamps: &mut Stamps, outlines: &[(Area, Mark)]) -> Option<Area> {
        let now = Instant::now();
        self.flashes.retain(|(_, at, _)| now - *at < FLASH);
        let flashes = &self.flashes;
        let mut bounds: Option<Area> = None;
        canvas.with_target(&mut |mut target| {
            let mut paint = |area: Area, mark: Mark, alpha: f32| {
                if let Some(area) = stamps.apply(&mut target, area, |_, _, p| *p = blend(*p, color(mark), alpha)) {
                    bounds = Some(bounds.map_or(area, |b| b.union(area)));
                }
            };
            for (area, at, mark) in flashes {
                let fade = 1.0 - (now - *at).as_secs_f32() / FLASH.as_secs_f32();
                paint(*area, *mark, 0.45 * fade);
            }
            for (area, mark) in outlines {
                for edge in edges(*area) {
                    paint(edge, *mark, 1.0);
                }
            }
        });
        bounds
    }
}

impl Renderer {
    pub(super) fn decorate(&mut self, canvas: &mut dyn Canvas) -> Option<Area> {
        self.overlay.pending = false;
        if !self.overlay.on {
            self.overlay.flashes.clear();
            return None;
        }
        let scale = self.params.map_or(1.0, |p| p.2);
        let mut outlines: Vec<(Area, Mark)> = super::scroll::snapshot().into_iter().map(|r| (covering(r.viewport.scale_from_origin(scale)), Mark::Region)).collect();
        outlines.extend(self.overlay.shifted.iter().map(|a| (*a, Mark::Shift)));
        outlines.extend(self.live.iter().map(|l| (area(l.area), Mark::Layer)));
        outlines.extend(self.sprites.iter().filter(|s| !s.covered).map(|s| (covering(s.bounds()), Mark::Sprite)));
        self.overlay.show(canvas, &mut self.stamps, &outlines)
    }
}

fn edges(a: Area) -> [Area; 4] {
    let e = EDGE.min(a.w).min(a.h);
    [
        Area { x: a.x, y: a.y, w: a.w, h: e },
        Area { x: a.x, y: (a.y + a.h).saturating_sub(e), w: a.w, h: e },
        Area { x: a.x, y: a.y + e, w: e, h: a.h.saturating_sub(e * 2) },
        Area { x: (a.x + a.w).saturating_sub(e), y: a.y + e, w: e, h: a.h.saturating_sub(e * 2) },
    ]
}
