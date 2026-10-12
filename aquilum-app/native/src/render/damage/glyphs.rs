use std::hash::Hasher;

use masonry::imaging::GlyphRunRef;
use masonry::imaging::record::Glyph;
use masonry::kurbo::{Affine, Rect};

use super::Tracker;
use super::hash::H;
use crate::render::paint::convert;

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
    pub(super) fn track_glyphs(&mut self, draw: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = Glyph>) {
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
}
