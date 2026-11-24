//! Преобразования типов между Masonry/imaging и vello_cpu.

use std::collections::HashMap;
use std::sync::Arc;

use masonry::imaging::{ClipRef, Composite, Filter, GeometryRef};
use masonry::kurbo::{Affine, BezPath, Rect, Shape as _, Stroke, StrokeOpts, stroke};
use masonry::peniko::{Brush, BrushRef, Fill, ImageData};
use vello_cpu::filter_effects::{EdgeMode, Filter as VelloFilter, FilterGraph, FilterPrimitive};
use vello_cpu::{Image, ImageSource, PaintType};

const IMAGE_FRAMES: u64 = 2;

pub const TOLERANCE: f64 = 0.1;

#[derive(Default)]
pub struct Images {
    sources: HashMap<u64, (ImageSource, u64)>,
    frame: u64,
}

impl Images {
    fn source(&mut self, data: &ImageData) -> ImageSource {
        let frame = self.frame;
        let (source, used) = self.sources.entry(data.data.id()).or_insert_with(|| {
            let source = match crate::render::pixels::lookup(data.data.id()) {
                Some((pixmap, _)) => ImageSource::Pixmap(pixmap),
                None => ImageSource::from_peniko_image_data(data),
            };
            (source, frame)
        });
        *used = frame;
        source.clone()
    }

    pub fn next_frame(&mut self) {
        self.frame += 1;
        let frame = self.frame;
        self.sources.retain(|_, (_, used)| frame - *used < IMAGE_FRAMES);
    }
}

pub fn brush_to_paint(brush: BrushRef<'_>, composite: Composite, images: &mut Images) -> PaintType {
    match brush.to_owned().multiply_alpha(composite.alpha) {
        Brush::Solid(c) => Brush::Solid(c),
        Brush::Gradient(g) => Brush::Gradient(g),
        Brush::Image(image) => Brush::Image(Image { image: images.source(&image.image), sampler: image.sampler }),
    }
}

pub fn geometry_to_path(geom: GeometryRef<'_>) -> BezPath {
    match geom {
        GeometryRef::Rect(r) => r.to_path(TOLERANCE),
        GeometryRef::RoundedRect(rr) => rr.to_path(TOLERANCE),
        GeometryRef::Path(p) => p.clone(),
        GeometryRef::OwnedPath(p) => p,
    }
}

pub fn clip_to_path(clip: ClipRef<'_>) -> (Affine, BezPath, Fill) {
    match clip {
        ClipRef::Fill { transform, shape, fill_rule } => (transform, geometry_to_path(shape), fill_rule),
        ClipRef::Stroke { transform, shape, stroke: style } => {
            let path = geometry_to_path(shape);
            let outline = stroke(path.iter(), style, &StrokeOpts::default(), TOLERANCE);
            (transform, outline, Fill::NonZero)
        }
    }
}

pub fn filters_to_vello(filters: &[Filter]) -> Option<VelloFilter> {
    let mut graph = FilterGraph::new();
    let mut last = None;
    for f in filters {
        let primitive = match *f {
            Filter::Flood { color } => FilterPrimitive::Flood { color },
            Filter::Blur { std_deviation_x, std_deviation_y } => FilterPrimitive::GaussianBlur {
                std_deviation: std_deviation_x.max(std_deviation_y),
                edge_mode: EdgeMode::None,
            },
            Filter::DropShadow { dx, dy, std_deviation_x, std_deviation_y, color } => {
                FilterPrimitive::DropShadow {
                    dx,
                    dy,
                    std_deviation: std_deviation_x.max(std_deviation_y),
                    color,
                    edge_mode: EdgeMode::None,
                }
            }
            Filter::Offset { dx, dy } => FilterPrimitive::Offset { dx, dy },
        };
        last = Some(graph.add(primitive, None));
    }
    let out = last?;
    graph.set_output(out);
    Some(VelloFilter { graph: Arc::new(graph) })
}

pub fn invisible(brush: &BrushRef<'_>, composite: Composite) -> bool {
    composite.alpha <= 0.0 || composite.blend == masonry::peniko::BlendMode::default() && matches!(brush, BrushRef::Solid(color) if color.components[3] <= 0.0)
}

pub fn shape_bounds(shape: &GeometryRef<'_>) -> Rect {
    match shape {
        GeometryRef::Rect(r) => *r,
        GeometryRef::RoundedRect(rr) => rr.rect(),
        GeometryRef::Path(p) => p.bounding_box(),
        GeometryRef::OwnedPath(p) => p.bounding_box(),
    }
}

pub fn stroke_reach(stroke: &Stroke) -> f64 {
    stroke.width.max(1.0) * stroke.miter_limit.max(1.0)
}

pub fn stroke_bounds(shape: &GeometryRef<'_>, stroke: &Stroke, transform: Affine) -> Rect {
    let reach = stroke_reach(stroke);
    transform.transform_rect_bbox(shape_bounds(shape).inflate(reach, reach))
}

pub fn line_bounds(y: f32, font_size: f32, glyph_transform: Option<Affine>, transform: Affine) -> Rect {
    let size = f64::from(font_size);
    let local = Rect::new(-1e7, f64::from(y) - size * 1.25, 1e7, f64::from(y) + size * 0.5);
    let local = glyph_transform.map_or(local, |g| g.transform_rect_bbox(local).union(local));
    transform.transform_rect_bbox(local)
}
