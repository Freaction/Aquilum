use std::ops::Range;

use krilla::geom::Point as KPoint;
use krilla::paint::{Fill, FillRule};
use krilla::surface::Location;
use krilla::text::{Glyph, GlyphId};
use masonry::imaging::GlyphRunRef;
use masonry::imaging::record::Glyph as SceneGlyph;
use masonry::kurbo::Rect;

use super::PdfSink;
use super::geometry::{paint, solid, transform};

struct Placed {
    id: GlyphId,
    text: Range<usize>,
    advance: f32,
}

impl Glyph for Placed {
    fn glyph_id(&self) -> GlyphId {
        self.id
    }

    fn text_range(&self) -> Range<usize> {
        self.text.clone()
    }

    fn x_advance(&self, size: f32) -> f32 {
        self.advance * size
    }

    fn x_offset(&self, _size: f32) -> f32 {
        0.0
    }

    fn y_offset(&self, _size: f32) -> f32 {
        0.0
    }

    fn y_advance(&self, _size: f32) -> f32 {
        0.0
    }

    fn location(&self) -> Option<Location> {
        None
    }
}

impl PdfSink<'_, '_> {
    pub(super) fn draw_glyph_run(&mut self, draw: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = SceneGlyph>) {
        let glyphs: Vec<SceneGlyph> = glyphs.collect();
        let Some(first) = glyphs.first() else { return };
        let size = f64::from(draw.font_size);
        let last = glyphs.last().map_or(first.x, |g| g.x);
        let line = Rect::new(f64::from(first.x) - size, f64::from(first.y) - size * 1.5, f64::from(last) + size * 2.0, f64::from(first.y) + size);
        if draw.composite.alpha <= 0.0 || !self.visible(draw.transform, line) {
            return;
        }
        let Some(color) = solid(&draw.brush) else { return };
        let (color, opacity) = paint(color, draw.composite.alpha);
        if opacity.get() <= 0.0 || draw.font_size <= 0.0 {
            return;
        }
        let Some((font, chars)) = self.fonts.get(draw.font, draw.normalized_coords) else { return };
        self.surface.set_stroke(None);
        self.surface.set_fill(Some(Fill { paint: color.into(), opacity, rule: FillRule::NonZero }));
        self.surface.push_transform(&transform(self.base * draw.transform));
        let mut start = 0;
        while start < glyphs.len() {
            let y = glyphs[start].y;
            let end = glyphs[start..].iter().position(|g| g.y != y).map_or(glyphs.len(), |n| start + n);
            let mut text = String::new();
            let placed: Vec<Placed> = glyphs[start..end]
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let from = text.len();
                    if let Some(c) = chars.get(&g.id) {
                        text.push(*c);
                    }
                    let next = glyphs.get(start + i + 1).filter(|_| start + i + 1 < end).map_or(g.x, |n| n.x);
                    Placed { id: GlyphId::new(g.id), text: from..text.len(), advance: (next - g.x) / draw.font_size }
                })
                .collect();
            self.surface.draw_glyphs(KPoint::from_xy(glyphs[start].x, y), &placed, font.clone(), &text, draw.font_size, false);
            start = end;
        }
        self.surface.pop();
    }
}
