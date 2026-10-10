use std::sync::OnceLock;

use masonry::imaging::{PaintSink, Painter};
use masonry::kurbo::{Affine, BezPath, Circle, Point, Rect};
use masonry::peniko::{Color, ColorStop, Gradient, ImageBrush};

const CITY_TILE: &[u8] = include_bytes!("../../assets/city-cover.png");
const CITY_WIDTH: f64 = 640.0;
const CITY_HEIGHT: f64 = 442.0;
const CITY_STRIP: f64 = CITY_WIDTH * 9.0;
const CITY_SECONDS: f64 = 60.0;

const DREAM_PARTICLES: u32 = 40;
const DREAM_COLOR: [u8; 3] = [153, 255, 255];

const TUNNEL_TILE: f64 = 44.0;
const TUNNEL_LINE: f64 = 2.0;
const TUNNEL_PERSPECTIVE: f64 = 84.0;
const TUNNEL_SECONDS: f64 = 2.0;
const TUNNEL_STREAK: Color = Color::from_rgb8(0xa2, 0xce, 0xf4);

pub fn animated(id: &str) -> bool {
    matches!(id, "city" | "dreams" | "tunnel")
}

pub fn enabled(setting: bool, prefers_reduced_motion: bool) -> bool {
    setting && !prefers_reduced_motion
}

pub fn paint(painter: &mut Painter<'_, impl PaintSink + ?Sized>, id: &str, bounds: Rect, seconds: f64, viewport_height: f64) {
    match id {
        "city" => city(painter, bounds, seconds),
        "dreams" => dreams(painter, bounds, seconds, viewport_height),
        "tunnel" => tunnel(painter, bounds, seconds),
        _ => {}
    }
}

fn city_tile() -> Option<&'static ImageBrush> {
    static TILE: OnceLock<Option<ImageBrush>> = OnceLock::new();
    TILE.get_or_init(|| crate::images::decode(CITY_TILE)).as_ref()
}

fn city(painter: &mut Painter<'_, impl PaintSink + ?Sized>, bounds: Rect, seconds: f64) {
    let Some(tile) = city_tile() else { return };
    let scale = CITY_WIDTH / f64::from(tile.image.width);
    let shift = -CITY_WIDTH + CITY_WIDTH * (seconds % CITY_SECONDS) / CITY_SECONDS;
    let start = bounds.width() / 2.0 - CITY_STRIP / 2.0 + shift;
    let first = ((-start) / CITY_WIDTH).floor().max(0.0);
    let y = bounds.y0 + bounds.height() - CITY_HEIGHT;
    let mut x = start + first * CITY_WIDTH;
    while x < bounds.width() && x < start + CITY_STRIP {
        painter.draw_image(tile, Affine::translate((bounds.x0 + x, y)) * Affine::scale(scale));
        x += CITY_WIDTH;
    }
}

fn dream_opacity(phase: f64) -> f64 {
    if phase < 0.15 {
        0.9 * phase / 0.15
    } else if phase < 0.85 {
        0.9 - 0.2 * (phase - 0.15) / 0.7
    } else {
        0.7 * (1.0 - phase) / 0.15
    }
}

fn dreams(painter: &mut Painter<'_, impl PaintSink + ?Sized>, bounds: Rect, seconds: f64, viewport_height: f64) {
    let (w, h) = (bounds.width(), bounds.height());
    let millis = seconds * 1000.0;
    for i in 0..DREAM_PARTICLES {
        let size = f64::from(4 + (i * 13) % 7);
        let delay = f64::from((i * 1700) % 28000);
        let duration = f64::from(24000 + (i * 2300) % 9000);
        let phase = ((millis + delay) % duration) / duration;
        let rise = viewport_height * (0.1 - 1.3 * phase);
        let grow = 0.5 + 1.3 * phase;
        let center = Point::new(bounds.x0 + w * f64::from((i * 37) % 100) / 100.0 + size / 2.0, bounds.y0 + h * 1.1 - size / 2.0 + rise);
        let radius = size / std::f64::consts::SQRT_2 * 0.65 * grow;
        let [r, g, b] = DREAM_COLOR;
        let alpha = dream_opacity(phase) as f32;
        let glow = Gradient::new_radial(center, radius as f32)
            .with_stops([ColorStop::from((0.0, Color::from_rgb8(r, g, b).with_alpha(alpha))), ColorStop::from((1.0, Color::from_rgb8(r, g, b).with_alpha(0.0)))].as_slice());
        painter.fill(Circle::new(center, radius), &glow).draw();
    }
}

