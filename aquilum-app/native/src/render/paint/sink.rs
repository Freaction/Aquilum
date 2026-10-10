//! Реализация трейта `PaintSink` для `Painter`.

use masonry::imaging::{
    BlurredRoundedRect, ClipRef, FillRef, GeometryRef, GlyphRunRef, GroupRef, PaintSink, StrokeRef,
};
use masonry::imaging::record::Glyph as SceneGlyph;
use masonry::kurbo::{Affine, Shape as _};
use masonry::peniko::BlendMode;

use super::Painter;
use super::convert;
use super::error::PaintError;
use super::mask;

impl PaintSink for Painter {
    fn push_clip(&mut self, clip: ClipRef<'_>) {
        if self.error.is_some() {
            return;
        }
        let (transform, path, fill_rule) = convert::clip_to_path(clip, self.tolerance);
        self.set_transform(transform);
        self.ctx.set_fill_rule(fill_rule);
        self.ctx.push_clip_path(&path);
        self.clip_depth += 1;
    }

    fn pop_clip(&mut self) {
        if self.error.is_some() {
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
        if self.error.is_some() {
            return;
        }
        let (clip_path, clip_transform) = match group.clip {
            None => (None, Affine::IDENTITY),
            Some(clip) => {
                let (transform, path, fill_rule) = convert::clip_to_path(clip, self.tolerance);
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
                    self.settings,
                    self.tolerance,
                ) {
                    Ok(m) => m,
                    Err(err) => {
                        self.set_error_once(err);
                        None
                    }
                }
            }
            None => None,
        };
        let filter = if !group.filters.is_empty() {
            match convert::filters_to_vello(group.filters) {
                Some(f) => Some(f),
                None => {
                    self.set_error_once(PaintError::EmptyFilter);
                    None
                }
            }
        } else {
            None
        };
        self.ctx.push_layer(clip_path.as_ref(), blend, opacity, mask, filter);
        self.group_depth += 1;
    }

    fn pop_group(&mut self) {
        if self.error.is_some() {
            return;
        }
        if self.group_depth == 0 {
            self.set_error_once(PaintError::Unbalanced("pop_group без push_group"));
            return;
        }
        self.ctx.pop_layer();
        self.group_depth -= 1;
    }

    fn fill(&mut self, draw: FillRef<'_>) {
        if self.error.is_some() || self.offscreen(draw.transform.transform_rect_bbox(convert::shape_bounds(&draw.shape))) {
            return;
        }
        let paint = convert::brush_to_paint(draw.brush, draw.composite, &mut self.images);
        self.set_transform(draw.transform);
        self.ctx.set_fill_rule(draw.fill_rule);
        self.ctx.set_paint_transform(draw.brush_transform.unwrap_or(Affine::IDENTITY));
        self.ctx.set_blend_mode(draw.composite.blend);
        self.ctx.set_paint(paint);
        match draw.shape {
            GeometryRef::Rect(r) => self.ctx.fill_rect(&r),
            GeometryRef::RoundedRect(rr) => self.ctx.fill_path(&rr.to_path(self.tolerance)),
            GeometryRef::Path(p) => self.ctx.fill_path(p),
            GeometryRef::OwnedPath(p) => self.ctx.fill_path(&p),
        }
    }

    fn stroke(&mut self, draw: StrokeRef<'_>) {
        let w = draw.stroke.width.max(1.0) * draw.stroke.miter_limit.max(1.0);
        if self.error.is_some() || self.offscreen(draw.transform.transform_rect_bbox(convert::shape_bounds(&draw.shape).inflate(w, w))) {
            return;
        }
        let paint = convert::brush_to_paint(draw.brush, draw.composite, &mut self.images);
        self.set_transform(draw.transform);
        self.ctx.set_stroke(draw.stroke.clone());
        self.ctx.set_paint_transform(draw.brush_transform.unwrap_or(Affine::IDENTITY));
        self.ctx.set_blend_mode(draw.composite.blend);
        self.ctx.set_paint(paint);
        match draw.shape {
            GeometryRef::Rect(r) => self.ctx.stroke_rect(&r),
            GeometryRef::RoundedRect(rr) => self.ctx.stroke_path(&rr.to_path(self.tolerance)),
            GeometryRef::Path(p) => self.ctx.stroke_path(p),
            GeometryRef::OwnedPath(p) => self.ctx.stroke_path(&p),
        }
    }

    fn glyph_run(&mut self, draw: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = SceneGlyph>) {
        if self.error.is_some() {
            return;
        }
        self.draw_glyph_run(draw, glyphs);
    }

    fn blurred_rounded_rect(&mut self, draw: BlurredRoundedRect) {
        if self.error.is_some() {
            return;
        }
        self.set_transform(draw.transform);
        self.ctx.set_paint(draw.color.multiply_alpha(draw.composite.alpha));
        self.ctx.set_blend_mode(draw.composite.blend);
        // Обычная тень; инвертированная (внутренняя) в imaging пока не выражается.
        self.ctx.fill_blurred_rounded_rect(&draw.rect, draw.radius as f32, draw.std_dev as f32, false);
    }
}
