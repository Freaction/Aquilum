use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use masonry::kurbo::{Rect, RoundedRect, Vec2};
use masonry::peniko::{Color, ImageBrush};

use super::in_canvas_order;
use super::stamp::{Stamps, covering};
use crate::cover::over;
use crate::output::{Area, Canvas};

struct Entry {
    color: Color,
    covered: bool,
    baked: Option<(Key, Rc<Vec<u32>>)>,
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
    SPRITES.with_borrow_mut(|s| s.entry(marker.image.data.id()).or_insert(Entry { color, covered: false, baked: None }).color = color);
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
            let pixels = baked(sprite, area);
            let width = area.w as usize;
            stamps.apply(&mut target, area, |x, y, pixel| {
                *pixel = over(pixels[(y - area.y) as usize * width + (x - area.x) as usize], *pixel);
            });
        }
    });
}

#[derive(PartialEq)]
struct Key {
    shape: RoundedRect,
    clip: Rect,
    color: Color,
    size: (u32, u32),
}

fn baked(sprite: &Sprite, area: Area) -> Rc<Vec<u32>> {
    let origin = Vec2::new(f64::from(area.x), f64::from(area.y));
    let (w, h) = (f64::from(area.w), f64::from(area.h));
    let key = Key { shape: sprite.shape - origin, clip: (sprite.clip - origin).intersect(Rect::new(-1.0, -1.0, w + 1.0, h + 1.0)), color: sprite.color, size: (area.w, area.h) };
    SPRITES.with_borrow_mut(|s| {
        let entry = s.get_mut(&sprite.id)?;
        if let Some((kept, pixels)) = &entry.baked
            && *kept == key
        {
            return Some(Rc::clone(pixels));
        }
        let color = key.color.premultiply().components;
        let pixels: Rc<Vec<u32>> = Rc::new(
            (0..area.h)
                .flat_map(|y| (0..area.w).map(move |x| (x, y)))
                .map(|(x, y)| {
                    let c = coverage(key.shape, key.clip, f64::from(x) + 0.5, f64::from(y) + 0.5);
                    let channel = |v: f32| (v * c * 255.0).round() as u8;
                    in_canvas_order(u32::from_ne_bytes([channel(color[0]), channel(color[1]), channel(color[2]), channel(color[3])]))
                })
                .collect(),
        );
        entry.baked = Some((key, Rc::clone(&pixels)));
        Some(pixels)
    })
    .unwrap_or_default()
}

fn coverage(shape: RoundedRect, clip: Rect, x: f64, y: f64) -> f32 {
    let rect = shape.rect();
    let radius = shape.radii().top_left.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let center = rect.center();
    let qx = (x - center.x).abs() - (rect.width() / 2.0 - radius);
    let qy = (y - center.y).abs() - (rect.height() / 2.0 - radius);
    let (ox, oy) = (qx.max(0.0), qy.max(0.0));
    let distance = (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - radius;
    let edge = |a: f64, b: f64| (b - a + 0.5).clamp(0.0, 1.0);
    let clipped = edge(clip.x0, x).min(edge(x, clip.x1)).min(edge(clip.y0, y)).min(edge(y, clip.y1));
    ((0.5 - distance).clamp(0.0, 1.0) * clipped) as f32
}
