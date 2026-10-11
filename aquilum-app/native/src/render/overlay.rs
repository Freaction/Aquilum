use std::time::{Duration, Instant};

use super::stamp::{Stamps, blend};
use crate::output::{Area, Canvas};

const FLASH: Duration = Duration::from_millis(350);
const EDGE: u32 = 2;

pub use super::mark::Mark;

fn color(mark: Mark) -> [f32; 4] {
        let rgb = match mark {
            Mark::Raster => [1.0, 0.23, 0.19],
            Mark::Live => [0.69, 0.32, 0.87],
            Mark::Shift => [0.04, 0.52, 1.0],
            Mark::Region => [0.2, 0.78, 0.35],
            Mark::Layer => [1.0, 0.84, 0.04],
            Mark::Sprite => [0.39, 0.82, 1.0],
        };
        [rgb[0], rgb[1], rgb[2], 1.0]
}

#[derive(Default)]
pub struct Overlay {
    pub on: bool,
    pub pending: bool,
    flashes: Vec<(Area, Instant, Mark)>,
}

impl Overlay {
    pub fn flash(&mut self, area: Area, mark: Mark) {
        if self.on && area.w > 0 && area.h > 0 {
            self.flashes.push((area, Instant::now(), mark));
        }
    }

    pub fn show(&mut self, canvas: &mut dyn Canvas, stamps: &mut Stamps, outlines: &[(Area, Mark)]) -> Option<Area> {
        let now = Instant::now();
        self.flashes.retain(|(_, at, _)| now - *at < FLASH);
        if !self.on {
            self.flashes.clear();
            return None;
        }
        let flashes = &self.flashes;
        let mut bounds: Option<Area> = None;
        canvas.with_target(&mut |mut target| {
            let mut paint = |area: Area, color: [f32; 4], alpha: f32| {
                let tint = color.map(|c| c * alpha);
                if let Some(area) = stamps.apply(&mut target, area, |_, _, p| *p = blend(*p, tint, 1.0)) {
                    bounds = Some(bounds.map_or(area, |b| b.union(area)));
                }
            };
            for (area, at, mark) in flashes {
                let fade = 1.0 - (now - *at).as_secs_f32() / FLASH.as_secs_f32();
                paint(*area, color(*mark), 0.45 * fade);
            }
            for (area, mark) in outlines {
                for edge in edges(*area) {
                    paint(edge, color(*mark), 1.0);
                }
            }
        });
        bounds
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
