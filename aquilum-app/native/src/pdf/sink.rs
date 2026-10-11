use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Point as KPoint, Size as KSize, Transform};
use krilla::image::{BitsPerComponent, CustomImage, Image, ImageColorspace};
use krilla::num::NormalizedF32;
use krilla::paint::{Fill, FillRule, LineCap, LineJoin, Stroke};
use krilla::surface::{Location, Surface};
use krilla::text::{Font, Glyph, GlyphId, Tag};
use masonry::imaging::record::Glyph as SceneGlyph;
use masonry::imaging::{BlurredRoundedRect, ClipRef, FillRef, GeometryRef, GlyphRunRef, GroupRef, PaintSink, StrokeRef};
use masonry::kurbo::{self, Affine, BezPath, PathEl, Rect, Shape};
use masonry::peniko::{self, BrushRef, Color, ImageAlphaType, ImageFormat};
use skrifa::MetadataProvider;

const TOLERANCE: f64 = 0.1;

pub struct Fonts {
    fonts: HashMap<(u64, u32, Vec<i16>), (Font, Arc<HashMap<u32, char>>)>,
}

impl Fonts {
    pub fn new() -> Self {
        Fonts { fonts: HashMap::new() }
    }

    fn get(&mut self, data: &peniko::FontData, coords: &[i16]) -> Option<(Font, Arc<HashMap<u32, char>>)> {
        let key = (data.data.id(), data.index, coords.to_vec());
        if let Some(found) = self.fonts.get(&key) {
            return Some(found.clone());
        }
        let bytes: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(data.data.clone());
        let parsed = skrifa::FontRef::from_index(data.data.data(), data.index).ok()?;
        let variations: Vec<(Tag, f32)> = parsed
            .axes()
            .iter()
            .zip(coords)
            .map(|(axis, &n)| {
                let n = f32::from(n) / 16384.0;
                let value = if n >= 0.0 { axis.default_value() + n * (axis.max_value() - axis.default_value()) } else { axis.default_value() + n * (axis.default_value() - axis.min_value()) };
                (Tag::new(&axis.tag().to_be_bytes()), value)
            })
            .collect();
        let mut chars = HashMap::new();
        for (code, glyph) in parsed.charmap().mappings() {
            if let Some(c) = char::from_u32(code) {
                chars.entry(glyph.to_u32()).or_insert(c);
            }
        }
        let font = Font::new_variable(krilla::Data::from(bytes), data.index, &variations)?;
        let entry = (font, Arc::new(chars));
        self.fonts.insert(key, entry.clone());
        Some(entry)
    }
}

#[derive(Clone)]
struct Pixels {
    id: u64,
    size: (u32, u32),
    rgb: Arc<Vec<u8>>,
    alpha: Arc<Vec<u8>>,
}

impl Hash for Pixels {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.size.hash(state);
    }
}

impl CustomImage for Pixels {
    fn color_channel(&self) -> &[u8] {
        &self.rgb
    }

    fn alpha_channel(&self) -> Option<&[u8]> {
        Some(&self.alpha)
    }

    fn bits_per_component(&self) -> BitsPerComponent {
        BitsPerComponent::Eight
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn icc_profile(&self) -> Option<&[u8]> {
        None
    }

    fn color_space(&self) -> ImageColorspace {
        ImageColorspace::Rgb
    }
}

fn pixels(image: &peniko::ImageData) -> Pixels {
    let bytes = image.data.data();
    let count = (image.width * image.height) as usize;
    let (mut rgb, mut alpha) = (Vec::with_capacity(count * 3), Vec::with_capacity(count));
    for px in bytes.chunks_exact(4).take(count) {
        let (r, g, b) = match image.format {
            ImageFormat::Bgra8 => (px[2], px[1], px[0]),
            _ => (px[0], px[1], px[2]),
        };
        let a = px[3];
        let straight = |c: u8| if image.alpha_type == ImageAlphaType::AlphaPremultiplied && a > 0 && a < 255 { ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8 } else { c };
        rgb.extend([straight(r), straight(g), straight(b)]);
        alpha.push(a);
    }
    Pixels { id: image.data.id(), size: (image.width, image.height), rgb: Arc::new(rgb), alpha: Arc::new(alpha) }
}

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

pub struct PdfSink<'s, 'a> {
    pub surface: &'s mut Surface<'a>,
    pub base: Affine,
    pub page: Rect,
    pub fonts: &'s mut Fonts,
    groups: Vec<usize>,
}

impl<'s, 'a> PdfSink<'s, 'a> {
    pub fn new(surface: &'s mut Surface<'a>, base: Affine, page: Rect, fonts: &'s mut Fonts) -> Self {
        PdfSink { surface, base, page, fonts, groups: Vec::new() }
    }

    fn visible(&self, transform: Affine, bounds: Rect) -> bool {
        let rect = (self.base * transform).transform_rect_bbox(bounds);
        rect.x1 >= self.page.x0 && rect.x0 <= self.page.x1 && rect.y1 >= self.page.y0 && rect.y0 <= self.page.y1
    }

