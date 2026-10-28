//! Живые слои: анимация, которую рендер дорисовывает сам, без пересборки сцены Masonry.
//! Виджет рисует в сцене неизменную метку — кисть-картинку из этого реестра. Обычный кадр
//! рисует на её месте узор на текущий момент, кадр анимации склеивает кэши «под» и «над»
//! меткой с узором (`Renderer::live`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use masonry::peniko::ImageBrush;

#[derive(Clone, Copy, PartialEq)]
pub struct Motion {
    pub pattern: &'static str,
    pub viewport: f64,
}

thread_local! {
    static MOTIONS: RefCell<HashMap<u64, Motion>> = RefCell::new(HashMap::new());
    static EPOCH: Instant = Instant::now();
    #[cfg(test)]
    static FROZEN: std::cell::Cell<Option<f64>> = const { std::cell::Cell::new(None) };
}

pub fn marker(motion: Motion) -> ImageBrush {
    let brush = super::pixels::marker();
    MOTIONS.with_borrow_mut(|m| m.insert(brush.image.data.id(), motion));
    brush
}

pub fn update(marker: &ImageBrush, motion: Motion) {
    MOTIONS.with_borrow_mut(|m| m.insert(marker.image.data.id(), motion));
}

pub fn forget(marker: &ImageBrush) {
    MOTIONS.with_borrow_mut(|m| m.remove(&marker.image.data.id()));
}

pub(super) fn shape(shape: &masonry::imaging::GeometryRef<'_>) -> Option<masonry::kurbo::RoundedRect> {
    match shape {
        masonry::imaging::GeometryRef::RoundedRect(rounded) => Some(*rounded),
        masonry::imaging::GeometryRef::Rect(rect) => Some(rect.to_rounded_rect(0.0)),
        _ => None,
    }
}

pub(super) fn motion(id: u64) -> Option<Motion> {
    MOTIONS.with_borrow(|m| m.get(&id).copied())
}

pub fn seconds() -> f64 {
    #[cfg(test)]
    if let Some(t) = FROZEN.get() {
        return t;
    }
    EPOCH.with(|epoch| epoch.elapsed().as_secs_f64())
}

#[cfg(test)]
pub fn freeze(seconds: Option<f64>) {
    FROZEN.set(seconds);
}

pub(super) fn draw(transform: masonry::kurbo::Affine, shape: masonry::kurbo::RoundedRect, motion: Motion, area: masonry::kurbo::Rect, seconds: f64, pixels: crate::cover::MotionPixels<'_>) {
    let cover = transform.transform_rect_bbox(shape.rect());
    let radii = shape.radii();
    let mut target = crate::cover::MotionTarget {
        pixels,
        width: area.width() as usize,
        height: area.height() as usize,
        origin: (area.x0 - cover.x0, area.y0 - cover.y0),
        scale: transform.as_coeffs()[0],
        size: (shape.width(), shape.height()),
        radius: radii.top_left.max(radii.top_right).max(radii.bottom_left).max(radii.bottom_right),
    };
    crate::cover::render_motion(motion.pattern, &mut target, seconds, motion.viewport);
}

pub(super) fn frame(motion: Motion, seconds: f64) -> i64 {
    (seconds * crate::cover::motion_fps(motion.pattern)).floor() as i64
}
