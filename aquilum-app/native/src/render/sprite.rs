use std::cell::RefCell;
use std::collections::HashMap;

use masonry::kurbo::{Rect, RoundedRect};
use masonry::peniko::{Color, ImageBrush};

use super::stamp::{Stamps, blend, covering};
use crate::output::{Area, Canvas};

#[derive(Clone, Copy)]
struct Entry {
    color: Color,
    covered: bool,
}

thread_local! {
    static SPRITES: RefCell<HashMap<u64, Entry>> = RefCell::new(HashMap::new());
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Sprite {
    pub id: u64,
    pub shape: RoundedRect,
    pub clip: Rect,
    pub color: Color,
    pub covered: bool,
}

impl Sprite {
    pub fn bounds(&self) -> Rect {
        self.shape.rect().intersect(self.clip)
    }
}

pub fn marker() -> ImageBrush {
    super::pixels::marker()
}

pub fn paint(marker: &ImageBrush, color: Color) {
    SPRITES.with_borrow_mut(|s| s.entry(marker.image.data.id()).or_insert(Entry { color, covered: false }).color = color);
}

pub fn forget(marker: &ImageBrush) {
    SPRITES.with_borrow_mut(|s| s.remove(&marker.image.data.id()));
}

pub(super) fn color(id: u64) -> Option<Color> {
    SPRITES.with_borrow(|s| s.get(&id).map(|e| e.color))
}

pub(super) fn covered(id: u64) -> Option<bool> {
    SPRITES.with_borrow(|s| s.get(&id).map(|e| e.covered))
}

pub(super) fn cover(sprites: &[Sprite]) {
    SPRITES.with_borrow_mut(|s| {
        for sprite in sprites {
            if let Some(entry) = s.get_mut(&sprite.id) {
                entry.covered = sprite.covered;
            }
        }
    });
}

pub(super) fn changes(before: &[Sprite], after: &[Sprite]) -> Option<Area> {
    let gone = before.iter().filter(|s| !s.covered && !after.contains(s));
    let new = after.iter().filter(|s| !s.covered && !before.contains(s));
    gone.chain(new).map(|s| covering(s.bounds())).filter(|a| a.w > 0 && a.h > 0).reduce(Area::union)
}

pub(super) fn stamp(sprites: &[Sprite], canvas: &mut dyn Canvas, stamps: &mut Stamps) {
    canvas.with_target(&mut |mut target| {
        for sprite in sprites.iter().filter(|s| !s.covered) {
            let area = covering(sprite.bounds());
            if area.w == 0 || area.h == 0 {
                continue;
            }
            let color = sprite.color.premultiply().components;
            let (shape, clip) = (sprite.shape, sprite.clip);
            stamps.apply(&mut target, area, |x, y, pixel| {
                let coverage = coverage(shape, clip, f64::from(x) + 0.5, f64::from(y) + 0.5);
                if coverage > 0.0 {
                    *pixel = blend(*pixel, color, coverage);
                }
            });
        }
    });
}

fn coverage(shape: RoundedRect, clip: Rect, x: f64, y: f64) -> f32 {
    let rect = shape.rect();
    let radius = shape.radii().top_left.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let center = rect.center();
    let qx = (x - center.x).abs() - (rect.width() / 2.0 - radius);
    let qy = (y - center.y).abs() - (rect.height() / 2.0 - radius);
    let distance = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius;
    let edge = |a: f64, b: f64| (b - a + 0.5).clamp(0.0, 1.0);
    let clipped = edge(clip.x0, x).min(edge(x, clip.x1)).min(edge(clip.y0, y)).min(edge(y, clip.y1));
    ((0.5 - distance).clamp(0.0, 1.0) * clipped) as f32
}
