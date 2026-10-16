mod area;
pub mod bar;
mod glide;
mod input;
mod widget;

#[cfg(test)]
mod tests;

pub use area::ScrollArea;

pub const LINE: f64 = 48.0;
pub const PAGE: f64 = 0.875;

#[derive(Debug)]
pub struct Scrolled(pub f64);
