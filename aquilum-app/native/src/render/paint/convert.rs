//! Преобразования типов между Masonry/imaging и vello_cpu.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use masonry::imaging::{ClipRef, Composite, Filter, GeometryRef};
use masonry::kurbo::{Affine, BezPath, Rect, Shape as _, StrokeOpts, stroke};
use masonry::peniko::color::DynamicColor;
use masonry::peniko::{Brush, BrushRef, Fill, ImageData};
use vello_cpu::filter_effects::{EdgeMode, Filter as VelloFilter, FilterGraph, FilterPrimitive};
use vello_cpu::{Image, ImageSource, PaintType};

const IMAGE_FRAMES: u64 = 2;

#[derive(Default)]
pub struct Images {
    sources: HashMap<u64, (ImageSource, u64, Option<Weak<()>>)>,
    frame: u64,
    pub(super) red: bool,
}

impl Images {
    fn source(&mut self, data: &ImageData) -> ImageSource {
        let (frame, red) = (self.frame, self.red);
        let (source, used, _) = self.sources.entry(data.data.id()).or_insert_with(|| {
            if let Some((pixmap, owner)) = crate::render::surface::find(data.data.id()) {
                return (ImageSource::Pixmap(pixmap), frame, Some(owner));
            }
            let source = match crate::render::pixels::lookup(data.data.id()) {
                Some(pixmap) => ImageSource::Pixmap(pixmap),
                None if data.data.data().len() != data.width as usize * data.height as usize * 4 => ImageSource::Pixmap(Arc::new(vello_cpu::Pixmap::new(1, 1))),
                None => ImageSource::from_peniko_image_data(data),
            };
            (if red { reddened(source) } else { source }, frame, None)
        });
        *used = frame;
        source.clone()
    }

    pub fn next_frame(&mut self) {
        self.frame += 1;
        if self.red != crate::ui::theme::is_red() {
            self.red = !self.red;
            self.sources.clear();
        }
        let frame = self.frame;
        self.sources.retain(|_, (_, used, owner)| frame - *used < IMAGE_FRAMES && owner.as_ref().is_none_or(|o| o.strong_count() > 0));
    }
}

fn reddened(source: ImageSource) -> ImageSource {
    let ImageSource::Pixmap(pixmap) = source else { return source };
    let mut red = (*pixmap).clone();
    vello_cpu::redden(&mut red);
    ImageSource::Pixmap(Arc::new(red))
}

pub fn brush_to_paint(brush: BrushRef<'_>, composite: Composite, images: &mut Images) -> PaintType {
    let red = images.red;
    let ink = |color| if red { vello_cpu::red(color) } else { color };
    match brush.to_owned().multiply_alpha(composite.alpha) {
        Brush::Solid(c) => Brush::Solid(ink(c)),
        Brush::Gradient(mut g) => {
            for stop in g.stops.iter_mut() {
                stop.color = DynamicColor::from_alpha_color(ink(stop.color.to_alpha_color()));
            }
            Brush::Gradient(g)
        }
        Brush::Image(image) => Brush::Image(Image { image: images.source(&image.image), sampler: image.sampler }),
    }
}

pub fn geometry_to_path(geom: GeometryRef<'_>, tolerance: f64) -> BezPath {
    match geom {
        GeometryRef::Rect(r) => r.to_path(tolerance),
        GeometryRef::RoundedRect(rr) => rr.to_path(tolerance),
        GeometryRef::Path(p) => p.clone(),
        GeometryRef::OwnedPath(p) => p,
    }
}

pub fn clip_to_path(clip: ClipRef<'_>, tolerance: f64) -> (Affine, BezPath, Fill) {
    match clip {
        ClipRef::Fill { transform, shape, fill_rule } => {
            (transform, geometry_to_path(shape, tolerance), fill_rule)
        }
        ClipRef::Stroke { transform, shape, stroke: style } => {
            let path = geometry_to_path(shape, tolerance);
            let outline = stroke(path.iter(), style, &StrokeOpts::default(), tolerance);
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

pub fn line_bounds(y: f32, font_size: f32, glyph_transform: Option<Affine>, transform: Affine) -> Rect {
    let size = f64::from(font_size);
    let local = Rect::new(-1e7, f64::from(y) - size * 1.25, 1e7, f64::from(y) + size * 0.5);
    let local = glyph_transform.map_or(local, |g| g.transform_rect_bbox(local).union(local));
    transform.transform_rect_bbox(local)
}