struct Plane {
    top: f64,
    width: f64,
    height: f64,
    angle: f64,
}

impl Plane {
    fn depth_limit(&self) -> (f64, f64) {
        let near = (TUNNEL_PERSPECTIVE - 1.0) / self.angle.sin();
        let half = self.height / 2.0;
        if near > 0.0 { (-half, near.min(half)) } else { (near.max(-half), half) }
    }

    fn project(&self, w: f64, h: f64, x: f64, y: f64) -> Point {
        let dy = y - self.height / 2.0;
        let world_x = -w / 2.0 + x;
        let world_y = self.top + self.height / 2.0 + dy * self.angle.cos();
        let z = dy * self.angle.sin();
        let s = TUNNEL_PERSPECTIVE / (TUNNEL_PERSPECTIVE - z);
        Point::new(w / 2.0 + (world_x - w / 2.0) * s, h / 2.0 + (world_y - h / 2.0) * s)
    }

    fn quad(&self, path: &mut BezPath, w: f64, h: f64, x0: f64, y0: f64, x1: f64, y1: f64) {
        path.move_to(self.project(w, h, x0, y0));
        path.line_to(self.project(w, h, x1, y0));
        path.line_to(self.project(w, h, x1, y1));
        path.line_to(self.project(w, h, x0, y1));
        path.close_path();
    }

    fn grid(&self, w: f64, h: f64, offset_x: f64, offset_y: f64) -> BezPath {
        let (lo, hi) = self.depth_limit();
        let (lo, hi) = (lo + self.height / 2.0, hi + self.height / 2.0);
        let mut path = BezPath::new();
        let mut y = offset_y + ((lo - offset_y) / TUNNEL_TILE).floor() * TUNNEL_TILE;
        while y < hi {
            let (a, b) = (y.max(lo), (y + TUNNEL_LINE).min(hi));
            if a < b {
                self.quad(&mut path, w, h, 0.0, a, self.width, b);
            }
            y += TUNNEL_TILE;
        }
        let mut x = offset_x - TUNNEL_TILE;
        while x < self.width {
            self.quad(&mut path, w, h, x.max(0.0), lo, (x + TUNNEL_LINE).min(self.width), hi);
            x += TUNNEL_TILE;
        }
        path
    }
}

fn tunnel(painter: &mut Painter<'_, impl PaintSink + ?Sized>, bounds: Rect, seconds: f64) {
    let (w, h) = (bounds.width(), bounds.height());
    let f = (seconds % TUNNEL_SECONDS) / TUNNEL_SECONDS;
    let floor = Plane { top: 0.0, width: w * 2.0, height: h * 1.3, angle: 85f64.to_radians() };
    let ceiling = Plane { top: -h * 0.3, width: w * 2.0, height: h * 1.3, angle: -85f64.to_radians() };
    let at = Affine::translate(bounds.origin().to_vec2());
    painter.fill(&floor.grid(w, h, TUNNEL_TILE * f, -TUNNEL_TILE + TUNNEL_TILE * f), TUNNEL_STREAK).transform(at).draw();
    painter.fill(&ceiling.grid(w, h, TUNNEL_TILE * f, -TUNNEL_TILE * f), TUNNEL_STREAK).transform(at).draw();
}

#[cfg(test)]
mod tests {
    #[test]
    fn motion_requires_the_setting_and_system_permission() {
        assert!(super::enabled(true, false));
        assert!(!super::enabled(false, false));
        assert!(!super::enabled(true, true));
    }
}
