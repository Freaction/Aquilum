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

pub fn settings(threads: u16) -> RenderSettings {
    let level = RenderSettings::default().level;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let level = level.as_sse4_2().map_or(level, vello_cpu::Level::Sse4_2);
    RenderSettings { num_threads: threads, level, ..Default::default() }
}

/// Исполняет команды сцены в `RenderContext` vello_cpu.
pub struct Painter {
    pub(super) ctx: RenderContext,
    pub(super) resources: Resources,
    pub(super) settings: RenderSettings,
    pub(super) tolerance: f64,
    pub(super) error: Option<PaintError>,
    pub(super) clip_depth: u32,
    pub(super) groups: Vec<bool>,
    pub(super) mask_cache: VecDeque<CachedMask>,
    pub(super) frame: Rect,
    pub(super) shift: Affine,
    pub(super) images: convert::Images,
    pub(super) culled: u32,
    pub(super) seconds: f64,
    pub(super) regions: Vec<Rect>,
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
            groups: Vec::new(),
            mask_cache: VecDeque::new(),
            frame: Rect::new(0.0, 0.0, f64::from(width), f64::from(height)),
            shift: Affine::IDENTITY,
            images: convert::Images::default(),
            culled: 0,
            seconds: 0.0,
            regions: Vec::new(),
        }
    }

    pub fn trim(&mut self) {
        self.ctx = RenderContext::new_with(1, 1, self.settings);
        self.resources = Resources::new();
        self.images = convert::Images::default();
        self.mask_cache = VecDeque::new();
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
        self.groups.clear();
        self.culled = 0;
    }

    pub(super) fn offscreen(&self, bounds: Rect) -> bool {
        self.culled > 0 || !self.filtered() && !self.frame.overlaps(bounds)
    }

    pub(super) fn cull(&mut self, clip: &masonry::imaging::ClipRef<'_>) -> bool {
        if self.culled > 0 {
            return self.cull_inside();
        }
        let bounds = match clip {
            masonry::imaging::ClipRef::Fill { transform, shape, .. } => transform.transform_rect_bbox(convert::shape_bounds(shape)),
            masonry::imaging::ClipRef::Stroke { transform, shape, stroke } => {
                let w = stroke.width.max(1.0) * stroke.miter_limit.max(1.0);
                transform.transform_rect_bbox(convert::shape_bounds(shape).inflate(w, w))
            }
        };
        if !self.filtered() && !self.frame.overlaps(bounds) {
            self.culled = 1;
            return true;
        }
        false
    }

    fn filtered(&self) -> bool {
        self.groups.contains(&true)
    }

    pub(super) fn cull_inside(&mut self) -> bool {
        self.culled += 1;
        true
    }

    pub(super) fn uncull(&mut self) -> bool {
        if self.culled == 0 {
            return false;
        }
        self.culled -= 1;
        true
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
        if !self.groups.is_empty() {
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

    pub(super) fn paint_live(&mut self, transform: Affine, shape: &masonry::imaging::GeometryRef<'_>, motion: super::live::Motion) {
        let Some(rounded) = super::live::shape(shape) else { return };
        let area = transform.transform_rect_bbox(rounded.rect()).intersect(self.frame).expand();
        if area.width() < 1.0 || area.height() < 1.0 {
            return;
        }
        let mut pixels = Vec::new();
        super::live::draw(transform, rounded, motion, area, self.seconds, &mut pixels);
        let metadata = vello_cpu::PixelMetadata::new(masonry::peniko::ImageAlphaType::AlphaPremultiplied, true);
        let pixmap = vello_cpu::Pixmap::from_parts(bytemuck::cast_slice::<u32, u8>(&pixels).to_vec(), area.width() as u16, area.height() as u16, metadata);
        let sampler = masonry::peniko::ImageSampler::default().with_quality(masonry::peniko::ImageQuality::Low);
        self.ctx.set_transform(self.shift * Affine::translate(area.origin().to_vec2()));
        self.ctx.set_paint_transform(Affine::IDENTITY);
        self.ctx.set_blend_mode(masonry::peniko::BlendMode::default());
        self.ctx.set_paint(vello_cpu::Image { image: vello_cpu::ImageSource::Pixmap(std::sync::Arc::new(pixmap)), sampler });
        self.ctx.fill_rect(&Rect::new(0.0, 0.0, area.width(), area.height()));
    }

    pub(super) fn draw_glyph_run(&mut self, run: GlyphRunRef<'_>, glyphs: &mut dyn Iterator<Item = SceneGlyph>) {
        let Some(first) = glyphs.next() else { return };
        if self.offscreen(convert::line_bounds(first.y, run.font_size, run.glyph_transform, run.transform)) {
            return;
        }
        let paint = convert::brush_to_paint(run.brush, run.composite, &mut self.images);
        self.set_transform(run.transform);
        self.ctx.set_paint_transform(run.brush_transform.unwrap_or(Affine::IDENTITY));
        self.ctx.set_paint(paint);
        self.ctx.set_blend_mode(run.composite.blend);
        let [a, b, c, d, _, _] = run.transform.as_coeffs();
        let reach = f64::from(run.font_size) * 1.5 * a.abs().max(b.abs()).max(c.abs()).max(d.abs());
        let area = self.frame.inflate(reach, reach);
        let filtered = self.filtered();
        let glyphs: Vec<Glyph> = std::iter::once(first)
            .chain(glyphs)
            .filter(|g| filtered || area.contains(run.transform * masonry::kurbo::Point::new(f64::from(g.x), f64::from(g.y))))
            .map(|g| Glyph { id: g.id, x: g.x, y: g.y })
            .collect();
        if glyphs.is_empty() {
            return;
        }
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
