use std::hash::{BuildHasher, Hasher};

use foldhash::fast::FixedState;
use masonry::imaging::{ClipRef, Composite, GeometryRef};
use masonry::kurbo::{Affine, PathEl, Point, Rect, Stroke};
use masonry::peniko::{BlendMode, BrushRef, Fill, Gradient, GradientKind, Style};

use crate::render::paint::convert;

#[derive(Clone)]
pub(super) struct H(pub(super) foldhash::fast::FoldHasher<'static>);

impl H {
    pub(super) fn new(tag: u8) -> Self {
        let mut h = H(FixedState::default().build_hasher());
        h.0.write_u8(tag);
        h
    }

    pub(super) fn f64(&mut self, v: f64) {
        self.0.write_u64(v.to_bits());
    }

    pub(super) fn point(&mut self, p: Point) {
        self.f64(p.x);
        self.f64(p.y);
    }

    pub(super) fn affine(&mut self, a: Affine) {
        for c in a.as_coeffs() {
            self.0.write_i64((c * 4096.0).round() as i64);
        }
    }

    pub(super) fn rect(&mut self, r: Rect) {
        for v in [r.x0, r.y0, r.x1, r.y1] {
            self.f64(v);
        }
    }

    fn f32(&mut self, v: f32) {
        self.0.write_u32(v.to_bits());
    }

    fn gradient(&mut self, g: &Gradient) {
        match g.kind {
            GradientKind::Linear(p) => {
                self.0.write_u8(0);
                self.point(p.start);
                self.point(p.end);
            }
            GradientKind::Radial(p) => {
                self.0.write_u8(1);
                self.point(p.start_center);
                self.f32(p.start_radius);
                self.point(p.end_center);
                self.f32(p.end_radius);
            }
            GradientKind::Sweep(p) => {
                self.0.write_u8(2);
                self.point(p.center);
                self.f32(p.start_angle);
                self.f32(p.end_angle);
            }
        }
        self.0.write_u8(g.extend as u8);
        self.0.write_u8(g.interpolation_cs as u8);
        self.0.write_u8(g.hue_direction as u8);
        self.0.write_u8(g.interpolation_alpha_space as u8);
        for stop in g.stops.iter() {
            self.f32(stop.offset);
            self.0.write_u8(stop.color.cs as u8);
            for c in stop.color.components {
                self.f32(c);
            }
        }
    }

    pub(super) fn geometry(&mut self, g: &GeometryRef<'_>) {
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

    pub(super) fn path(&mut self, elements: &[PathEl]) {
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

    pub(super) fn brush(&mut self, b: &BrushRef<'_>, transform: Option<Affine>) {
        match b {
            BrushRef::Solid(c) => {
                for v in c.components {
                    self.0.write_u32(v.to_bits());
                }
            }
            BrushRef::Gradient(g) => self.gradient(g),
            BrushRef::Image(i) => {
                self.0.write_u64(i.image.data.id());
                self.0.write_u32(i.image.width);
                self.0.write_u32(i.image.height);
                self.0.write_u8(i.sampler.x_extend as u8);
                self.0.write_u8(i.sampler.y_extend as u8);
                self.0.write_u8(i.sampler.quality as u8);
                self.f32(i.sampler.alpha);
            }
        }
        if let Some(t) = transform {
            self.affine(t);
        }
    }

    pub(super) fn fill_rule(&mut self, rule: Fill) {
        self.0.write_u8(u8::from(rule == Fill::NonZero));
    }

    pub(super) fn stroke(&mut self, s: &Stroke) {
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

    pub(super) fn style(&mut self, style: &Style) {
        match style {
            Style::Fill(rule) => self.fill_rule(*rule),
            Style::Stroke(s) => self.stroke(s),
        }
    }

    pub(super) fn composite(&mut self, c: Composite) {
        self.0.write_u32(c.alpha.to_bits());
        if c.blend != BlendMode::default() {
            self.0.write_u8(c.blend.mix as u8);
            self.0.write_u8(c.blend.compose as u8);
        }
    }

    pub(super) fn finish(self) -> u64 {
        self.0.finish()
    }
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
