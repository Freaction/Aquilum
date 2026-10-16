use std::hash::Hasher;

use masonry::imaging::{ClipRef, GeometryRef};
use masonry::kurbo::{Affine, Rect, Vec2};

use super::hash::H;
use super::{Level, OPEN, Tracker, close};
use crate::render::paint::convert;

impl Tracker {
    pub(super) fn top(&self) -> Level {
        *self.stack.last().expect("корень стека")
    }

    pub(super) fn lift(&self, region: Option<usize>) -> f64 {
        region.map_or(0.0, |r| self.regions[r].offset)
    }

    pub(super) fn push(&mut self, hash: u64, rect: Rect, faded: bool, square: bool) {
        let level = self.top();
        let found = level.region.is_none().then(|| self.regions.iter().position(|r| !r.started && close(r.bounds, rect))).flatten();
        if let Some(found) = found {
            self.regions[found].started = true;
        }
        let clip = level.clip.intersect(found.map_or(rect, |i| self.regions[i].viewport));
        let region = level.region.or(found);
        let mut cut = if square { level.cut.intersect(rect + Vec2::new(0.0, self.lift(level.region))) } else { level.cut };
        let mut h = H::new(11);
        h.0.write_u64(level.state);
        if !square {
            h.0.write_u64(hash);
        }
        if found.is_some() {
            h.f64(clip.y0);
            h.f64(clip.y1);
            cut = Rect::new(cut.x0, OPEN.y0, cut.x1, OPEN.y1);
        }
        let state = if square && found.is_none() { level.state } else { h.finish() };
        self.stack.push(Level { state, clip, cut, region, faded: level.faded || faded });
    }

    pub(super) fn pop(&mut self) {
        if self.stack.len() > 1 {
            let level = self.stack.pop().expect("уровень стека");
            if let Some(region) = level.region
                && self.top().region != Some(region)
            {
                self.regions[region].ended = true;
            }
        }
    }
}

pub(super) fn square(clip: &ClipRef<'_>) -> bool {
    matches!(clip, ClipRef::Fill { transform, shape: GeometryRef::Rect(_), .. } if transform.as_coeffs()[1] == 0.0 && transform.as_coeffs()[2] == 0.0)
}

pub(super) fn clip_bounds(clip: &ClipRef<'_>, shift: f64) -> (u64, Rect) {
    let mut h = H::new(10);
    let rect = match clip {
        ClipRef::Fill { transform, shape, fill_rule } => {
            h.affine(Affine::translate((0.0, shift)) * *transform);
            h.geometry(shape);
            h.fill_rule(*fill_rule);
            transform.transform_rect_bbox(convert::shape_bounds(shape))
        }
        ClipRef::Stroke { transform, shape, stroke } => {
            h.affine(Affine::translate((0.0, shift)) * *transform);
            h.geometry(shape);
            h.stroke(stroke);
            let w = stroke.width.max(1.0) * stroke.miter_limit.max(1.0);
            transform.transform_rect_bbox(convert::shape_bounds(shape).inflate(w, w))
        }
    };
    (h.finish(), rect)
}
