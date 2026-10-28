mod blit;
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

#[cfg(target_os = "android")]
pub const PIXEL_FORMAT: vello_cpu::PixelFormat = vello_cpu::PixelFormat::Rgba8;
#[cfg(not(target_os = "android"))]
pub const PIXEL_FORMAT: vello_cpu::PixelFormat = vello_cpu::PixelFormat::Bgra8;

pub fn in_canvas_order(rgba: u32) -> u32 {
    match PIXEL_FORMAT {
        vello_cpu::PixelFormat::Rgba8 => rgba,
        _ => (rgba & 0xff00_ff00) | (rgba & 0xff) << 16 | (rgba >> 16) & 0xff,
    }
}

thread_local! {
    static TRIM: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn trim_soon() {
    TRIM.set(true);
}

pub fn take_trim() -> bool {
    TRIM.replace(false)
}