    fn clip(&mut self, transform: Affine, shape: &GeometryRef<'_>, rule: FillRule) -> usize {
        match path(&(self.base * transform), shape) {
            Some(path) => {
                self.surface.push_clip_path(&path, &rule);
                1
            }
            None => 0,
        }
    }
}

fn outline(shape: &GeometryRef<'_>) -> BezPath {
    match shape {
        GeometryRef::Rect(r) => r.to_path(TOLERANCE),
        GeometryRef::RoundedRect(r) => r.to_path(TOLERANCE),
        GeometryRef::Path(p) => (*p).clone(),
        GeometryRef::OwnedPath(p) => p.clone(),
    }
}

fn bounds(shape: &GeometryRef<'_>) -> Rect {
    match shape {
        GeometryRef::Rect(r) => *r,
        GeometryRef::RoundedRect(r) => r.rect(),
        GeometryRef::Path(p) => p.bounding_box(),
        GeometryRef::OwnedPath(p) => p.bounding_box(),
    }
}

fn path(transform: &Affine, shape: &GeometryRef<'_>) -> Option<Path> {
    let mut outline = outline(shape);
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

fn rule(fill: peniko::Fill) -> FillRule {
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

fn paint(color: Color, alpha: f32) -> (rgb::Color, NormalizedF32) {
    let c = color.to_rgba8();
    (rgb::Color::new(c.r, c.g, c.b), NormalizedF32::new((f32::from(c.a) / 255.0 * alpha).clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ONE))
}

fn transform(affine: Affine) -> Transform {
    let [a, b, c, d, e, f] = affine.as_coeffs();
    Transform::from_row(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32)
}

fn scale_of(affine: &Affine) -> f64 {
    affine.determinant().abs().sqrt()
}

impl PaintSink for PdfSink<'_, '_> {
    fn push_clip(&mut self, clip: ClipRef<'_>) {
        let pushed = match clip {
            ClipRef::Fill { transform, shape, fill_rule } => self.clip(transform, &shape, rule(fill_rule)),
            ClipRef::Stroke { transform, shape, .. } => self.clip(transform, &shape, FillRule::NonZero),
        };
        self.groups.push(pushed);
    }

    fn pop_clip(&mut self) {
        for _ in 0..self.groups.pop().unwrap_or(0) {
            self.surface.pop();
        }
    }

    fn push_group(&mut self, group: GroupRef<'_>) {
        let mut pushed = 0;
        if let Some(clip) = &group.clip {
            pushed += match clip {
                ClipRef::Fill { transform, shape, fill_rule } => self.clip(*transform, shape, rule(*fill_rule)),
                ClipRef::Stroke { transform, shape, .. } => self.clip(*transform, shape, FillRule::NonZero),
            };
        }
        if group.composite.alpha < 1.0 {
            self.surface.push_opacity(NormalizedF32::new(group.composite.alpha.max(0.0)).unwrap_or(NormalizedF32::ZERO));
            pushed += 1;
        }
        self.groups.push(pushed);
    }

    fn pop_group(&mut self) {
        self.pop_clip();
    }

    fn fill(&mut self, draw: FillRef<'_>) {
        if draw.composite.alpha <= 0.0 || !self.visible(draw.transform, bounds(&draw.shape)) {
            return;
        }
        if let BrushRef::Image(brush) = &draw.brush {
            let pushed = self.clip(draw.transform, &draw.shape, rule(draw.fill_rule));
            let image = Image::from_custom(pixels(&brush.image), true);
            let placed = self.base * draw.transform * draw.brush_transform.unwrap_or(Affine::IDENTITY);
            if let (Ok(image), Some(size)) = (image, KSize::from_wh(brush.image.width as f32, brush.image.height as f32)) {
                self.surface.push_transform(&transform(placed));
                if draw.composite.alpha < 1.0 || brush.sampler.alpha < 1.0 {
                    self.surface.push_opacity(NormalizedF32::new((draw.composite.alpha * brush.sampler.alpha).clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ONE));
                    self.surface.draw_image(image, size);
                    self.surface.pop();
                } else {
                    self.surface.draw_image(image, size);
                }
                self.surface.pop();
            }
            for _ in 0..pushed {
                self.surface.pop();
            }
            return;
        }
        let Some(color) = solid(&draw.brush) else { return };
        let (color, opacity) = paint(color, draw.composite.alpha);
        if opacity.get() <= 0.0 {
            return;
        }
        let Some(path) = path(&(self.base * draw.transform), &draw.shape) else { return };
        self.surface.set_stroke(None);
        self.surface.set_fill(Some(Fill { paint: color.into(), opacity, rule: rule(draw.fill_rule) }));
        self.surface.draw_path(&path);
    }

    fn stroke(&mut self, draw: StrokeRef<'_>) {
        let width = draw.stroke.width;
        if draw.composite.alpha <= 0.0 || !self.visible(draw.transform, bounds(&draw.shape).inflate(width, width)) {
            return;
        }
        let Some(color) = solid(&draw.brush) else { return };
        let full = self.base * draw.transform;
        let (color, opacity) = paint(color, draw.composite.alpha);
        if opacity.get() <= 0.0 {
            return;
        }
        let Some(path) = path(&full, &draw.shape) else { return };
        let line_cap = match draw.stroke.start_cap {
            kurbo::Cap::Butt => LineCap::Butt,
            kurbo::Cap::Round => LineCap::Round,
            kurbo::Cap::Square => LineCap::Square,
        };
        let line_join = match draw.stroke.join {
            kurbo::Join::Bevel => LineJoin::Bevel,
            kurbo::Join::Miter => LineJoin::Miter,
            kurbo::Join::Round => LineJoin::Round,
        };
        self.surface.set_fill(None);
        self.surface.set_stroke(Some(Stroke {
            paint: color.into(),
            width: (width * scale_of(&full)) as f32,
            miter_limit: draw.stroke.miter_limit as f32,
            line_cap,
            line_join,
            opacity,
            dash: None,
        }));
        self.surface.draw_path(&path);
        self.surface.set_stroke(None);
    }

    fn glyph_run(&mut self, draw: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = SceneGlyph>) {
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

    fn blurred_rounded_rect(&mut self, _draw: BlurredRoundedRect) {}
}
