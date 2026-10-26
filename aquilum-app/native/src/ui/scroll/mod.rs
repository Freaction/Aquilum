mod area;
pub mod bar;
mod glide;
mod input;
mod pan;
mod widget;

#[cfg(test)]
mod tests;

pub use area::ScrollArea;
pub use pan::{HScroll, Pan};

pub const PAGE: f64 = 0.875;

pub fn pixels(delta: &masonry::core::ScrollDelta, scale: f64, page: f64) -> masonry::kurbo::Vec2 {
    use masonry::core::ScrollDelta;
    use masonry::kurbo::Vec2;
    match *delta {
        ScrollDelta::LineDelta(x, y) => Vec2::new(f64::from(x), f64::from(y)) * crate::ui::keymap::gesture::WHEEL_LINE,
        ScrollDelta::PixelDelta(p) => Vec2::new(p.x, p.y) / scale,
        ScrollDelta::PageDelta(x, y) => Vec2::new(f64::from(x), f64::from(y)) * page,
    }
}

pub fn sideways(d: masonry::kurbo::Vec2, shift: bool) -> f64 {
    if shift && d.x == 0.0 {
        d.y
    } else if d.x.abs() > d.y.abs() {
        d.x
    } else {
        0.0
    }
}

#[derive(Debug)]
pub struct Scrolled(pub f64);
