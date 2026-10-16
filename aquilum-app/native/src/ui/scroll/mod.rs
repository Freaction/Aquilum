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

pub const LINE: f64 = 48.0;
pub const PAGE: f64 = 0.875;

#[derive(Debug)]
pub struct Scrolled(pub f64);
