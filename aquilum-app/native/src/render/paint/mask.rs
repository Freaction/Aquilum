//! Кэширование и отрисовка масок групп.

use aquilum_core::recent::Recent;
use masonry::imaging::MaskMode;
use masonry::imaging::record::{Scene, replay_transformed};
use masonry::kurbo::Affine;
use vello_cpu::{Mask, Pixmap, RenderSettings};

use super::Painter;
use super::error::PaintError;

pub type Masks = Recent<(Scene, MaskMode, Affine), Mask>;

pub const MASK_CACHE_LIMIT: usize = 8;

pub fn get_or_render_mask(
    cache: &mut Masks,
    scene: &Scene,
    mode: MaskMode,
    transform: Affine,
    width: u16,
    height: u16,
    settings: RenderSettings,
) -> Result<Mask, PaintError> {
    if let Some(mask) = cache.find(|(s, m, t)| *m == mode && *t == transform && s == scene) {
        return Ok(mask.clone());
    }

    let mut sub = Painter::new(width, height, settings);
    replay_transformed(scene, &mut sub, transform);
    sub.finish()?;
    let mut pixmap = Pixmap::new(width, height);
    sub.ctx.render(&mut pixmap, &mut sub.resources);
    let mask = match mode {
        MaskMode::Alpha => Mask::new_alpha(&pixmap),
        MaskMode::Luminance => Mask::new_luminance(&pixmap),
    };
    cache.put((scene.clone(), mode, transform), mask.clone(), 0);
    Ok(mask)
}
