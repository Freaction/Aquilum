use masonry::kurbo::{Rect, Vec2};

use super::{Draw, Move, Tracker, overlap};

pub(super) struct Region {
    pub(super) id: u64,
    pub(super) bounds: Rect,
    pub(super) viewport: Rect,
    pub(super) offset: f64,
    pub(super) started: bool,
    pub(super) ended: bool,
    pub(super) blocked: bool,
    pub(super) band: (f64, f64),
    pub(super) above: Vec<Rect>,
}

impl Region {
    pub(super) fn occlude(&mut self, y0: f64, y1: f64) {
        self.band = (y0.max(self.viewport.y0), y1.min(self.viewport.y1));
        self.blocked = false;
    }

    pub(super) fn exclude(&mut self, y0: f64, y1: f64) {
        let (y0, y1) = (y0.max(self.viewport.y0), y1.min(self.viewport.y1));
        if y1 <= y0 {
            return;
        }
        let middle = (self.viewport.y0 + self.viewport.y1) / 2.0;
        if y1 <= middle {
            self.band.0 = self.band.0.max(y1);
        } else if y0 >= middle {
            self.band.1 = self.band.1.min(y0);
        } else {
            self.blocked = true;
        }
    }
}

pub(super) struct Past {
    pub(super) id: u64,
    pub(super) offset: f64,
    pub(super) above: Vec<Rect>,
}

impl Tracker {
    pub(super) fn shift(&self, region: &Region) -> Option<f64> {
        let past = self.past.iter().find(|p| p.id == region.id)?;
        let dy = past.offset - region.offset;
        (region.started && dy != 0.0).then_some(dy)
    }

    pub(super) fn moves(&self, shifts: &[Option<f64>], rects: &mut Vec<Rect>) -> Vec<Move> {
        let mut moves = Vec::new();
        for (region, shift) in self.regions.iter().zip(shifts) {
            let Some(dy) = *shift else { continue };
            let past = self.past.iter().find(|p| p.id == region.id);
            if region.blocked || dy.abs() >= region.viewport.height() {
                rects.push(region.viewport);
                continue;
            }
            let vp = region.viewport;
            let inner = Rect::new(vp.x0.ceil(), vp.y0.max(region.band.0).ceil(), vp.x1.floor(), vp.y1.min(region.band.1).floor()).intersect(self.frame);
            let touched = |area: Rect| {
                let hit = |d: &Draw| d.region == Some(region.id) && !d.steady && overlap(d.rect, area);
                self.draws.iter().any(hit) || self.previous.iter().any(hit)
            };
            let content = self.draws.iter().chain(&self.previous).filter(|d| d.region == Some(region.id) && !d.steady).map(|d| d.rect).reduce(|a, b| a.union(b));
            let Some(content) = content else { continue };
            let mut moved = Rect::new(vp.x0.floor().max(content.x0.floor()), inner.y0, vp.x1.ceil().min(content.x1.ceil()), inner.y1).intersect(self.frame);
            for above in &region.above {
                if above.y0 > inner.y0 + 1.0 || above.y1 < inner.y1 - 1.0 {
                    continue;
                }
                if above.x1 >= moved.x1 - 0.5 && above.x0 > moved.x0 {
                    moved.x1 = above.x0.floor();
                } else if above.x0 <= moved.x0 + 0.5 && above.x1 < moved.x1 {
                    moved.x0 = above.x1.ceil();
                }
            }
            if moved.width() < 1.0 || moved.height() <= dy.abs() {
                rects.push(vp);
                continue;
            }
            moves.push(Move { x: moved.x0 as u16, y: moved.y0 as u16, w: moved.width() as u16, h: moved.height() as u16, dy: dy as i32 });
            rects.push(if dy < 0.0 { Rect::new(moved.x0, moved.y1 + dy, moved.x1, vp.y1) } else { Rect::new(moved.x0, vp.y0, moved.x1, moved.y0 + dy) });
            rects.extend([Rect::new(vp.x0, vp.y0, vp.x1, inner.y0), Rect::new(vp.x0, inner.y1, vp.x1, vp.y1)]);
            for column in [Rect::new(vp.x0, inner.y0, moved.x0, inner.y1), Rect::new(moved.x1, inner.y0, vp.x1, inner.y1)] {
                if column.width() > 0.0 && touched(column) {
                    rects.push(column);
                }
            }
            rects.extend(region.above.iter().map(|r| r.intersect(moved)));
            if let Some(past) = past {
                rects.extend(past.above.iter().map(|r| (*r + Vec2::new(0.0, dy)).intersect(moved)));
            }
        }
        moves
    }
}
