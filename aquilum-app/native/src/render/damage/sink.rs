use std::hash::Hasher;

use masonry::imaging::record::Glyph;
use masonry::imaging::{BlurredRoundedRect, ClipRef, FillRef, GeometryRef, GlyphRunRef, GroupRef, PaintSink, StrokeRef};
use masonry::kurbo::{Affine, Rect, RoundedRect};
use masonry::peniko::{BlendMode, BrushRef};

use super::hash::{H, clip_bounds};
use super::{Draw, Level, Solid, Tracker, close, overlap};
use crate::render::paint::convert;
use crate::render::sprite::Sprite;

const CELL: f32 = 48.0;

struct Piece {
    cell: i32,
    h: H,
    block: [u32; 96],
    filled: usize,
    extent: (f32, f32, f32, f32),
}

impl Piece {
    fn push(&mut self, g: Glyph) {
        self.block[self.filled..self.filled + 3].copy_from_slice(&[g.id, g.x.to_bits(), g.y.to_bits()]);
        self.filled += 3;
        if self.filled == self.block.len() {
            self.h.0.write(bytemuck::cast_slice(&self.block));
            self.filled = 0;
        }
        let (x0, y0, x1, y1) = self.extent;
        self.extent = (x0.min(g.x), y0.min(g.y), x1.max(g.x), y1.max(g.y));
    }

    fn finish(mut self, size: f64, glyph_transform: Option<Affine>, transform: Affine) -> (H, Rect) {
        self.h.0.write(bytemuck::cast_slice(&self.block[..self.filled]));
        let (x0, y0, x1, y1) = self.extent;
        let local = Rect::new(f64::from(x0) - size * 0.5, f64::from(y0) - size * 1.25, f64::from(x1) + size * 1.5, f64::from(y1) + size * 0.5);
        let local = glyph_transform.map_or(local, |g| g.transform_rect_bbox(local).union(local));
        (self.h, transform.transform_rect_bbox(local))
    }
}

impl Tracker {
    fn top(&self) -> Level {
        *self.stack.last().expect("корень стека")
    }

    fn lift(&self, region: Option<usize>) -> f64 {
        region.map_or(0.0, |r| self.regions[r].offset)
    }

    fn add(&mut self, mut h: H, bounds: Rect, steady: Option<(f64, f64)>, solid: Option<(H, f64)>) {
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

    fn solid_key(draw: &FillRef<'_>) -> Option<(H, f64)> {
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

    fn push(&mut self, hash: u64, rect: Rect, faded: bool) {
        let level = self.top();
        let mut h = H::new(11);
        h.0.write_u64(level.state);
        h.0.write_u64(hash);
        let found = level.region.is_none().then(|| self.regions.iter().position(|r| !r.started && close(r.bounds, rect))).flatten();
        if let Some(found) = found {
            self.regions[found].started = true;
        }
        let clip = level.clip.intersect(found.map_or(rect, |i| self.regions[i].viewport));
        let region = level.region.or(found);
        self.stack.push(Level { state: h.finish(), clip, region, faded: level.faded || faded });
    }

    fn pop(&mut self) {
        if self.stack.len() > 1 {
            let level = self.stack.pop().expect("уровень стека");
            if let Some(region) = level.region
                && self.top().region != Some(region)
            {
                self.regions[region].ended = true;
            }
        }
    }

    fn steady(&self, shape: &GeometryRef<'_>, transform: Affine, brush: &BrushRef<'_>, band: f64) -> Option<(f64, f64)> {
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

impl PaintSink for Tracker {
    fn push_clip(&mut self, clip: ClipRef<'_>) {
        let (hash, rect) = clip_bounds(&clip, self.lift(self.top().region));
        self.push(hash, rect, false);
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
            self.push(h.finish(), Rect::ZERO, true);
            return;
        }
        if group.mask.is_some() || !group.filters.is_empty() {
            let area = level.clip.intersect(rect);
            self.forced = Some(self.forced.map_or(area, |f| f.union(area)));
            if let Some(region) = level.region {
                self.regions[region].blocked = true;
            }
        }
        self.push(h.finish(), rect, group.composite.alpha < 1.0);
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
                if cover.x0 <= vp.x0 && cover.x1 >= vp.x1 && y0 <= middle && y1 >= middle {
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
        let Some(first) = glyphs.next() else { return };
        let level = self.top();
        if !level.clip.overlaps(convert::line_bounds(first.y, draw.font_size, draw.glyph_transform, draw.transform)) {
            return;
        }
        let mut h = H::new(3);
        h.0.write_u64(draw.font.data.id());
        h.0.write_u32(draw.font.index);
        h.0.write_u32(draw.font_size.to_bits());
        h.0.write_u8(u8::from(draw.hint));
        h.affine(Affine::translate((0.0, self.lift(level.region))) * draw.transform);
        if let Some(g) = draw.glyph_transform {
            h.affine(g);
        }
        h.f64(draw.font_embolden.x);
        h.f64(draw.font_embolden.y);
        for c in draw.normalized_coords {
            h.0.write_i16(*c);
        }
        h.style(draw.style);
        h.brush(&draw.brush, draw.brush_transform);
        h.composite(draw.composite);
        let size = f64::from(draw.font_size);
        let mut piece: Option<Piece> = None;
        for g in std::iter::once(first).chain(glyphs) {
            let cell = (g.x / CELL).floor() as i32;
            if let Some(done) = piece.take_if(|p| p.cell != cell) {
                let (h, rect) = done.finish(size, draw.glyph_transform, draw.transform);
                self.add(h, rect, None, None);
            }
            piece.get_or_insert_with(|| Piece { cell, h: h.clone(), block: [0; 96], filled: 0, extent: (g.x, g.y, g.x, g.y) }).push(g);
        }
        if let Some(done) = piece {
            let (h, rect) = done.finish(size, draw.glyph_transform, draw.transform);
            self.add(h, rect, None, None);
        }
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
