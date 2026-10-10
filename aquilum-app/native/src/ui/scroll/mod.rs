//! Вертикальная прокрутка с полосой по токенам — как `.q-scroll-area` и `::-webkit-scrollbar`
//! из `src/styles/base.css`: полоса 16 px справа занимает место, только когда содержимое не
//! помещается; ползунок со скруглением, отступом 3 px и минимальной высотой 24, при наведении
//! темнее. У `Portal` из Masonry цвета полос зашиты в код, поэтому свой виджет.

mod area;
pub mod bar;
mod widget;

#[cfg(test)]
mod tests;

pub use area::ScrollArea;

/// Прокрутка на одну «строку» колеса, логические пиксели.
pub const LINE: f64 = 48.0;

pub const GLIDE_SECONDS: f64 = 0.16;

#[derive(Debug)]
pub struct Scrolled(pub f64);
