pub(super) mod convert;
mod cull;
mod error;
mod mask;
mod runs;
mod sink;

use masonry::kurbo::{Affine, Rect};
use vello_cpu::{RenderContext, RenderSettings, Resources};

pub use error::PaintError;
use mask::{MASK_CACHE_LIMIT, Masks};

pub fn settings(threads: u16) -> RenderSettings {
    let level = RenderSettings::default().level;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let level = level.as_sse4_2().map_or(level, vello_cpu::Level::Sse4_2);
    RenderSettings { num_threads: threads, level }
}

const TOLERANCE: f64 = 0.1;

pub struct Painter {
    pub(super) ctx: RenderContext,
    pub(super) resources: Resources,
    pub(super) settings: RenderSettings,
    pub(super) error: Option<PaintError>,
    pub(super) clip_depth: u32,
    pub(super) groups: Vec<bool>,
    pub(super) mask_cache: Masks,
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
            error: None,
            clip_depth: 0,
            groups: Vec::new(),
            mask_cache: Masks::new(MASK_CACHE_LIMIT, usize::MAX),
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
        self.mask_cache.clear();
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

    pub(super) fn set_transform(&mut self, transform: Affine) {
        self.ctx.set_transform(self.shift * transform);
    }

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
}
