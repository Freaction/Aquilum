use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Transform};
use krilla::num::NormalizedF32;
use krilla::paint::FillRule;
use masonry::imaging::GeometryRef;
use masonry::kurbo::{Affine, PathEl};
use masonry::peniko::{self, BrushRef, Color};

pub(super) fn path(transform: &Affine, shape: &GeometryRef<'_>) -> Option<Path> {
    let mut outline = crate::render::geometry_to_path(shape.clone());
    outline.apply_affine(*transform);
    let mut builder = PathBuilder::new();
    for el in outline.elements() {
        match *el {
            PathEl::MoveTo(p) => builder.move_to(p.x as f32, p.y as f32),
            PathEl::LineTo(p) => builder.line_to(p.x as f32, p.y as f32),
            PathEl::QuadTo(a, p) => builder.quad_to(a.x as f32, a.y as f32, p.x as f32, p.y as f32),
            PathEl::CurveTo(a, b, p) => builder.cubic_to(a.x as f32, a.y as f32, b.x as f32, b.y as f32, p.x as f32, p.y as f32),
            PathEl::ClosePath => builder.close(),
        }
    }
    builder.finish()
}

pub(super) fn rule(fill: peniko::Fill) -> FillRule {
    match fill {
        peniko::Fill::NonZero => FillRule::NonZero,
        peniko::Fill::EvenOdd => FillRule::EvenOdd,
    }
}

fn solid(brush: &BrushRef<'_>) -> Option<Color> {
    match brush {
        BrushRef::Solid(color) => Some(*color),
        BrushRef::Gradient(gradient) => gradient.stops.first().map(|stop| stop.color.to_alpha_color()),
        BrushRef::Image(_) => None,
    }
}

pub(super) fn ink(brush: &BrushRef<'_>, alpha: f32) -> Option<(rgb::Color, NormalizedF32)> {
    let (color, opacity) = paint(solid(brush)?, alpha);
    (opacity.get() > 0.0).then_some((color, opacity))
}

fn paint(color: Color, alpha: f32) -> (rgb::Color, NormalizedF32) {
    let c = color.to_rgba8();
    (rgb::Color::new(c.r, c.g, c.b), NormalizedF32::new((f32::from(c.a) / 255.0 * alpha).clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ONE))
}

pub(super) fn transform(affine: Affine) -> Transform {
    let [a, b, c, d, e, f] = affine.as_coeffs();
    Transform::from_row(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32)
}

pub(super) fn scale_of(affine: &Affine) -> f64 {
    affine.determinant().abs().sqrt()
}
