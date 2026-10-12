const MIN_LAYOUT_SPACING: f64 = 0.6;
pub const MIN_SPREAD: f64 = 0.6;
const NODE_GAP_SHARE: f64 = 0.49;
const BASE_RADIUS: f64 = 0.055;
const DEGREE_RADIUS: f64 = 0.03;
const MAX_WORLD_RADIUS: f64 = 0.7;
pub const SUB_PIXEL_RADIUS: f64 = 0.75;
pub const MAX_NODE_PIXELS: f64 = 160.0;
const HIGHLIGHT_FALLOFF: f64 = 0.8;
pub const DIMMED_STRENGTH: f64 = 0.28;

pub fn highlight_state(mark: u8) -> f64 {
    if mark < 1 { 0.0 } else { HIGHLIGHT_FALLOFF.powi(i32::from(mark) - 1) }
}

pub fn world_radius(degree: u32, size: f64, spread: f64) -> f64 {
    let ceiling = MAX_WORLD_RADIUS.min(MIN_LAYOUT_SPACING * spread * NODE_GAP_SHARE);
    ((BASE_RADIUS + DEGREE_RADIUS * f64::from(degree.max(1)).sqrt()) * size).min(ceiling)
}

pub fn radius_pixels(degree: u32, scale: f64, size: f64, spread: f64) -> f64 {
    (world_radius(degree, size, spread) * scale).clamp(SUB_PIXEL_RADIUS, MAX_NODE_PIXELS)
}
