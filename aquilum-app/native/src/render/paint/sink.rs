//! Реализация трейта `PaintSink` для `Painter`.

use masonry::imaging::{
    BlurredRoundedRect, ClipRef, FillRef, GeometryRef, GlyphRunRef, GroupRef, PaintSink, StrokeRef,
};
use masonry::imaging::record::Glyph as SceneGlyph;
use masonry::kurbo::{Affine, Shape as _};
use masonry::peniko::{BlendMode, BrushRef};

use super::Painter;
use super::convert::{self, TOLERANCE};
use super::error::PaintError;
use super::mask;

impl PaintSink for Painter {
    fn push_clip(&mut self, clip: ClipRef<'_>) {
        if self.error.is_some() || self.cull(&clip) {
            return;
        }
        if let ClipRef::Fill { transform, shape: GeometryRef::Rect(rect), fill_rule } = clip {
            let window = transform.transform_rect_bbox(rect);
            let [_, skew_y, skew_x, _, _, _] = transform.as_coeffs();
            let region = (skew_x == 0.0 && skew_y == 0.0 && self.regions.iter().any(|r| crate::render::scroll::close(*r, window))).then(|| crate::render::scroll::snap(window));
            self.ctx.set_fill_rule(fill_rule);
            match region {
                Some(snapped) => {
                    self.set_transform(Affine::IDENTITY);
                    self.ctx.push_clip_rect(&snapped);
                }
                None => {
                    self.set_transform(transform);
                    self.ctx.push_clip_rect(&rect);
                }
            }
        } else {
            let (transform, path, fill_rule) = convert::clip_to_path(clip);
            self.set_transform(transform);
            self.ctx.set_fill_rule(fill_rule);
            self.ctx.push_clip_path(&path);
        }
        self.clip_depth += 1;
    }

    fn pop_clip(&mut self) {
        if self.error.is_some() || self.uncull() {
            return;
        }
        if self.clip_depth == 0 {
            self.set_error_once(PaintError::Unbalanced("pop_clip без push_clip"));
            return;
        }
        self.ctx.pop_clip();
        self.clip_depth -= 1;
    }

    fn push_group(&mut self, group: GroupRef<'_>) {
        if self.error.is_some() || (self.culled > 0 || group.composite.alpha <= 0.0) && self.cull_inside() || group.clip.as_ref().is_some_and(|clip| self.cull(clip)) {
            return;
        }
        let (clip_path, clip_transform) = match group.clip {
            None => (None, Affine::IDENTITY),
            Some(clip) => {
                let (transform, path, fill_rule) = convert::clip_to_path(clip);
                self.ctx.set_fill_rule(fill_rule);
                (Some(path), transform)
            }
        };
        self.set_transform(clip_transform);
        let blend: Option<BlendMode> = Some(group.composite.blend);
        let opacity = Some(group.composite.alpha);
        let (width, height) = (self.width(), self.height());
        let mask = match group.mask {
            Some(mask) => {
                match mask::get_or_render_mask(
                    &mut self.mask_cache,
                    mask.mask.scene,
                    mask.mask.mode,
                    self.shift * mask.transform,
                    width,
                    height,
                ) {
                    Ok(m) => Some(m),
                    Err(err) => {
                        self.set_error_once(err);
                        None
                    }
                }
            }
            None => None,
        };
        let filter = convert::filters_to_vello(group.filters);
        self.ctx.push_layer(clip_path.as_ref(), blend, opacity, mask, filter);
        self.groups.push(!group.filters.is_empty());
    }

    fn pop_group(&mut self) {
        if self.uncull() {
            return;
        }
        if self.error.is_some() {
            return;
        }
        if self.groups.pop().is_none() {
            self.set_error_once(PaintError::Unbalanced("pop_group без push_group"));
            return;
        }
        self.ctx.pop_layer();
    }

