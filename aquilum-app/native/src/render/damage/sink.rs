use std::hash::Hasher;

use masonry::imaging::record::Glyph;
use masonry::imaging::{BlurredRoundedRect, ClipRef, FillRef, GeometryRef, GlyphRunRef, GroupRef, PaintSink, StrokeRef};
use masonry::kurbo::{Affine, Rect, RoundedRect};
use masonry::peniko::{BlendMode, BrushRef};

use super::Tracker;
use super::hash::H;
use super::stack::{clip_bounds, square};
use crate::render::paint::convert;
use crate::render::sprite::Sprite;

impl PaintSink for Tracker {
    fn push_clip(&mut self, clip: ClipRef<'_>) {
        let (hash, rect) = clip_bounds(&clip, self.lift(self.top().region));
        self.push(hash, rect, false, square(&clip));
    }

    fn pop_clip(&mut self) {
        self.pop();
    }

    fn push_group(&mut self, group: GroupRef<'_>) {
        let level = self.top();
        let (hash, rect) = group.clip.as_ref().map_or((0, self.frame), |clip| clip_bounds(clip, self.lift(level.region)));
        let mut h = H::new(12);
        h.0.write_u64(hash);
        h.composite(group.composite);
        if group.composite.alpha <= 0.0 {
            self.push(h.finish(), Rect::ZERO, true, false);
            return;
        }
        if group.mask.is_some() || !group.filters.is_empty() {
            let area = level.clip.intersect(rect);
            self.forced = Some(self.forced.map_or(area, |f| f.union(area)));
            if let Some(region) = level.region {
                self.regions[region].blocked = true;
            }
        }
        self.push(h.finish(), rect, group.composite.alpha < 1.0, false);
    }

    fn pop_group(&mut self) {
        self.pop();
    }

    fn fill(&mut self, draw: FillRef<'_>) {
        if convert::invisible(&draw.brush, draw.composite) {
            return;
        }
        let level = self.top();
        let mut h = H::new(1);
        h.affine(Affine::translate((0.0, self.lift(level.region))) * draw.transform);
        h.geometry(&draw.shape);
        h.brush(&draw.brush, draw.brush_transform);
        h.composite(draw.composite);
        h.fill_rule(draw.fill_rule);
        let bounds = draw.transform.transform_rect_bbox(convert::shape_bounds(&draw.shape));
        if let BrushRef::Image(image) = &draw.brush
            && let Some(color) = crate::render::sprite::color(image.image.data.id())
            && !level.faded
        {
            let radius = match draw.shape {
                GeometryRef::RoundedRect(rounded) => rounded.radii().top_left * draw.transform.as_coeffs()[0].abs(),
                _ => 0.0,
            };
            let shape = RoundedRect::from_rect(bounds, radius);
            self.sprites.push(Sprite { id: image.image.data.id(), shape, clip: level.clip, color, covered: false });
            return;
        }
        if let BrushRef::Image(image) = &draw.brush
            && crate::render::live::motion(image.image.data.id()).is_some()
            && !level.faded
            && let Some(shape) = crate::render::live::shape(&draw.shape)
        {
            self.live.push((image.image.data.id(), bounds.intersect(level.clip), draw.transform, shape));
        }
        let steady = self.steady(&draw.shape, draw.transform, &draw.brush, 0.0);
        let opaque = matches!(draw.brush, BrushRef::Solid(c) if c.components[3] >= 1.0) && draw.composite.alpha >= 1.0 && draw.composite.blend == BlendMode::default();
        if let (true, false, Some((y0, y1))) = (opaque, level.faded, steady) {
            let cover = bounds.intersect(level.clip);
            for region in self.regions.iter_mut().filter(|r| !r.started) {
                let vp = region.viewport;
                let middle = (vp.y0 + vp.y1) / 2.0;
                if cover.x0 <= vp.x0 + 0.5 && cover.x1 + 0.5 >= vp.x1 && y0 <= middle && y1 >= middle {
                    region.occlude(y0, y1);
                }
            }
        }
        self.add(h, bounds, steady, Self::solid_key(&draw));
    }

    fn stroke(&mut self, draw: StrokeRef<'_>) {
        if convert::invisible(&draw.brush, draw.composite) {
            return;
        }
        let mut h = H::new(2);
        h.affine(Affine::translate((0.0, self.lift(self.top().region))) * draw.transform);
        h.geometry(&draw.shape);
        h.brush(&draw.brush, draw.brush_transform);
        h.composite(draw.composite);
        h.stroke(draw.stroke);
        let w = draw.stroke.width.max(1.0) * draw.stroke.miter_limit.max(1.0);
        let bounds = draw.transform.transform_rect_bbox(convert::shape_bounds(&draw.shape).inflate(w, w));
        let steady = if draw.stroke.dash_pattern.is_empty() { self.steady(&draw.shape, draw.transform, &draw.brush, w) } else { None };
        self.add(h, bounds, steady, None);
    }

    fn glyph_run(&mut self, draw: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = Glyph>) {
        self.track_glyphs(draw, glyphs);
    }

    fn blurred_rounded_rect(&mut self, draw: BlurredRoundedRect) {
        let mut h = H::new(4);
        h.affine(Affine::translate((0.0, self.lift(self.top().region))) * draw.transform);
        h.rect(draw.rect);
        h.f64(draw.radius);
        h.f64(draw.std_dev);
        for v in draw.color.components {
            h.0.write_u32(v.to_bits());
        }
        h.composite(draw.composite);
        let spread = draw.std_dev * 3.0;
        let [_, skew_y, skew_x, scale_y, _, _] = draw.transform.as_coeffs();
        let steady = (skew_x == 0.0 && skew_y == 0.0 && scale_y > 0.0).then(|| {
            let inset = draw.radius + spread;
            let band = draw.transform.transform_rect_bbox(Rect::new(draw.rect.x0, draw.rect.y0 + inset, draw.rect.x1, draw.rect.y1 - inset));
            let clip = self.top().clip;
            (band.y0.max(clip.y0).ceil(), band.y1.min(clip.y1).floor())
        });
        self.add(h, draw.transform.transform_rect_bbox(draw.rect.inflate(spread, spread)), steady, None);
    }
}
