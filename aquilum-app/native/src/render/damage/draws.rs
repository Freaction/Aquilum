use std::hash::Hasher;

use masonry::imaging::{FillRef, GeometryRef};
use masonry::kurbo::{Affine, Rect};
use masonry::peniko::BrushRef;

use super::hash::H;
use super::{Draw, Solid, Tracker, overlap};

impl Tracker {
    pub(super) fn add(&mut self, mut h: H, bounds: Rect, steady: Option<(f64, f64)>, solid: Option<(H, f64)>) {
        let level = self.top();
        let rect = bounds.intersect(level.clip);
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        for sprite in &mut self.sprites {
            sprite.covered |= overlap(sprite.bounds(), rect);
        }
        let covers = |vp: Rect| steady.is_some_and(|(y0, y1)| y0 <= vp.y0 && y1 >= vp.y1);
        let steady_inside = match level.region {
            Some(region) => covers(self.regions[region].viewport.intersect(self.frame)),
            None => {
                self.outside(rect, steady);
                false
            }
        };
        h.0.write_u64(level.state);
        let region = level.region.map(|r| self.regions[r].id);
        let solid = solid.map(|(mut key, radius)| {
            key.0.write_u64(level.state);
            Solid { key: key.finish(), rect: bounds, radius, clip: level.clip }
        });
        self.draws.push(Draw { hash: h.finish(), rect, region, steady: steady_inside, solid });
    }

    pub(super) fn solid_key(draw: &FillRef<'_>) -> Option<(H, f64)> {
        let [scale_x, skew_y, skew_x, scale_y, _, _] = draw.transform.as_coeffs();
        if !matches!(draw.brush, BrushRef::Solid(_)) || skew_x != 0.0 || skew_y != 0.0 {
            return None;
        }
        let radii = match &draw.shape {
            GeometryRef::Rect(_) => [0.0; 4],
            GeometryRef::RoundedRect(rounded) => {
                let r = rounded.radii();
                [r.top_left, r.top_right, r.bottom_right, r.bottom_left]
            }
            _ => return None,
        };
        let mut key = H::new(5);
        key.f64(scale_x);
        key.f64(scale_y);
        for radius in radii {
            key.f64(radius);
        }
        key.brush(&draw.brush, draw.brush_transform);
        key.composite(draw.composite);
        key.fill_rule(draw.fill_rule);
        Some((key, radii.into_iter().fold(0.0, f64::max) * scale_x.abs().max(scale_y.abs())))
    }

    fn outside(&mut self, rect: Rect, steady: Option<(f64, f64)>) {
        let (top, bottom) = (rect.y0.floor(), rect.y1.ceil());
        let (y0, y1) = steady.map_or((bottom, bottom), |(y0, y1)| (y0.max(top), y1.min(bottom)));
        for region in &mut self.regions {
            let pixels = region.viewport.expand();
            if !overlap(pixels, rect) {
                continue;
            }
            if region.ended {
                region.above.push(rect.intersect(pixels));
            } else if !region.started {
                region.exclude(top, y0);
                region.exclude(y1, bottom);
            }
        }
    }

    pub(super) fn steady(&self, shape: &GeometryRef<'_>, transform: Affine, brush: &BrushRef<'_>, band: f64) -> Option<(f64, f64)> {
        let [_, skew_y, skew_x, scale_y, _, _] = transform.as_coeffs();
        if !matches!(brush, BrushRef::Solid(_)) || skew_x != 0.0 || skew_y != 0.0 || scale_y <= 0.0 {
            return None;
        }
        let (rect, radius) = match shape {
            GeometryRef::Rect(rect) => (*rect, 0.0),
            GeometryRef::RoundedRect(rounded) => {
                let r = rounded.radii();
                (rounded.rect(), r.top_left.max(r.top_right).max(r.bottom_left).max(r.bottom_right))
            }
            _ => return None,
        };
        let inset = radius + band;
        let steady = transform.transform_rect_bbox(Rect::new(rect.x0, rect.y0 + inset, rect.x1, rect.y1 - inset));
        let clip = self.top().clip;
        Some((steady.y0.max(clip.y0).ceil(), steady.y1.min(clip.y1).floor()))
    }
}
