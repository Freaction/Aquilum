//! Кэширование и отрисовка масок групп.

use aquilum_core::recent::Recent;
use masonry::imaging::MaskMode;
use masonry::imaging::record::{Scene, replay_transformed};
use masonry::kurbo::Affine;
use vello_cpu::Mask;

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
) -> Result<Mask, PaintError> {
    if let Some(mask) = cache.find(|(s, m, t)| *m == mode && *t == transform && s == scene) {
        return Ok(mask.clone());
    }

    let mut sub = Painter::new(width, height, super::render_settings(0));
    replay_transformed(scene, &mut sub, transform);
    sub.finish()?;
    let pixmap = sub.pixmap();
    let mask = match mode {
        MaskMode::Alpha => Mask::new_alpha(&pixmap),
        MaskMode::Luminance => Mask::new_luminance(&pixmap),
    };
    cache.put((scene.clone(), mode, transform), mask.clone(), 0);
    Ok(mask)
}