    fn fill(&mut self, draw: FillRef<'_>) {
        if let Some(id) = self.after {
            if matches!(&draw.brush, BrushRef::Image(image) if image.image.data.id() == id) {
                self.after = None;
            }
            return;
        }
        if self.error.is_some() || convert::invisible(&draw.brush, draw.composite) || self.offscreen(draw.transform.transform_rect_bbox(convert::shape_bounds(&draw.shape))) {
            return;
        }
        if let BrushRef::Image(image) = &draw.brush
            && let Some(motion) = crate::render::live::motion(image.image.data.id())
        {
            return self.paint_live(draw.transform, &draw.shape, motion);
        }
        let sprite = match &draw.brush {
            BrushRef::Image(image) => crate::render::sprite::covered(image.image.data.id()).map(|covered| (covered, image.image.data.id())),
            _ => None,
        };
        let brush = match sprite {
            Some((false, _)) => return,
            Some((true, id)) => BrushRef::Solid(crate::render::sprite::color(id).unwrap_or(masonry::peniko::Color::TRANSPARENT)),
            None => draw.brush,
        };
        let paint = convert::brush_to_paint(brush, draw.composite, &mut self.images);
        self.ctx.set_fill_rule(draw.fill_rule);
        place(self, draw.transform, draw.brush_transform, draw.composite.blend, paint);
        draw_shape(&mut self.ctx, draw.shape, false);
    }

    fn stroke(&mut self, draw: StrokeRef<'_>) {
        if self.error.is_some() || self.after.is_some() || convert::invisible(&draw.brush, draw.composite) || self.offscreen(convert::stroke_bounds(&draw.shape, draw.stroke, draw.transform)) {
            return;
        }
        let paint = convert::brush_to_paint(draw.brush, draw.composite, &mut self.images);
        self.ctx.set_stroke(draw.stroke.clone());
        place(self, draw.transform, draw.brush_transform, draw.composite.blend, paint);
        draw_shape(&mut self.ctx, draw.shape, true);
    }

    fn glyph_run(&mut self, draw: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = SceneGlyph>) {
        if self.error.is_some() || self.after.is_some() {
            return;
        }
        self.draw_glyph_run(draw, glyphs);
    }

    fn blurred_rounded_rect(&mut self, draw: BlurredRoundedRect) {
        if self.error.is_some() || self.after.is_some() {
            return;
        }
        self.set_transform(draw.transform);
        self.ctx.set_paint(draw.color.multiply_alpha(draw.composite.alpha));
        self.ctx.set_blend_mode(draw.composite.blend);
        // Обычная тень; инвертированная (внутренняя) в imaging пока не выражается.
        self.ctx.fill_blurred_rounded_rect(&draw.rect, draw.radius as f32, draw.std_dev as f32, false);
    }
}

fn place(painter: &mut Painter, transform: Affine, brush_transform: Option<Affine>, blend: BlendMode, paint: vello_cpu::PaintType) {
    painter.set_transform(transform);
    painter.ctx.set_paint_transform(brush_transform.unwrap_or(Affine::IDENTITY));
    painter.ctx.set_blend_mode(blend);
    painter.ctx.set_paint(paint);
}

fn draw_shape(ctx: &mut vello_cpu::RenderContext, shape: GeometryRef<'_>, stroke: bool) {
    match shape {
        GeometryRef::Rect(rect) => {
            if stroke { ctx.stroke_rect(&rect) } else { ctx.fill_rect(&rect) }
        }
        GeometryRef::RoundedRect(rounded) => {
            let path = rounded.to_path(TOLERANCE);
            if stroke { ctx.stroke_path(&path) } else { ctx.fill_path(&path) }
        }
        GeometryRef::Path(path) => {
            if stroke { ctx.stroke_path(path) } else { ctx.fill_path(path) }
        }
        GeometryRef::OwnedPath(path) => {
            if stroke { ctx.stroke_path(&path) } else { ctx.fill_path(&path) }
        }
    }
}
