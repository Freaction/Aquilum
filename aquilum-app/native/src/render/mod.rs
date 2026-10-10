//! Отрисовка кадра: сцены Masonry → vello_cpu 0.3 → буфер окна softbuffer.
//!
//! Каждый кадр рисуется целиком в буфер размером с окно и целиком отдаётся окну.

mod damage;
mod offscreen;
mod paint;
mod renderer;
mod surface;

#[cfg(test)]
mod tests;

pub use offscreen::offscreen;
#[allow(unused_imports)]
pub use paint::{PaintError, Painter};
pub use renderer::Renderer;
pub use surface::Surfaces;
