use std::fmt::Debug;
use std::hash::{BuildHasher, Hasher};

use foldhash::fast::FixedState;
use masonry::imaging::record::Glyph;
use masonry::imaging::{BlurredRoundedRect, ClipRef, Composite, FillRef, GeometryRef, GlyphRunRef, GroupRef, PaintSink, StrokeRef};
use masonry::kurbo::{Affine, PathEl, Point, Rect, Stroke};
use masonry::peniko::{BlendMode, BrushRef, Fill, Style};

use super::paint::convert;

#[derive(Clone, Copy)]
struct Draw {
    hash: u64,
    rect: Rect,
}

pub struct Tracker {
    frame: Rect,
    stack: Vec<(u64, Rect)>,
    draws: Vec<Draw>,
    previous: Vec<Draw>,
    forced: Option<Rect>,
}

struct H(foldhash::fast::FoldHasher<'static>);

impl H {
    fn new(tag: u8) -> Self {
        let mut h = H(FixedState::default().build_hasher());
        h.0.write_u8(tag);
        h
    }

    fn f64(&mut self, v: f64) {
        self.0.write_u64(v.to_bits());
    }

    fn point(&mut self, p: Point) {
        self.f64(p.x);
        self.f64(p.y);
    }

    fn affine(&mut self, a: Affine) {
        for c in a.as_coeffs() {
            self.f64(c);
        }
    }

    fn rect(&mut self, r: Rect) {
        for v in [r.x0, r.y0, r.x1, r.y1] {
            self.f64(v);
        }
    }

    fn debug(&mut self, v: &impl Debug) {
        self.0.write(format!("{v:?}").as_bytes());
    }

    fn geometry(&mut self, g: &GeometryRef<'_>) {
        match g {
            GeometryRef::Rect(r) => self.rect(*r),
            GeometryRef::RoundedRect(rr) => {
                self.rect(rr.rect());
                let r = rr.radii();
                for v in [r.top_left, r.top_right, r.bottom_right, r.bottom_left] {
                    self.f64(v);
                }
            }
            GeometryRef::Path(p) => self.path(p.elements()),
            GeometryRef::OwnedPath(p) => self.path(p.elements()),
        }
    }

    fn path(&mut self, elements: &[PathEl]) {
        for el in elements {
            match *el {
                PathEl::MoveTo(p) => {
                    self.0.write_u8(0);
                    self.point(p);
                }
                PathEl::LineTo(p) => {
                    self.0.write_u8(1);
                    self.point(p);
                }
                PathEl::QuadTo(a, b) => {
                    self.0.write_u8(2);
                    self.point(a);
                    self.point(b);
                }
                PathEl::CurveTo(a, b, c) => {
                    self.0.write_u8(3);
                    self.point(a);
                    self.point(b);
                    self.point(c);
                }
                PathEl::ClosePath => self.0.write_u8(4),
            }
        }
    }

    fn brush(&mut self, b: &BrushRef<'_>, transform: Option<Affine>) {
        match b {
            BrushRef::Solid(c) => {
                for v in c.components {
                    self.0.write_u32(v.to_bits());
                }
            }
            BrushRef::Gradient(g) => self.debug(g),
            BrushRef::Image(i) => {
                self.0.write_u64(i.image.data.id());
                self.0.write_u32(i.image.width);
                self.0.write_u32(i.image.height);
                self.debug(&i.sampler);
            }
        }
        if let Some(t) = transform {
            self.affine(t);
        }
    }

    fn fill_rule(&mut self, rule: Fill) {
        self.0.write_u8(u8::from(rule == Fill::NonZero));
    }

    fn stroke(&mut self, s: &Stroke) {
        self.f64(s.width);
        self.f64(s.miter_limit);
        self.f64(s.dash_offset);
        self.0.write_u8(s.join as u8);
        self.0.write_u8(s.start_cap as u8);
        self.0.write_u8(s.end_cap as u8);
        for d in &s.dash_pattern {
            self.f64(*d);
        }
    }

    fn style(&mut self, style: &Style) {
        match style {
            Style::Fill(rule) => self.fill_rule(*rule),
            Style::Stroke(s) => self.stroke(s),
        }
    }

    fn composite(&mut self, c: Composite) {
        self.0.write_u32(c.alpha.to_bits());
        if c.blend != BlendMode::default() {
            self.debug(&c.blend);
        }
    }

    fn finish(self) -> u64 {
        self.0.finish()
    }
}

fn clip_bounds(clip: &ClipRef<'_>) -> (u64, Rect) {
    let mut h = H::new(10);
    let rect = match clip {
        ClipRef::Fill { transform, shape, fill_rule } => {
            h.affine(*transform);
            h.geometry(shape);
            h.fill_rule(*fill_rule);
            transform.transform_rect_bbox(convert::shape_bounds(shape))
        }
        ClipRef::Stroke { transform, shape, stroke } => {
            h.affine(*transform);
            h.geometry(shape);
            h.stroke(stroke);
            let w = stroke.width.max(1.0) * stroke.miter_limit.max(1.0);
            transform.transform_rect_bbox(convert::shape_bounds(shape).inflate(w, w))
        }
    };
    (h.finish(), rect)
}

impl Tracker {
    pub fn new() -> Self {
        Tracker { frame: Rect::ZERO, stack: Vec::new(), draws: Vec::new(), previous: Vec::new(), forced: None }
    }

    pub fn forget(&mut self) {
        self.previous.clear();
    }

    pub fn begin(&mut self, width: u16, height: u16) {
        self.frame = Rect::new(0.0, 0.0, f64::from(width), f64::from(height));
        self.stack.clear();
        self.stack.push((0, self.frame));
        self.draws.clear();
        self.forced = None;
    }

    pub fn damage(&mut self) -> Option<(u16, u16, u16, u16)> {
        self.draws.sort_unstable_by_key(|d| d.hash);
        let mut damage = self.forced;
        let mut add = |r: Rect| damage = Some(damage.map_or(r, |d| d.union(r)));
        let (mut i, mut j) = (0, 0);
        while i < self.draws.len() || j < self.previous.len() {
            match (self.draws.get(i), self.previous.get(j)) {
                (Some(a), Some(b)) if a.hash == b.hash => {
                    i += 1;
                    j += 1;
                }
                (Some(a), Some(b)) if a.hash < b.hash => {
                    add(a.rect);
                    i += 1;
                }
                (Some(a), None) => {
                    add(a.rect);
                    i += 1;
                }
                (_, Some(b)) => {
                    add(b.rect);
                    j += 1;
                }
                (None, None) => unreachable!(),
            }
        }
        std::mem::swap(&mut self.draws, &mut self.previous);
        let d = damage?.inflate(1.0, 1.0).expand();
        let d = Rect::new((d.x0 / 4.0).floor() * 4.0, (d.y0 / 4.0).floor() * 4.0, d.x1, d.y1).intersect(self.frame);
        (d.width() > 0.0 && d.height() > 0.0).then(|| (d.x0 as u16, d.y0 as u16, d.width() as u16, d.height() as u16))
    }

    fn add(&mut self, mut h: H, bounds: Rect) {
        let (state, clip) = *self.stack.last().expect("корень стека");
        let rect = bounds.intersect(clip);
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        h.0.write_u64(state);
        self.draws.push(Draw { hash: h.finish(), rect });
    }

    fn push(&mut self, hash: u64, rect: Rect) {
        let (state, clip) = *self.stack.last().expect("корень стека");
        let mut h = H::new(11);
        h.0.write_u64(state);
        h.0.write_u64(hash);
        self.stack.push((h.finish(), clip.intersect(rect)));
    }

    fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }
}

