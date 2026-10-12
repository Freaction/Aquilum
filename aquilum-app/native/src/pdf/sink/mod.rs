mod fonts;
mod geometry;
mod glyphs;
mod image;

use krilla::num::NormalizedF32;
use krilla::paint::{Fill, FillRule, LineCap, LineJoin, Stroke};
use krilla::surface::Surface;
use masonry::imaging::record::Glyph as SceneGlyph;
use masonry::imaging::{BlurredRoundedRect, ClipRef, FillRef, GeometryRef, GlyphRunRef, GroupRef, PaintSink, StrokeRef};
use masonry::kurbo::{self, Affine, Rect};
use masonry::peniko::BrushRef;

use geometry::{bounds, paint, path, rule, scale_of, solid};

pub use fonts::Fonts;

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

impl PdfSink<'_, '_> {
    fn clip_ref(&mut self, clip: &ClipRef<'_>) -> usize {
        match clip {
            ClipRef::Fill { transform, shape, fill_rule } => self.clip(*transform, shape, rule(*fill_rule)),
            ClipRef::Stroke { transform, shape, .. } => self.clip(*transform, shape, FillRule::NonZero),
        }
    }
}

impl PaintSink for PdfSink<'_, '_> {
    fn push_clip(&mut self, clip: ClipRef<'_>) {
        let pushed = self.clip_ref(&clip);
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
            pushed += self.clip_ref(clip);
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
            self.fill_image(&draw, brush);
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
        self.draw_glyph_run(draw, glyphs);
    }

    fn blurred_rounded_rect(&mut self, _draw: BlurredRoundedRect) {}
}
