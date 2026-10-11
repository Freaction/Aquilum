mod hash;
mod merge;
mod region;
mod sink;

use masonry::kurbo::{Affine, Rect, RoundedRect, Vec2};

use super::scroll::{self, close};
use super::sprite::Sprite;
use region::{Past, Region};

#[derive(Clone, Copy)]
struct Draw {
    hash: u64,
    rect: Rect,
    region: Option<u64>,
    steady: bool,
    solid: Option<Solid>,
}

#[derive(Clone, Copy)]
struct Solid {
    key: u64,
    rect: Rect,
    radius: f64,
    clip: Rect,
}

impl Solid {
    fn changed(self, old: Solid) -> Vec<Rect> {
        let span = self.rect.union(old.rect);
        let r = self.radius;
        let band = |a: f64, b: f64| (a != b).then(|| (a.min(b) - r, a.max(b) + r));
        let columns = [band(old.rect.x0, self.rect.x0), band(old.rect.x1, self.rect.x1)].into_iter().flatten().map(|(x0, x1)| Rect::new(x0, span.y0, x1, span.y1));
        let rows = [band(old.rect.y0, self.rect.y0), band(old.rect.y1, self.rect.y1)].into_iter().flatten().map(|(y0, y1)| Rect::new(span.x0, y0, span.x1, y1));
        let edges: Vec<Rect> = columns.chain(rows).map(|rect| rect.intersect(self.clip)).collect();
        let whole = [self.rect.intersect(self.clip), old.rect.intersect(old.clip)];
        let size = |rects: &[Rect]| rects.iter().map(|r| r.width().max(0.0) * r.height().max(0.0)).sum::<f64>();
        if size(&edges) < size(&whole) { edges } else { whole.to_vec() }
    }
}

#[derive(Clone, Copy)]
struct Level {
    state: u64,
    clip: Rect,
    region: Option<usize>,
    faded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub dy: i32,
}

#[derive(Default)]
pub struct Damage {
    pub areas: Vec<(u16, u16, u16, u16)>,
    pub moves: Vec<Move>,
}

pub struct Tracker {
    frame: Rect,
    stack: Vec<Level>,
    draws: Vec<Draw>,
    previous: Vec<Draw>,
    forced: Option<Rect>,
    regions: Vec<Region>,
    past: Vec<Past>,
    pub live: Vec<(u64, Rect, Affine, RoundedRect)>,
    pub sprites: Vec<Sprite>,
    covered: Vec<Rect>,
}

impl Tracker {
    pub fn new() -> Self {
        Tracker { frame: Rect::ZERO, stack: Vec::new(), draws: Vec::new(), previous: Vec::new(), forced: None, regions: Vec::new(), past: Vec::new(), live: Vec::new(), sprites: Vec::new(), covered: Vec::new() }
    }

    pub fn forget(&mut self) {
        self.previous.clear();
        self.past.clear();
    }

    pub fn begin(&mut self, width: u16, height: u16, scale: f64) {
        self.frame = Rect::new(0.0, 0.0, f64::from(width), f64::from(height));
        self.stack.clear();
        self.stack.push(Level { state: 0, clip: self.frame, region: None, faded: false });
        self.draws.clear();
        self.live.clear();
        self.sprites.clear();
        self.forced = None;
        self.regions = scroll::snapshot()
            .into_iter()
            .map(|r| {
                let bounds = r.viewport.scale_from_origin(scale);
                let viewport = scroll::snap(bounds);
                Region { id: r.id, bounds, viewport, offset: (r.offset * scale).round(), started: false, ended: false, blocked: false, band: (viewport.y0, viewport.y1), above: Vec::new() }
            })
            .collect();
    }

    pub fn damage(&mut self) -> Damage {
        self.draws.sort_unstable_by_key(|d| d.hash);
        let shifts: Vec<Option<f64>> = self.regions.iter().map(|r| self.shift(r)).collect();
        let mut rects = Vec::new();
        rects.extend(self.forced);
        let covered: Vec<Rect> = self.sprites.iter().filter(|s| s.covered).map(Sprite::bounds).collect();
        rects.extend(covered.iter().copied().chain(std::mem::replace(&mut self.covered, covered.clone())));
        super::sprite::cover(&self.sprites);
        let (mut i, mut j) = (0, 0);
        let (mut added, mut gone) = (Vec::new(), Vec::new());
        while i < self.draws.len() || j < self.previous.len() {
            match (self.draws.get(i), self.previous.get(j)) {
                (Some(a), Some(b)) if a.hash == b.hash => {
                    i += 1;
                    j += 1;
                }
                (Some(a), Some(b)) if a.hash < b.hash => {
                    added.push(*a);
                    i += 1;
                }
                (Some(a), None) => {
                    added.push(*a);
                    i += 1;
                }
                (_, Some(b)) => {
                    gone.push(*b);
                    j += 1;
                }
                (None, None) => unreachable!(),
            }
        }
        let calm = shifts.iter().all(Option::is_none);
        let (mut solids, rest): (Vec<Draw>, Vec<Draw>) = gone.into_iter().partition(|d| calm && d.solid.is_some());
        for draw in added {
            let pair = draw.solid.filter(|_| calm).and_then(|solid| Some((solid, solids.iter().position(|old| old.solid.is_some_and(|o| o.key == solid.key))?)));
            match pair {
                Some((solid, at)) => rects.extend(solid.changed(solids.swap_remove(at).solid.expect("заливка с ключом"))),
                None => rects.push(draw.rect),
            }
        }
        for draw in solids.into_iter().chain(rest) {
            self.vanished(draw, &shifts, &mut rects);
        }
        let moves = self.moves(&shifts, &mut rects);
        std::mem::swap(&mut self.draws, &mut self.previous);
        self.past = self.regions.drain(..).filter(|r| r.started).map(|r| Past { id: r.id, offset: r.offset, above: r.above }).collect();
        Damage { areas: self.areas(rects), moves }
    }

    fn vanished(&self, draw: Draw, shifts: &[Option<f64>], rects: &mut Vec<Rect>) {
        let moved = draw.region.and_then(|id| self.regions.iter().zip(shifts).find(|(r, _)| r.id == id));
        if let Some((region, Some(dy))) = moved {
            rects.push((draw.rect + Vec2::new(0.0, *dy)).intersect(region.viewport));
            return;
        }
        rects.push(draw.rect);
        for (region, shift) in self.regions.iter().zip(shifts) {
            if let Some(dy) = shift
                && draw.region != Some(region.id)
            {
                rects.push((draw.rect.intersect(region.viewport) + Vec2::new(0.0, *dy)).intersect(region.viewport));
            }
        }
    }
}

fn overlap(a: Rect, b: Rect) -> bool {
    let i = a.intersect(b);
    i.width() > 0.0 && i.height() > 0.0
}
