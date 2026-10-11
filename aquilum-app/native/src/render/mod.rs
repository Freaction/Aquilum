mod damage;
pub mod fade;
mod layer;
pub mod live;
mod mark;
mod offscreen;
#[cfg_attr(feature = "installed", path = "overlay_off.rs")]
mod overlay;
mod paint;
mod pixels;
pub mod scroll;
mod renderer;
pub mod sprite;
mod stamp;
mod surface;

#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;

pub use offscreen::offscreen;
pub use pixels::{brush, from_rgba};
pub use renderer::Renderer;
pub use surface::Surfaces;
