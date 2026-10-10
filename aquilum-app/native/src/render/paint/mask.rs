//! Кэширование и отрисовка масок групп.

use std::collections::VecDeque;

use masonry::imaging::MaskMode;
use masonry::imaging::record::{Scene, replay_transformed};
use masonry::kurbo::Affine;
use vello_cpu::{Mask, Pixmap, RenderSettings};

use super::Painter;
use super::error::PaintError;

#[derive(Clone)]
pub struct CachedMask {
    pub scene: Scene,
    pub mode: MaskMode,
    pub transform: Affine,
    pub mask: Mask,
}

pub const MASK_CACHE_LIMIT: usize = 8;

pub fn get_or_render_mask(
    cache: &mut VecDeque<CachedMask>,
    scene: &Scene,
    mode: MaskMode,
    transform: Affine,
    width: u16,
    height: u16,
    settings: RenderSettings,
    tolerance: f64,
) -> Result<Option<Mask>, PaintError> {
    if let Some(entry) = cache.iter().find(|e| e.mode == mode && e.transform == transform && e.scene == *scene) {
        return Ok(Some(entry.mask.clone()));
    }

    let mut sub = Painter::new(width, height, settings);
    sub.tolerance = tolerance;
    replay_transformed(scene, &mut sub, transform);
    sub.finish()?;
    let mut pixmap = Pixmap::new(width, height);
    sub.ctx.render(&mut pixmap, &mut sub.resources);
    let mask = match mode {
        MaskMode::Alpha => Mask::new_alpha(&pixmap),
        MaskMode::Luminance => Mask::new_luminance(&pixmap),
    };
    if cache.len() == MASK_CACHE_LIMIT {
        cache.pop_front();
    }
    cache.push_back(CachedMask { scene: scene.clone(), mode, transform, mask: mask.clone() });
    Ok(Some(mask))
}
