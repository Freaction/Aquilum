mod blit;
mod damage;
pub mod fade;
mod layer;
pub mod live;
#[cfg(not(feature = "installed"))]
mod mark;
pub mod night;
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
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use pixels::from_bgrx;
pub use pixels::{brush, from_rgba, unpremultiply};
pub use renderer::Renderer;
pub use surface::Surfaces;
#[cfg(not(target_os = "android"))]
pub(crate) use damage::merge::merge_closest;
pub(crate) use paint::convert::{geometry_to_path, shape_bounds};
pub(crate) use paint::render_settings;

pub(super) const TRANSFORM_PRECISION: f64 = 4096.0;

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

pub fn canvas_color(rgba: [u8; 4]) -> u32 {
    in_canvas_order(u32::from_ne_bytes(rgba))
}

pub fn over(source: u32, dest: u32) -> u32 {
    let inverse = 255 - (source >> 24);
    let rb = (dest & 0x00ff_00ff) * inverse;
    let ag = ((dest >> 8) & 0x00ff_00ff) * inverse;
    let rb = ((rb + ((rb >> 8) & 0x00ff_00ff) + 0x0080_0080) >> 8) & 0x00ff_00ff;
    let ag = (ag + ((ag >> 8) & 0x00ff_00ff) + 0x0080_0080) & 0xff00_ff00;
    source + (rb | ag)
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
