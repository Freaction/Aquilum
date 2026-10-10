//! Исполнитель команд `imaging` (сцены Masonry) на vello_cpu 0.3.

pub(super) mod convert;
mod error;
mod mask;
mod sink;

use std::collections::VecDeque;

use masonry::imaging::GlyphRunRef;
use masonry::imaging::record::Glyph as SceneGlyph;
use masonry::kurbo::{Affine, Rect};
use masonry::peniko::Style;
use vello_cpu::{Glyph, RenderContext, RenderSettings, Resources};

pub use error::PaintError;
use mask::CachedMask;

/// Исполняет команды сцены в `RenderContext` vello_cpu.
pub struct Painter {
    pub(super) ctx: RenderContext,
    pub(super) resources: Resources,
    pub(super) settings: RenderSettings,
    pub(super) tolerance: f64,
    pub(super) error: Option<PaintError>,
    pub(super) clip_depth: u32,
    pub(super) group_depth: u32,
    pub(super) mask_cache: VecDeque<CachedMask>,
    pub(super) frame: Rect,
    pub(super) shift: Affine,
    pub(super) images: convert::Images,
}

impl Painter {
    pub fn new(width: u16, height: u16, settings: RenderSettings) -> Self {
        Painter {
            ctx: RenderContext::new_with(width, height, settings),
            resources: Resources::new(),
            settings,
            tolerance: 0.1,
            error: None,
            clip_depth: 0,
            group_depth: 0,
            mask_cache: VecDeque::new(),
            frame: Rect::new(0.0, 0.0, f64::from(width), f64::from(height)),
            shift: Affine::IDENTITY,
            images: convert::Images::default(),
        }
    }

    pub fn width(&self) -> u16 {
        self.ctx.width()
    }

    pub fn height(&self) -> u16 {
        self.ctx.height()
    }

    pub fn begin(&mut self, x: u16, y: u16, width: u16, height: u16) {
        if self.ctx.width() != width || self.ctx.height() != height {
            self.ctx.reset_and_resize(width, height);
            self.mask_cache.clear();
        } else {
            self.ctx.reset();
        }
        self.images.next_frame();
        self.frame = Rect::new(f64::from(x), f64::from(y), f64::from(x) + f64::from(width), f64::from(y) + f64::from(height));
        self.shift = Affine::translate((-f64::from(x), -f64::from(y)));
        self.error = None;
        self.clip_depth = 0;
        self.group_depth = 0;
    }

    pub(super) fn offscreen(&self, bounds: Rect) -> bool {
        self.group_depth == 0 && !self.frame.overlaps(bounds)
    }

    pub(super) fn set_transform(&mut self, transform: Affine) {
        self.ctx.set_transform(self.shift * transform);
    }

    /// Проверяет, что кадр собран без ошибок и стеки сбалансированы.
    pub fn finish(&mut self) -> Result<(), PaintError> {
        if let Some(err) = self.error.take() {
            return Err(err);
        }
        if self.clip_depth != 0 {
            return Err(PaintError::Unbalanced("клип не закрыт"));
        }
        if self.group_depth != 0 {
            return Err(PaintError::Unbalanced("группа не закрыта"));
        }
        self.ctx.flush();
        Ok(())
    }

    pub(super) fn set_error_once(&mut self, err: PaintError) {
        if self.error.is_none() {
            self.error = Some(err);
        }
    }

    pub(super) fn draw_glyph_run(&mut self, run: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = SceneGlyph>) {
        let paint = convert::brush_to_paint(run.brush, run.composite, &mut self.images);
        self.set_transform(run.transform);
        self.ctx.set_paint_transform(run.brush_transform.unwrap_or(Affine::IDENTITY));
        self.ctx.set_paint(paint);
        self.ctx.set_blend_mode(run.composite.blend);

        let Some(first) = glyphs.next() else { return };
        if self.offscreen(convert::line_bounds(first.y, run.font_size, run.glyph_transform, run.transform)) {
            return;
        }
        let glyphs: Vec<Glyph> = std::iter::once(first).chain(glyphs).map(|g| Glyph { id: g.id, x: g.x, y: g.y }).collect();
        match run.style {
            Style::Fill(fill_rule) => self.ctx.set_fill_rule(*fill_rule),
            Style::Stroke(stroke) => self.ctx.set_stroke(stroke.clone()),
        }
        let builder = self
            .ctx
            .glyph_run(&mut self.resources, run.font)
            .font_size(run.font_size)
            .atlas_cache(true)
            .hint(run.hint)
            .normalized_coords(run.normalized_coords);
        let builder = match run.glyph_transform {
            Some(transform) => builder.glyph_transform(transform),
            None => builder,
        };
        // Ошибка отдельного прогона (нет контура у глифа и т. п.) не должна ронять кадр.
        let _ = match run.style {
            Style::Fill(_) => builder.fill_glyphs(glyphs.into_iter()),
            Style::Stroke(_) => builder.stroke_glyphs(glyphs.into_iter()),
        };
    }
}
