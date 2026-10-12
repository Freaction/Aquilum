mod controls;
mod hit;
mod markdown;
mod paint;
mod view;

pub use hit::{MediaHit, MediaOverlay};
pub use markdown::{EmbedLine, format, is_embed, media_line, parse};
pub use view::MediaView;

const BUTTON: f64 = 24.0;
const BAR_PAD: f64 = 3.0;
const BAR_GAP: f64 = 4.0;
const SEPARATOR: f64 = 1.0;
const INSET: f64 = 8.0;
const PLAY: f64 = 56.0;
const HANDLE: f64 = 10.0;
const HANDLE_HIT: f64 = 8.0;
pub const MIN_WIDTH: f64 = 96.0;
const MIN_HEIGHT: f64 = 48.0;
const VIDEO_MIN_HEIGHT: f64 = 120.0;
pub const MIN_CROP: f64 = 5.0;
const FULL: [f64; 4] = [0.0, 0.0, 100.0, 100.0];