impl PaintSink for Tracker {
    fn push_clip(&mut self, clip: ClipRef<'_>) {
        let (hash, rect) = clip_bounds(&clip);
        self.push(hash, rect);
    }

    fn pop_clip(&mut self) {
        self.pop();
    }

    fn push_group(&mut self, group: GroupRef<'_>) {
        let (hash, rect) = group.clip.as_ref().map_or((0, self.frame), clip_bounds);
        let mut h = H::new(12);
        h.0.write_u64(hash);
        h.composite(group.composite);
        if group.mask.is_some() || !group.filters.is_empty() {
            let area = self.stack.last().expect("корень стека").1.intersect(rect);
            self.forced = Some(self.forced.map_or(area, |f| f.union(area)));
        }
        self.push(h.finish(), rect);
    }

    fn pop_group(&mut self) {
        self.pop();
    }

    fn fill(&mut self, draw: FillRef<'_>) {
        let mut h = H::new(1);
        h.affine(draw.transform);
        h.geometry(&draw.shape);
        h.brush(&draw.brush, draw.brush_transform);
        h.composite(draw.composite);
        h.fill_rule(draw.fill_rule);
        let bounds = draw.transform.transform_rect_bbox(convert::shape_bounds(&draw.shape));
        self.add(h, bounds);
    }

    fn stroke(&mut self, draw: StrokeRef<'_>) {
        let mut h = H::new(2);
        h.affine(draw.transform);
        h.geometry(&draw.shape);
        h.brush(&draw.brush, draw.brush_transform);
        h.composite(draw.composite);
        h.stroke(draw.stroke);
        let w = draw.stroke.width.max(1.0) * draw.stroke.miter_limit.max(1.0);
        let bounds = draw.transform.transform_rect_bbox(convert::shape_bounds(&draw.shape).inflate(w, w));
        self.add(h, bounds);
    }

    fn glyph_run(&mut self, draw: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = Glyph>) {
        let Some(first) = glyphs.next() else { return };
        let clip = self.stack.last().expect("корень стека").1;
        if !clip.overlaps(convert::line_bounds(first.y, draw.font_size, draw.glyph_transform, draw.transform)) {
            return;
        }
        let mut h = H::new(3);
        h.0.write_u64(draw.font.data.id());
        h.0.write_u32(draw.font.index);
        h.0.write_u32(draw.font_size.to_bits());
        h.0.write_u8(u8::from(draw.hint));
        h.affine(draw.transform);
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
        let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
        for g in std::iter::once(first).chain(glyphs) {
            h.0.write_u32(g.id);
            h.0.write_u32(g.x.to_bits());
            h.0.write_u32(g.y.to_bits());
            (x0, y0, x1, y1) = (x0.min(g.x), y0.min(g.y), x1.max(g.x), y1.max(g.y));
        }
        let size = f64::from(draw.font_size);
        let local = Rect::new(f64::from(x0) - size * 0.5, f64::from(y0) - size * 1.25, f64::from(x1) + size * 1.5, f64::from(y1) + size * 0.5);
        let local = draw.glyph_transform.map_or(local, |g| g.transform_rect_bbox(local).union(local));
        self.add(h, draw.transform.transform_rect_bbox(local));
    }

    fn blurred_rounded_rect(&mut self, draw: BlurredRoundedRect) {
        let mut h = H::new(4);
        h.affine(draw.transform);
        h.rect(draw.rect);
        h.f64(draw.radius);
        h.f64(draw.std_dev);
        for v in draw.color.components {
            h.0.write_u32(v.to_bits());
        }
        h.composite(draw.composite);
        let spread = draw.std_dev * 3.0;
        self.add(h, draw.transform.transform_rect_bbox(draw.rect.inflate(spread, spread)));
    }
}
